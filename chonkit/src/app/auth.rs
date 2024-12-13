use crate::error::ChonkitError;
use crate::{err, map_err};
use axum::{extract::Request, middleware::Next, response::Response};
use axum::{http::StatusCode, response::IntoResponse};
use axum_macros::debug_middleware;
use base64::Engine;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use vaultrs::api::transit::requests::VerifySignedDataRequestBuilder;
use vaultrs::client::{Client, VaultClient, VaultClientSettingsBuilder};

#[derive(Clone)]
pub struct VaultAuthenticator {
    /// The name of the key used to verify tokens.
    key: String,

    /// Vault client. Has to be in RwLock because of refreshing the token.
    /// Can probably be mitigated with channels, but this is good enough currently.
    client: Arc<RwLock<VaultClient>>,

    _refresh_task: Arc<tokio::task::JoinHandle<()>>,
}

impl VaultAuthenticator {
    /// Construct a new instance of `VaultAuthenticator` and perform the AppRole login.
    pub async fn new(endpoint: String, role_id: String, secret_id: String, key: String) -> Self {
        let mut client = VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address(endpoint)
                .build()
                .expect("error building vault client"),
        )
        .expect("error building vault client");

        let response = vaultrs::auth::approle::login(&client, "approle", &role_id, &secret_id)
            .await
            .expect("unable to login to vault");

        client.set_token(&response.client_token);

        let client = Arc::new(RwLock::new(client));

        let token_duration = response.lease_duration;

        let job_client = client.clone();

        let refresh_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(Self::refresh_rate(token_duration))).await;

                tracing::info!("Renewing Vault token");

                let mut client = job_client.write().await;

                let response =
                    match vaultrs::token::renew_self(&*client, Some(&token_duration.to_string()))
                        .await
                    {
                        Ok(res) => res,
                        Err(e) => {
                            tracing::error!("Unable to renew token: {e}");
                            continue;
                        }
                    };

                client.set_token(&response.client_token);

                tracing::info!(
                    "Successfully renewed Vault token. Token valid for {}s, refreshing every {} seconds",
                    response.lease_duration,
                    Self::refresh_rate(response.lease_duration)
                );
            }
        });

        Self {
            key,
            client,
            _refresh_task: Arc::new(refresh_task),
        }
    }

    pub async fn verify_token(&self, token: &str) -> Result<(), ChonkitError> {
        let Some((payload, signature)) = token.rsplit_once('.') else {
            tracing::error!("malformed access token: {token}");
            return err!(Unauthorized);
        };

        let Some((_, body)) = payload.split_once('.') else {
            tracing::error!("malformed access token payload: {payload}");
            return err!(Unauthorized);
        };

        let token = map_err!(base64::prelude::BASE64_STANDARD.decode(body));
        let token = map_err!(String::from_utf8(token));
        let token: ChonkitJwt = map_err!(serde_json::from_str(&token));

        let version = token.version;

        // Vault needs the payload to be B64 encoded.
        let input = base64::prelude::BASE64_STANDARD.encode(payload);

        let client = self.client.read().await;

        let mut request = VerifySignedDataRequestBuilder::default();

        let result = match vaultrs::transit::data::verify(
            &*client,
            "llmao-transit-engine",
            &self.key,
            &input,
            Some(request.signature(format!("vault:v{version}:{signature}"))),
        )
        .await
        {
            Ok(res) => res,
            Err(e) => {
                tracing::error!("Vault error: {e}");
                return err!(Unauthorized);
            }
        };

        if !result.valid {
            tracing::warn!("Token signature not valid, rejecting request");
            return err!(Unauthorized);
        }

        if token.exp <= chrono::Utc::now().timestamp() {
            tracing::warn!("Token expired, rejecting request");
            return err!(Unauthorized);
        }

        if token.aud != "chonkit" {
            tracing::warn!("Token audience not valid, rejecting request");
            return err!(Unauthorized);
        }

        Ok(())
    }

    #[inline]
    fn refresh_rate(token_duration: u64) -> u64 {
        if token_duration.saturating_sub(300) == 0 {
            token_duration
        } else {
            token_duration.saturating_sub(300)
        }
    }
}

impl std::fmt::Debug for VaultAuthenticator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VaultAuthenticator").finish()
    }
}

#[derive(Debug, Deserialize)]
struct ChonkitJwt {
    aud: String,
    exp: i64,
    version: usize,
}

#[debug_middleware]
pub async fn auth_check(
    vault: axum::extract::State<VaultAuthenticator>,
    cookies: axum_extra::extract::cookie::CookieJar,
    request: Request,
    next: Next,
) -> Response {
    let access_token = match cookies.get("chonkit_access_token") {
        Some(token) => token.value(),
        None => {
            tracing::info!("No access token found in cookie, checking authorization header");

            let Some(header) = request.headers().get("Authorization") else {
                tracing::error!("No authorization header found");
                return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
            };

            let header = match header.to_str() {
                Ok(header) => header,
                Err(e) => {
                    tracing::error!("Invalid header: {e}");
                    return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
                }
            };

            let Some(token) = header.strip_prefix("Bearer ") else {
                tracing::error!("Invalid authorization header");
                return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
            };

            token
        }
    };

    if let Err(e) = vault.verify_token(access_token).await {
        return e.into_response();
    };

    next.run(request).await
}
