use crate::{
    core::{model::collection::VectorCollection, vector::VectorDb},
    error::ChonkitError,
    DEFAULT_COLLECTION_NAME, DEFAULT_COLLECTION_SIZE,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use weaviate_community::{
    collections::{
        error::SchemaError,
        objects::{ConsistencyLevel, MultiObjects, Object},
        query::GetQuery,
        schema::{Class, Properties, PropertyBuilder},
    },
    WeaviateClient,
};

/// Alias for an arced Qdrant instance.
pub type WeaviateDb = Arc<WeaviateClient>;

pub fn init(url: &str) -> WeaviateDb {
    info!("Connecting to weaviate at {url}");
    Arc::new(WeaviateClient::new(url, None, None).expect("error initialising qdrant"))
}

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

    async fn create_vector_collection(&self, name: &str, size: u64) -> Result<(), ChonkitError> {
        let size = PropertyBuilder::new("size", vec!["int"])
            .with_description(&size.to_string())
            .build();

        let content = PropertyBuilder::new("content", vec!["text"])
            .with_description("Chunk content")
            .build();

        let props = Properties::new(vec![content, size]);

        let class = Class::builder(name).with_properties(props).build();

        self.schema
            .create_class(&class)
            .await
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))?;

        Ok(())
    }

    async fn get_collection(&self, name: &str) -> Result<VectorCollection, ChonkitError> {
        self.schema
            .get_class(name)
            .await
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))?
            .try_into()
    }

    async fn delete_vector_collection(&self, name: &str) -> Result<(), ChonkitError> {
        self.schema
            .delete(name)
            .await
            .map(|_| ())
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))
    }

    async fn create_default_collection(&self) {
        let size = PropertyBuilder::new("size", vec!["int"])
            .with_description(&DEFAULT_COLLECTION_SIZE.to_string())
            .build();

        let content = PropertyBuilder::new("content", vec!["text"])
            .with_description("Chunk content")
            .build();

        let props = Properties::new(vec![content, size]);

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

            // Capitalize, because Weaviate capitalizes class names
            let collection = weaviate_class_name(DEFAULT_COLLECTION_NAME);
            let expected = format!(r#"class name "{collection}" already exists"#);

            if err.error[0].message != expected {
                panic!("{e}")
            }
        };
    }

    async fn query(
        &self,
        search: Vec<f32>,
        collection: &str,
        limit: u32,
    ) -> Result<Vec<String>, ChonkitError> {
        let query = GetQuery::builder(collection, vec!["content"])
            .with_near_vector(&json!({ "vector": search }).to_string())
            .with_limit(limit)
            .build();

        let res = self
            .query
            .get(query)
            .await
            .map_err(|e| ChonkitError::Weaviate(e.to_string()))?;

        dbg!(res);
        todo!()
    }

    async fn store(
        &self,
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
                    "content": content
                });
                Object::builder(collection, properties)
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
}

fn parse_weaviate_error(s: &str) -> Option<WeaviateError> {
    let json_err = s.rsplit_once("Response: ")?.1;
    serde_json::from_str(json_err).ok()
}

fn weaviate_class_name(s: &str) -> String {
    format!("{}{}", s[0..1].to_uppercase(), &s[1..])
}

#[derive(Debug, Deserialize)]
struct WeaviateError {
    error: Vec<ErrorMessage>,
}

#[derive(Debug, Deserialize)]
struct ErrorMessage {
    message: String,
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
                _ => continue,
            }
        }

        if v_collection.size == 0 {
            return Err(ChonkitError::Weaviate(format!(
                "Missing 'size' property in class {class_name}",
            )));
        }

        Ok(v_collection)
    }
}

#[cfg(test)]
#[suitest::suite(weaviate_tests)]
mod weaviate_tests {
    use crate::{
        app::{
            test::{init_weaviate, AsyncContainer},
            vector::weaviate::{weaviate_class_name, WeaviateDb},
        },
        core::vector::VectorDb,
        DEFAULT_COLLECTION_NAME, DEFAULT_COLLECTION_SIZE,
    };
    use suitest::before_all;

    #[before_all]
    async fn setup() -> (WeaviateDb, AsyncContainer) {
        let (weaver, img) = init_weaviate().await;
        weaver.create_default_collection().await;
        (weaver, img)
    }

    #[test]
    async fn creates_default_collection(weaver: WeaviateDb) {
        let collection = weaviate_class_name(DEFAULT_COLLECTION_NAME);
        let default = weaver.get_collection(&collection).await.unwrap();

        assert_eq!(collection, default.name);
        assert_eq!(DEFAULT_COLLECTION_SIZE, default.size);
    }

    #[test]
    async fn creates_collection(weaver: WeaviateDb) {
        let collection = "MyCollection";
        weaver
            .create_vector_collection(collection, DEFAULT_COLLECTION_SIZE as u64)
            .await
            .unwrap();

        let default = weaver.get_collection(collection).await.unwrap();

        assert_eq!(collection, default.name);
    }
}
