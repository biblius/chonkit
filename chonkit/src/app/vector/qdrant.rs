use crate::app::vector::DOCUMENT_ID_PROPERTY;
use crate::core::model::collection::VectorCollection;
use crate::core::vector::VectorDb;
use crate::error::{ChonkitErr, ChonkitError};
use crate::{err, map_err, DEFAULT_COLLECTION_NAME};
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::with_payload_selector::SelectorOptions;
use qdrant_client::qdrant::{
    value, Condition, CreateCollection, DeletePointsBuilder, Distance, Filter,
    GetCollectionInfoResponse, PointStruct, SearchParams, SearchPoints, UpsertPointsBuilder,
    VectorParams, VectorsConfig, WithPayloadSelector,
};
use qdrant_client::{Payload, Qdrant, QdrantError};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::CONTENT_PROPERTY;

/// Alias for an arced Qdrant instance.
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
                collections.push(VectorCollection::new(name, size));
            }
        }

        Ok(collections)
    }

    async fn create_vector_collection(&self, name: &str, size: usize) -> Result<(), ChonkitError> {
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

        debug_assert!(res.result);

        Ok(())
    }

    async fn get_collection(&self, name: &str) -> Result<VectorCollection, ChonkitError> {
        let info = map_err!(self.collection_info(name).await);
        let Some(size) = get_collection_size(&info) else {
            #[cfg(debug_assertions)]
            {
                debug!("{info:?}")
            }
            return err!(
                DoesNotExist,
                "Size information for vector collection '{name}'"
            );
        };
        Ok(VectorCollection::new(name.to_string(), size))
    }

    async fn delete_vector_collection(&self, name: &str) -> Result<(), ChonkitError> {
        map_err!(self.delete_collection(name).await);
        Ok(())
    }

    async fn create_default_collection(&self, size: usize) {
        let result = self
            .create_vector_collection(DEFAULT_COLLECTION_NAME, size)
            .await;

        match result {
            Ok(_) => {}
            Err(ChonkitError {
                error: ChonkitErr::Qdrant(QdrantError::ResponseError { status }),
                ..
            }) if matches!(status.code(), tonic::Code::AlreadyExists) => {}
            Err(e) => panic!("{e}"),
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

#[cfg(test)]
#[suitest::suite(qdrant_tests)]
mod qdrant_tests {
    use crate::{
        app::{
            test::{init_qdrant, AsyncContainer},
            vector::qdrant::QdrantDb,
        },
        core::vector::VectorDb,
        DEFAULT_COLLECTION_NAME,
    };
    use suitest::before_all;

    #[before_all]
    async fn setup() -> (QdrantDb, AsyncContainer) {
        let (weaver, img) = init_qdrant().await;
        weaver.create_default_collection(420).await;
        (weaver, img)
    }

    #[test]
    async fn creates_default_collection(qdrant: QdrantDb) {
        let default = qdrant
            .get_collection(DEFAULT_COLLECTION_NAME)
            .await
            .unwrap();

        assert_eq!(DEFAULT_COLLECTION_NAME, default.name);
        assert_eq!(420, default.size);
    }

    #[test]
    async fn creates_collection(qdrant: QdrantDb) {
        let name = "my_collection_0";

        qdrant.create_vector_collection(name, 420).await.unwrap();

        let collection = qdrant.get_collection(name).await.unwrap();

        assert_eq!(name, collection.name,);
        assert_eq!(420, collection.size);
    }
}
