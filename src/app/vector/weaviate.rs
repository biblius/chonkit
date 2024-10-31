use crate::{
    app::vector::DOCUMENT_ID_PROPERTY,
    core::{model::collection::VectorCollection, vector::VectorDb},
    error::ChonkitError,
    DEFAULT_COLLECTION_NAME,
};
use dto::{QueryResult, WeaviateError};
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
use weaviate_community::{
    collections::{
        batch::{BatchDeleteRequest, MatchConfig},
        error::SchemaError,
        objects::{ConsistencyLevel, MultiObjects, Object},
        query::GetQuery,
        schema::{Class, Properties, PropertyBuilder},
    },
    WeaviateClient,
};

use super::CONTENT_PROPERTY;

/// Alias for an arced Qdrant instance.
pub type WeaviateDb = Arc<WeaviateClient>;

pub fn init(url: &str) -> WeaviateDb {
    info!("Connecting to weaviate at {url}");
    Arc::new(WeaviateClient::new(url, None, None).expect("error initialising qdrant"))
}

#[async_trait::async_trait]
impl VectorDb for Arc<WeaviateClient> {
    fn id(&self) -> &'static str {
        "weaviate"
    }

    async fn list_vector_collections(&self) -> Result<Vec<VectorCollection>, ChonkitError> {
        let classes = self
            .schema
            .get()
            .await
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))?;

        let mut collections = vec![];

        for class in classes.classes {
            let Ok(v_collection) = class.try_into() else {
                continue;
            };
            collections.push(v_collection);
        }

        Ok(collections)
    }

    async fn create_vector_collection(&self, name: &str, size: usize) -> Result<(), ChonkitError> {
        let props = create_properties(name, size);

        let class = Class::builder(name).with_properties(props).build();

        self.schema
            .create_class(&class)
            .await
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))?;

        Ok(())
    }

    async fn get_collection(&self, name: &str) -> Result<VectorCollection, ChonkitError> {
        self.schema
            .get_class(&name)
            .await
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))?
            .try_into()
    }

    async fn delete_vector_collection(&self, name: &str) -> Result<(), ChonkitError> {
        self.schema
            .delete(&name)
            .await
            .map(|_| ())
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))
    }

    async fn create_default_collection(&self, size: usize) {
        let props = create_properties(DEFAULT_COLLECTION_NAME, size);
        let class = Class::builder(DEFAULT_COLLECTION_NAME)
            .with_description("Default vector collection")
            .with_properties(props)
            .build();

        if let Err(e) = self.schema.create_class(&class).await {
            let Some(err) = e.downcast_ref::<SchemaError>() else {
                panic!("{e}");
            };

            let Some(err) = parse_weaviate_error(&err.0) else {
                panic!("{err}")
            };

            if !err.error[0].message.contains("already exists") {
                panic!("Error: {e}; parsed: {err:?}")
            }
        };
    }

    async fn query(
        &self,
        search: Vec<f32>,
        collection: &str,
        limit: u32,
    ) -> Result<Vec<String>, ChonkitError> {
        // God help us all
        let near_vector = &format!("{{ vector: {search:?} }}");
        let query = GetQuery::builder(&collection, vec![CONTENT_PROPERTY])
            .with_near_vector(near_vector)
            .with_limit(limit)
            .build();

        let response = self
            .query
            .get(query)
            .await
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))?;

        let result: QueryResult = serde_json::from_value(response)?;

        let Some(results) = result.data.get.get(&collection) else {
            return Err(ChonkitError::Weaviate(format!(
                "Response error - cannot index into '{collection}' in {}",
                result.data.get
            )));
        };

        let results = serde_json::from_value::<Vec<serde_json::Value>>(results.clone())?
            .into_iter()
            .filter_map(|obj| obj.get(CONTENT_PROPERTY).cloned())
            .map(serde_json::from_value::<String>)
            .filter_map(Result::ok)
            .collect();

        Ok(results)
    }

    async fn insert_embeddings(
        &self,
        document_id: Uuid,
        collection: &str,
        content: &[&str],
        vectors: Vec<Vec<f32>>,
    ) -> Result<(), ChonkitError> {
        debug_assert_eq!(content.len(), vectors.len());

        let objects = content
            .iter()
            .zip(vectors.iter())
            .map(|(content, vector)| {
                let properties = json!({
                    CONTENT_PROPERTY: content,
                    DOCUMENT_ID_PROPERTY: document_id
                });
                Object::builder(&collection, properties)
                    .with_vector(vector.iter().map(|f| *f as f64).collect())
                    .with_id(uuid::Uuid::new_v4())
                    .build()
            })
            .collect();

        let objects = MultiObjects::new(objects);

        self.batch
            .objects_batch_add(objects, Some(ConsistencyLevel::ONE), None)
            .await
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))?;

        Ok(())
    }

    async fn delete_embeddings(
        &self,
        collection: &str,
        document_id: Uuid,
    ) -> Result<(), ChonkitError> {
        let delete = BatchDeleteRequest::builder(MatchConfig::new(
            &collection,
            json!({
                "path": [DOCUMENT_ID_PROPERTY],
                "operator": "Equal",
                "valueText": document_id.to_string()
            }),
        ))
        .build();

        self.batch
            .objects_batch_delete(delete, Some(ConsistencyLevel::ALL), None)
            .await
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))?;

        Ok(())
    }

    async fn count_vectors(
        &self,
        collection: &str,
        document_id: Uuid,
    ) -> Result<usize, ChonkitError> {
        let query = GetQuery::builder(&collection, vec![DOCUMENT_ID_PROPERTY])
            .with_where(&format!(
                "{{ 
                path: [\"{DOCUMENT_ID_PROPERTY}\"],
                operator: Equal,
                valueText: \"{document_id}\" 
                }}"
            ))
            .build();

        let response = self
            .query
            .get(query)
            .await
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))?;

        let result: QueryResult = serde_json::from_value(response)?;

        let Some(results) = result.data.get.get(&collection) else {
            return Err(ChonkitError::Weaviate(format!(
                "Response error - cannot index into '{collection}' in {}",
                result.data.get
            )));
        };

        let amount = serde_json::from_value::<Vec<serde_json::Value>>(results.clone())?.len();

        Ok(amount)
    }
}

