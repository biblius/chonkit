use crate::core::vector::{
    CreateVectorCollection, VectorCollection, VectorDb, COLLECTION_EMBEDDING_MODEL_PROPERTY,
    COLLECTION_EMBEDDING_PROVIDER_PROPERTY, COLLECTION_ID_PROPERTY, COLLECTION_NAME_PROPERTY,
    COLLECTION_SIZE_PROPERTY, CONTENT_PROPERTY, DOCUMENT_ID_PROPERTY,
};
use crate::error::{ChonkitErr, ChonkitError};
use crate::{err, map_err};
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::with_payload_selector::SelectorOptions;
use qdrant_client::qdrant::{
    value, Condition, CreateCollection, DeletePointsBuilder, Distance, Filter,
    GetCollectionInfoResponse, PointStruct, SearchParams, SearchPoints, UpsertPointsBuilder,
    VectorParams, VectorsConfig, WithPayloadSelector,
};
use qdrant_client::{Payload, Qdrant, QdrantError};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Alias for an arced Qdrant instance.
///
/// Since Qdrant does not support collection properties, we have to create a vector
/// representing them (which is how this implementation can actually get info about a collection).
/// Every collection has a "null" vector, i.e. a vector that is the same size as the embeddings,
/// but with all values set to 0.0. The null vector is used to get the properties of the collection.
///
/// It is jank, but there is no other way to do it.
pub type QdrantDb = Arc<Qdrant>;

pub fn init(url: &str) -> QdrantDb {
    info!("Connecting to qdrant at {url}");
    Arc::new(
        Qdrant::from_url(url)
            .build()
            .expect("error initialising qdrant"),
    )
}

