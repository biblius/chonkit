use crate::core::vector::{
    CreateVectorCollection, VectorCollection, VectorDb, COLLECTION_EMBEDDING_MODEL_PROPERTY,
    COLLECTION_EMBEDDING_PROVIDER_PROPERTY, COLLECTION_ID_PROPERTY, COLLECTION_NAME_PROPERTY,
    COLLECTION_SIZE_PROPERTY, CONTENT_PROPERTY, DOCUMENT_ID_PROPERTY,
};
use crate::{err, error::ChonkitError, map_err};
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

/// Alias for an arced Weaviate instance.
pub type WeaviateDb = Arc<WeaviateClient>;

pub fn init(url: &str) -> WeaviateDb {
    info!("Connecting to weaviate at {url}");
    Arc::new(WeaviateClient::new(url, None, None).expect("error initialising weaviate"))
}

#[async_trait::async_trait]
impl VectorDb for WeaviateClient {
    fn id(&self) -> &'static str {
        "weaviate"
    }

    async fn list_vector_collections(&self) -> Result<Vec<VectorCollection>, ChonkitError> {
        let classes = match self.schema.get().await {
            Ok(classes) => classes,
            Err(e) => return err!(Weaviate, "{}", e),
        };

        let mut collections = vec![];

        for class in classes.classes {
            let Ok(v_collection) = class.try_into() else {
                continue;
            };
            collections.push(v_collection);
        }

        Ok(collections)
    }

    async fn create_vector_collection(
        &self,
        data: CreateVectorCollection<'_>,
    ) -> Result<(), ChonkitError> {
        let class = Class::builder(data.name);
        let props = create_collection_properties(data);
        let class = class.with_properties(props).build();

        if let Err(e) = self.schema.create_class(&class).await {
            return err!(Weaviate, "{}", e);
        }

        Ok(())
    }

    async fn get_collection(&self, name: &str) -> Result<VectorCollection, ChonkitError> {
        match self.schema.get_class(&name).await {
            Ok(class) => class.try_into(),
            Err(e) => err!(Weaviate, "{}", e),
        }
    }

    async fn delete_vector_collection(&self, name: &str) -> Result<(), ChonkitError> {
        if let Err(e) = self.schema.delete(&name).await {
            return err!(Weaviate, "{}", e);
        }
        Ok(())
    }

    async fn create_default_collection(
        &self,
        data: CreateVectorCollection<'_>,
    ) -> Result<(), ChonkitError> {
        let class = Class::builder(data.name);
        let props = create_collection_properties(data);
        let class = class
            .with_description("Default vector collection")
            .with_properties(props)
            .build();

        if let Err(e) = self.schema.create_class(&class).await {
            let Some(err) = e.downcast_ref::<SchemaError>() else {
                return err!(Weaviate, "{e}");
            };

            let Some(err) = parse_weaviate_error(&err.0) else {
                return err!(Weaviate, "{e}");
            };

            if !err.error[0].message.contains("already exists") {
                return err!(Weaviate, "{e}");
            }
        };

        Ok(())
    }

    async fn query(
        &self,
        search: Vec<f64>,
        collection: &str,
        limit: u32,
    ) -> Result<Vec<String>, ChonkitError> {
        // God help us all
        let near_vector = &format!("{{ vector: {search:?} }}");
        let query = GetQuery::builder(&collection, vec![CONTENT_PROPERTY])
            .with_near_vector(near_vector)
            .with_limit(limit)
            .build();

        let response = match self.query.get(query).await {
            Ok(res) => res,
            Err(e) => return err!(Weaviate, "{}", e),
        };

        let result: QueryResult = map_err!(serde_json::from_value(response));

        let Some(results) = result.data.get.get(&collection) else {
            return err!(
                Weaviate,
                "Response error - cannot index into '{collection}' in {}",
                result.data.get
            );
        };

        let results = map_err!(serde_json::from_value::<Vec<serde_json::Value>>(
            results.clone()
        ))
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
        vectors: Vec<Vec<f64>>,
    ) -> Result<(), ChonkitError> {
        debug_assert_eq!(content.len(), vectors.len());

        let objects = content
            .iter()
            .zip(vectors.into_iter())
            .map(|(content, vector)| {
                let properties = json!({
                    CONTENT_PROPERTY: content,
                    DOCUMENT_ID_PROPERTY: document_id
                });
                Object::builder(&collection, properties)
                    .with_vector(vector)
                    .with_id(uuid::Uuid::new_v4())
                    .build()
            })
            .collect();

        let objects = MultiObjects::new(objects);

        if let Err(e) = self
            .batch
            .objects_batch_add(objects, Some(ConsistencyLevel::ONE), None)
            .await
        {
            return err!(Weaviate, "{}", e);
        }

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

        if let Err(e) = self
            .batch
            .objects_batch_delete(delete, Some(ConsistencyLevel::ALL), None)
            .await
        {
            return err!(Weaviate, "{}", e);
        }

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

        let response = match self.query.get(query).await {
            Ok(res) => res,
            Err(e) => return err!(Weaviate, "{}", e),
        };

        let result: QueryResult = map_err!(serde_json::from_value(response));

        let Some(results) = result.data.get.get(&collection) else {
            return err!(
                Weaviate,
                "Response error - cannot index into '{collection}' in {}",
                result.data.get
            );
        };

        let amount = map_err!(serde_json::from_value::<Vec<serde_json::Value>>(
            results.clone()
        ))
        .len();

        Ok(amount)
    }
}