/// Create properties for a collection (weaviate class).
fn create_properties(name: &str, size: usize) -> Properties {
    let size = PropertyBuilder::new("size", vec!["int"])
        .with_description(&size.to_string())
        .build();

    let name = PropertyBuilder::new("name", vec!["text"])
        .with_description(name)
        .build();

    Properties::new(vec![size, name])
}

/// Attempt to parse Weaviate GraphQL data to a [dto::WeaviateError].
fn parse_weaviate_error(s: &str) -> Option<WeaviateError> {
    let json_err = s.rsplit_once("Response: ")?.1;
    serde_json::from_str(json_err).ok()
}

impl TryFrom<Class> for VectorCollection {
    type Error = ChonkitError;

    fn try_from(class: Class) -> Result<Self, Self::Error> {
        let class_name = &class.class;

        let Some(props) = class.properties else {
            return Err(ChonkitError::Weaviate(format!(
                "Missing 'properties' field in class {class_name}",
            )));
        };

        let mut v_collection = VectorCollection::default().with_name(class_name.clone());

        for prop in props.0 {
            match prop.name.as_str() {
                "size" => {
                    let Some(size) = prop.description else {
                        return Err(ChonkitError::Weaviate(format!(
                            "Missing 'size' property in class {class_name}",
                        )));
                    };
                    let size = size.parse()?;
                    v_collection = v_collection.with_size(size);
                }
                "original_name" => {
                    let Some(name) = prop.description else {
                        return Err(ChonkitError::Weaviate(format!(
                            "Missing 'original_name' property in class {class_name}",
                        )));
                    };
                    v_collection = v_collection.with_name(name);
                }
                _ => continue,
            }
        }

        if v_collection.size == 0 {
            return Err(ChonkitError::Weaviate(format!(
                "Missing 'size' property in class {class_name}",
            )));
        }

        if v_collection.name.is_empty() {
            return Err(ChonkitError::Weaviate(format!(
                "Missing 'original_name' property in class {class_name}",
            )));
        }

        Ok(v_collection)
    }
}

mod dto {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct WeaviateError {
        pub error: Vec<ErrorMessage>,
    }

    #[derive(Debug, Deserialize)]
    pub struct ErrorMessage {
        pub message: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct QueryResult {
        pub data: GetResult,
    }

    #[derive(Debug, Deserialize)]
    pub struct GetResult {
        #[serde(rename = "Get")]
        pub get: serde_json::Value,
    }
}

#[cfg(test)]
#[suitest::suite(weaviate_tests)]
mod weaviate_tests {
    use crate::{
        app::{
            test::{init_weaviate, AsyncContainer},
            vector::weaviate::WeaviateDb,
        },
        core::vector::VectorDb,
        DEFAULT_COLLECTION_NAME,
    };
    use suitest::before_all;

    #[before_all]
    async fn setup() -> (WeaviateDb, AsyncContainer) {
        let (weaver, img) = init_weaviate().await;
        weaver.create_default_collection(420).await;
        (weaver, img)
    }

    #[test]
    async fn creates_default_collection(weaver: WeaviateDb) {
        let default = weaver
            .get_collection(DEFAULT_COLLECTION_NAME)
            .await
            .unwrap();

        assert_eq!(DEFAULT_COLLECTION_NAME, default.name);
        assert_eq!(420, default.size);
    }

    #[test]
    async fn creates_collection(weaver: WeaviateDb) {
        let name = "My_collection_0";

        weaver.create_vector_collection(name, 420).await.unwrap();

        let collection = weaver.get_collection(name).await.unwrap();

        assert_eq!(name, collection.name);
        assert_eq!(420, collection.size);
    }
}
