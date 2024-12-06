use clap::Parser;

/// The ID for the default collection created on application startup.
pub const DEFAULT_COLLECTION_ID: uuid::Uuid = uuid::Uuid::nil();
/// The name for the default collection created on application startup.
pub const DEFAULT_COLLECTION_NAME: &str = "Chonkit_Default_Collection";
/// The size for the default collection created on application startup.
pub const DEFAULT_COLLECTION_SIZE: usize = 768;
/// The embedding provider for the default collection created on application startup.
pub const DEFAULT_COLLECTION_EMBEDDING_PROVIDER: &str = "fastembed";
/// The embedding model for the default collection created on application startup.
pub const DEFAULT_COLLECTION_EMBEDDING_MODEL: &str = "Xenova/bge-base-en-v1.5";
/// The default upload path for the `fs` document storage provider.
const DEFAULT_UPLOAD_PATH: &str = "upload";
/// The default address to listen on.
const DEFAULT_ADDRESS: &str = "0.0.0.0:42069";

#[derive(Debug, Parser)]
#[command(name = "chonkit", author = "biblius", version = "0.1", about = "Chunk documents", long_about = None)]
pub struct StartArgs {
    /// Database URL.
    #[arg(short, long)]
    db_url: Option<String>,

    /// RUST_LOG string to use as the env filter.
    #[arg(short, long)]
    log: Option<String>,

    /// If using the `FsDocumentStore`, sets its path.
    #[arg(short, long)]
    upload_path: Option<String>,

    /// Address to listen on.
    #[arg(short, long)]
    address: Option<String>,

    /// CORS allowed origins.
    #[arg(long)]
    cors_allowed_origins: Option<String>,

    /// CORS allowed headers.
    #[arg(long)]
    cors_allowed_headers: Option<String>,

    /// Qdrant URL.
    #[cfg(feature = "qdrant")]
    #[arg(short, long)]
    qdrant_url: Option<String>,

    /// Weaviate URL.
    #[cfg(feature = "weaviate")]
    #[arg(short, long)]
    weaviate_url: Option<String>,

    /// If using the [OpenAiEmbeddings][crate::app::embedder::openai::OpenAiEmbeddings] module, set its endpoint.
    #[cfg(feature = "openai")]
    #[arg(short, long)]
    openai_endpoint: Option<String>,

    /// If using the fastembedder remote embedding module, set its endpoint.
    #[cfg(feature = "fe-remote")]
    #[arg(short, long)]
    fembed_url: Option<String>,

    /// Vault endpoint.
    #[cfg(feature = "auth-vault")]
    #[arg(long)]
    vault_url: Option<String>,

    /// Vault approle role ID.
    #[cfg(feature = "auth-vault")]
    #[arg(long)]
    vault_role_id: Option<String>,

    /// Vault approle secret ID.
    #[cfg(feature = "auth-vault")]
    #[arg(long)]
    vault_secret_id: Option<String>,

    /// Vault transit engine key name to use for signature verification.
    #[cfg(feature = "auth-vault")]
    #[arg(long)]
    vault_key_name: Option<String>,
}

/// Implement a getter method on [StartArgs], using the `$var` environment variable as a fallback
/// and either panic or default if neither the argument nor the environment variable is set.
macro_rules! arg {
    ($id:ident, $var:literal, panic $msg:literal) => {
        impl StartArgs {
            pub fn $id(&self) -> String {
                match &self.$id {
                    Some(val) => val.to_string(),
                    None => match std::env::var($var) {
                        Ok(val) => val,
                        Err(_) => panic!($msg),
                    },
                }
            }
        }
    };
    ($id:ident, $var:literal, default $value:expr) => {
        impl StartArgs {
            pub fn $id(&self) -> String {
                match &self.$id {
                    Some(val) => val.to_string(),
                    None => match std::env::var($var) {
                        Ok(val) => val,
                        Err(_) => $value,
                    },
                }
            }
        }
    };
}

impl StartArgs {
    pub fn allowed_origins(&self) -> Vec<String> {
        match &self.cors_allowed_origins {
            Some(origins) => origins
                .split(',')
                .filter_map(|o| (!o.is_empty()).then_some(String::from(o)))
                .collect(),
            None => match std::env::var("CORS_ALLOWED_ORIGINS") {
                Ok(origins) => origins
                    .split(',')
                    .filter_map(|o| (!o.is_empty()).then_some(String::from(o)))
                    .collect(),
                Err(_) => panic!(
                    "Allowed origins not found; Pass --allowed-origins (-c) or set CORS_ALLOWED_ORIGINS as a comma separated list"
                ),
            },
        }
    }

    pub fn allowed_headers(&self) -> Vec<String> {
        match &self.cors_allowed_headers {
            Some(headers) => headers
                .split(',')
                .filter_map(|h| (!h.is_empty()).then_some(String::from(h)))
                .collect(),
            None => match std::env::var("CORS_ALLOWED_HEADERS") {
                Ok(headers) => headers
                    .split(',')
                    .filter_map(|h| (!h.is_empty()).then_some(String::from(h)))
                    .collect(),
                Err(_) => panic!(
                    "Allowed headers not found; Pass --allowed-headers or set CORS_ALLOWED_HEADERS as a comma separated list"
                ),
            },
        }
    }

    #[cfg(feature = "openai")]
    pub fn open_ai_key(&self) -> String {
        std::env::var("OPENAI_KEY").expect("Missing OPENAI_KEY in env")
    }
}

arg!(db_url,          "DATABASE_URL",    panic   "Database url not found; Pass --db-url or set DATABASE_URL");
arg!(log,             "RUST_LOG",        default "info".to_string());
arg!(upload_path,     "UPLOAD_PATH",     default DEFAULT_UPLOAD_PATH.to_string());
arg!(address,         "ADDRESS",         default DEFAULT_ADDRESS.to_string());

#[cfg(feature = "qdrant")]
arg!(qdrant_url,      "QDRANT_URL",      panic   "Qdrant url not found; Pass --qdrant-url or set QDRANT_URL");

#[cfg(feature = "weaviate")]
arg!(weaviate_url,    "WEAVIATE_URL",    panic   "Weaviate url not found; Pass --weaviate-url or set WEAVIATE_URL");

#[cfg(feature = "fe-remote")]
arg!(fembed_url,      "FEMBED_URL",      panic   "Fembed url not found; Pass --fembed-url or set FEMBED_URL");

#[cfg(feature = "auth-vault")]
arg!(vault_url,  "VAULT_URL",   panic "Vault url not found; Pass --vault-url or set VAULT_URL");
#[cfg(feature = "auth-vault")]
arg!(vault_role_id,   "VAULT_ROLE_ID",     panic "Vault role id not found; Pass --vault-role-id or set VAULT_ROLE_ID");
#[cfg(feature = "auth-vault")]
arg!(vault_secret_id, "VAULT_SECRET_ID", panic "Vault secret id not found; Pass --vault-secret-id or set VAULT_SECRET_ID");
#[cfg(feature = "auth-vault")]
arg!(vault_key_name, "VAULT_KEY_NAME", panic "Vault key name not found; Pass --vault-key-name or set VAULT_KEY_NAME");