/// Create properties for a collection (weaviate class).
fn create_collection_properties(data: CreateVectorCollection<'_>) -> Properties {
    let id = PropertyBuilder::new(COLLECTION_ID_PROPERTY, vec!["text"])
        .with_description(&data.collection_id.to_string())
        .build();

    let size = PropertyBuilder::new(COLLECTION_SIZE_PROPERTY, vec!["int"])
        .with_description(&data.size.to_string())
        .build();

    let name = PropertyBuilder::new(COLLECTION_NAME_PROPERTY, vec!["text"])
        .with_description(data.name)
        .build();

    let embedding_provider =
        PropertyBuilder::new(COLLECTION_EMBEDDING_PROVIDER_PROPERTY, vec!["text"])
            .with_description(data.embedding_provider)
            .build();

    let embedding_model = PropertyBuilder::new(COLLECTION_EMBEDDING_MODEL_PROPERTY, vec!["text"])
        .with_description(data.embedding_model)
        .build();

    Properties::new(vec![id, size, name, embedding_provider, embedding_model])
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
            return err!(Weaviate, "Missing 'properties' field in class {class_name}");
        };

        let mut v_collection = VectorCollection::default().with_name(class_name.clone());

        for prop in props.0 {
            match prop.name.as_str() {
                COLLECTION_SIZE_PROPERTY => {
                    let Some(size) = prop.description else {
                        return err!(Weaviate, "Missing 'size' property in class {class_name}",);
                    };
                    let size = map_err!(size.parse::<usize>());
                    v_collection = v_collection.with_size(size);
                }
                COLLECTION_NAME_PROPERTY => {
                    let Some(name) = prop.description else {
                        return err!(Weaviate, "Missing 'name' property in class {class_name}",);
                    };
                    v_collection = v_collection.with_name(name);
                }
                COLLECTION_EMBEDDING_PROVIDER_PROPERTY => {
                    let Some(embedding_provider) = prop.description else {
                        return err!(
                            Weaviate,
                            "Missing 'embedding_provider' property in class {class_name}",
                        );
                    };
                    v_collection = v_collection.with_embedding_provider(embedding_provider);
                }
                COLLECTION_EMBEDDING_MODEL_PROPERTY => {
                    let Some(embedding_model) = prop.description else {
                        return err!(
                            Weaviate,
                            "Missing 'embedding_model' property in class {class_name}",
                        );
                    };
                    v_collection = v_collection.with_embedding_model(embedding_model);
                }
                COLLECTION_ID_PROPERTY => {
                    let Some(id) = prop.description else {
                        return err!(
                            Weaviate,
                            "Missing 'collection_id' property in class {class_name}",
                        );
                    };
                    let id = map_err!(id.parse::<Uuid>());
                    v_collection = v_collection.with_id(id);
                }
                _ => continue,
            }
        }

        if v_collection.size == 0 {
            return err!(Weaviate, "Missing 'size' property in class {class_name}",);
        }

        if v_collection.name.is_empty() {
            return err!(
                Weaviate,
                "Missing 'original_name' property in class {class_name}",
            );
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
        config::{
            DEFAULT_COLLECTION_EMBEDDING_MODEL, DEFAULT_COLLECTION_EMBEDDING_PROVIDER,
            DEFAULT_COLLECTION_ID, DEFAULT_COLLECTION_NAME, DEFAULT_COLLECTION_SIZE,
        },
        core::vector::{CreateVectorCollection, VectorDb},
    };
    use suitest::before_all;
    use uuid::Uuid;

    #[before_all]
    async fn setup() -> (WeaviateDb, AsyncContainer) {
        let (weaver, img) = init_weaviate().await;
        weaver
            .create_default_collection(CreateVectorCollection::default())
            .await
            .unwrap();
        (weaver, img)
    }

    #[test]
    async fn creates_default_collection(weaver: WeaviateDb) {
        let default = weaver
            .get_collection(DEFAULT_COLLECTION_NAME)
            .await
            .unwrap();

        assert_eq!(DEFAULT_COLLECTION_ID, default.id);
        assert_eq!(DEFAULT_COLLECTION_NAME, default.name);
        assert_eq!(DEFAULT_COLLECTION_SIZE, default.size);
        assert_eq!(
            DEFAULT_COLLECTION_EMBEDDING_PROVIDER,
            default.embedding_provider
        );
        assert_eq!(DEFAULT_COLLECTION_EMBEDDING_MODEL, default.embedding_model);
    }

    #[test]
    async fn creates_collection(weaver: WeaviateDb) {
        let name = "My_collection_0";
        let id = Uuid::new_v4();

        let data = CreateVectorCollection::new(id, name, 420, "openai", "text-embedding-ada-002");

        weaver.create_vector_collection(data).await.unwrap();

        let collection = weaver.get_collection(name).await.unwrap();

        assert_eq!(id, collection.id);
        assert_eq!(name, collection.name);
        assert_eq!(420, collection.size);
        assert_eq!("openai", collection.embedding_provider);
        assert_eq!("text-embedding-ada-002", collection.embedding_model);
    }
}