#[async_trait::async_trait]
impl VectorDb for Qdrant {
    fn id(&self) -> &'static str {
        "qdrant"
    }

    async fn list_vector_collections(&self) -> Result<Vec<VectorCollection>, ChonkitError> {
        let collection_names = map_err!(self.list_collections().await)
            .collections
            .into_iter()
            .map(|col| col.name)
            .collect::<Vec<_>>();

        let mut collections = vec![];

        for name in collection_names {
            let info = map_err!(self.collection_info(&name).await);
            let size = get_collection_size(&info);
            if let Some(size) = size {
                let info = get_id_vector(self, &name, size).await?;
                collections.push(info);
            }
        }

        Ok(collections)
    }

    async fn create_vector_collection(
        &self,
        data: CreateVectorCollection<'_>,
    ) -> Result<(), ChonkitError> {
        let CreateVectorCollection { name, size, .. } = data;

        let config = VectorsConfig {
            config: Some(Config::Params(VectorParams {
                size: size as u64,
                distance: Distance::Cosine.into(),
                ..Default::default()
            })),
        };

        let res = map_err!(
            self.create_collection(CreateCollection {
                collection_name: name.to_string(),
                vectors_config: Some(config),
                ..Default::default()
            })
            .await
        );

        map_err!(create_id_vector(self, data).await);

        debug_assert!(res.result);

        Ok(())
    }

    async fn get_collection(&self, name: &str) -> Result<VectorCollection, ChonkitError> {
        let info = map_err!(self.collection_info(name).await);
        let size = get_collection_size(&info);
        let Some(size) = size else {
            #[cfg(debug_assertions)]
            {
                debug!("{info:?}")
            }
            return err!(
                DoesNotExist,
                "Size information for vector collection '{name}'"
            );
        };

        let info = get_id_vector(self, name, size).await?;

        Ok(info)
    }

    async fn delete_vector_collection(&self, name: &str) -> Result<(), ChonkitError> {
        map_err!(self.delete_collection(name).await);
        Ok(())
    }

    async fn create_default_collection(
        &self,
        data: CreateVectorCollection<'_>,
    ) -> Result<(), ChonkitError> {
        let result = self.create_vector_collection(data).await;

        match result {
            Ok(_) => Ok(()),
            Err(ChonkitError {
                error: ChonkitErr::Qdrant(QdrantError::ResponseError { status }),
                ..
            }) if matches!(status.code(), tonic::Code::AlreadyExists) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn query(
        &self,
        search: Vec<f64>,
        collection: &str,
        limit: u32,
    ) -> Result<Vec<String>, ChonkitError> {
        let search_points = SearchPoints {
            collection_name: collection.to_string(),
            vector: search.into_iter().map(|x| x as f32).collect(),
            filter: None,
            limit: limit as u64,
            with_payload: Some(WithPayloadSelector {
                selector_options: Some(SelectorOptions::Enable(true)),
            }),
            params: Some(SearchParams::default()),
            ..Default::default()
        };

        let search_result = map_err!(self.search_points(search_points).await);

        let results = search_result
            .result
            .into_iter()
            .filter_map(|mut point| point.payload.remove(CONTENT_PROPERTY)?.kind)
            .filter_map(|value| match value {
                value::Kind::StringValue(s) => Some(s),
                v => {
                    warn!("Found unsupported value kind: {v:?}");
                    None
                }
            })
            .collect();

        Ok(results)
    }

    async fn insert_embeddings(
        &self,
        document_id: Uuid,
        collection: &str,
        content: &[&str],
        vectors: Vec<Vec<f64>>,
    ) -> Result<(), ChonkitError> {
        debug!("Inserting vectors to {collection}");

        debug_assert_eq!(
            content.len(),
            vectors.len(),
            "Content length is different from embeddings!"
        );

        let points: Vec<PointStruct> = vectors
            .into_iter()
            .zip(content.iter())
            .map(|(embedding, content)| {
                let mut payload = Payload::new();
                payload.insert(CONTENT_PROPERTY, content.to_string());
                payload.insert(DOCUMENT_ID_PROPERTY, document_id.to_string());
                PointStruct::new(
                    uuid::Uuid::new_v4().to_string(),
                    embedding
                        .into_iter()
                        .map(|x| x as f32)
                        .collect::<Vec<f32>>(),
                    payload,
                )
            })
            .collect();

        map_err!(
            self.upsert_points(UpsertPointsBuilder::new(collection, points).wait(true))
                .await
        );

        Ok(())
    }

    async fn delete_embeddings(
        &self,
        collection: &str,
        document_id: uuid::Uuid,
    ) -> Result<(), ChonkitError> {
        map_err!(
            self.delete_points(
                DeletePointsBuilder::new(collection)
                    .points(Filter::must([Condition::matches(
                        DOCUMENT_ID_PROPERTY,
                        document_id.to_string(),
                    )]))
                    .wait(true),
            )
            .await
        );

        Ok(())
    }

    async fn count_vectors(
        &self,
        collection: &str,
        document_id: Uuid,
    ) -> Result<usize, ChonkitError> {
        use qdrant_client::qdrant::{Condition, Filter, ScrollPointsBuilder};

        let scroll = map_err!(
            self.scroll(
                ScrollPointsBuilder::new(collection)
                    .filter(Filter::must([Condition::matches(
                        DOCUMENT_ID_PROPERTY,
                        document_id.to_string(),
                    )]))
                    .with_payload(false)
                    .with_vectors(false),
            )
            .await
        );

        Ok(scroll.result.len())
    }
}

async fn create_id_vector(
    qdrant: &Qdrant,
    collection: CreateVectorCollection<'_>,
) -> Result<(), QdrantError> {
    let mut payload = Payload::new();
    payload.insert("collection_info", json! { collection });

    let point = PointStruct::new(
        uuid::Uuid::nil().to_string(),
        vec![0.0; collection.size],
        payload,
    );

    qdrant
        .upsert_points(UpsertPointsBuilder::new(collection.name, vec![point]).wait(true))
        .await?;

    Ok(())
}

async fn get_id_vector(
    qdrant: &Qdrant,
    name: &str,
    size: usize,
) -> Result<VectorCollection, ChonkitError> {
    let search_points = SearchPoints {
        collection_name: name.to_string(),
        vector: vec![0.0; size],
        filter: Some(qdrant_client::qdrant::Filter::must(vec![
            Condition::has_id(vec![Uuid::nil().to_string()]),
        ])),
        limit: 1,
        with_payload: Some(WithPayloadSelector {
            selector_options: Some(SelectorOptions::Enable(true)),
        }),
        params: Some(SearchParams::default()),
        ..Default::default()
    };

    let mut search_result = map_err!(qdrant.search_points(search_points).await);

    let results = &mut search_result.result[0];

    let Some(info_string) = results.payload.remove("collection_info") else {
        return err!(DoesNotExist, "Collection info vector for '{name}'");
    };

    let Some(kind) = info_string.kind else {
        return err!(DoesNotExist, "Collection info vector for '{name}'");
    };

    let value = match kind {
        value::Kind::StructValue(s) => s,
        v => {
            warn!("Found unsupported value kind: {v:?}");
            return err!(DoesNotExist, "Collection info vector for '{name}'");
        }
    };

    let config = match VectorCollection::try_from_map(value.fields) {
        Some(c) => c,
        None => {
            tracing::error!("Invalid collection info vector for '{name}'");
            return err!(DoesNotExist, "Collection info vector for '{name}'");
        }
    };

    Ok(config)
}

fn get_collection_size(info: &GetCollectionInfoResponse) -> Option<usize> {
    let config = info
        .result
        .as_ref()?
        .config
        .as_ref()?
        .params
        .as_ref()?
        .vectors_config
        .as_ref()?
        .config
        .as_ref()?;
    match config {
        Config::Params(VectorParams { size, .. }) => Some(*size as usize),
        Config::ParamsMap(pm) => {
            warn!("Found unexpected params map! {pm:?}");
            None
        }
    }
}

impl VectorCollection {
    fn try_from_map(map: HashMap<String, qdrant_client::qdrant::Value>) -> Option<Self> {
        macro_rules! get_for_type {
            ($map:ident, $field:ident, $const:ident, $kind:ident) => {{
                let Some($field) = $map.get($const) else {
                    tracing::error!("Missing '{}' property", $const);
                    return None;
                };

                let Some(value::Kind::$kind(ref $field)) = $field.kind else {
                    tracing::error!("Invalid '{}' property", $const);
                    return None;
                };

                $field
            }};
        }

        let name = get_for_type!(map, name, COLLECTION_NAME_PROPERTY, StringValue);
        let size = get_for_type!(map, size, COLLECTION_SIZE_PROPERTY, IntegerValue);
        let embedding_model = get_for_type!(
            map,
            embedding_model,
            COLLECTION_EMBEDDING_MODEL_PROPERTY,
            StringValue
        );
        let embedding_provider = get_for_type!(
            map,
            embedding_provider,
            COLLECTION_EMBEDDING_PROVIDER_PROPERTY,
            StringValue
        );

        let id = get_for_type!(map, id, COLLECTION_ID_PROPERTY, StringValue);
        let id = match id.parse() {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Invalid UUID for 'collection_id' property: {e}");
                return None;
            }
        };

        Some(VectorCollection::new(
            id,
            name.to_string(),
            *size as usize,
            embedding_provider.to_string(),
            embedding_model.to_string(),
        ))
    }
}

#[cfg(test)]
#[suitest::suite(qdrant_tests)]
mod qdrant_tests {
    use crate::{
        app::{
            test::{init_qdrant, AsyncContainer},
            vector::qdrant::QdrantDb,
        },
        config::{
            DEFAULT_COLLECTION_EMBEDDING_MODEL, DEFAULT_COLLECTION_EMBEDDING_PROVIDER,
            DEFAULT_COLLECTION_NAME, DEFAULT_COLLECTION_SIZE,
        },
        core::vector::{CreateVectorCollection, VectorDb},
    };
    use suitest::before_all;
    use uuid::Uuid;

    #[before_all]
    async fn setup() -> (QdrantDb, AsyncContainer) {
        let (qdrant, img) = init_qdrant().await;

        let data = CreateVectorCollection::default();

        qdrant.create_default_collection(data).await.unwrap();
        (qdrant, img)
    }

    #[test]
    async fn creates_default_collection(qdrant: QdrantDb) {
        let default = qdrant
            .get_collection(DEFAULT_COLLECTION_NAME)
            .await
            .unwrap();

        assert_eq!(DEFAULT_COLLECTION_NAME, default.name);
        assert_eq!(DEFAULT_COLLECTION_SIZE, default.size);
        assert_eq!(
            DEFAULT_COLLECTION_EMBEDDING_PROVIDER,
            default.embedding_provider
        );
        assert_eq!(DEFAULT_COLLECTION_EMBEDDING_MODEL, default.embedding_model);
    }

    #[test]
    async fn creates_collection(qdrant: QdrantDb) {
        let name = "My_collection_0";
        let id = Uuid::new_v4();

        let data = CreateVectorCollection::new(id, name, 420, "openai", "text-embedding-ada-002");

        qdrant.create_vector_collection(data).await.unwrap();

        let collection = qdrant.get_collection(name).await.unwrap();

        assert_eq!(id, collection.id);
        assert_eq!(name, collection.name);
        assert_eq!(420, collection.size);
        assert_eq!("openai", collection.embedding_provider);
        assert_eq!("text-embedding-ada-002", collection.embedding_model);
    }
}
