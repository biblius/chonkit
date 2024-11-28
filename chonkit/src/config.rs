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

const DEFAULT_UPLOAD_PATH: &str = "upload";

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
    #[arg(short = 'c', long)]
    allowed_origins: Option<String>,

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
}

impl StartArgs {
    pub fn db_url(&self) -> String {
        match &self.db_url {
            Some(url) => url.to_string(),
            None => match std::env::var("DATABASE_URL") {
                Ok(url) => url,
                Err(_) => panic!("Database url not found; Pass --db-url or set DATABASE_URL"),
            },
        }
    }

    pub fn log(&self) -> String {
        match &self.log {
            Some(log) => log.to_string(),
            None => match std::env::var("RUST_LOG") {
                Ok(url) => url,
                Err(_) => "info".to_string(),
            },
        }
    }

    pub fn upload_path(&self) -> String {
        match &self.upload_path {
            Some(path) => path.to_string(),
            None => match std::env::var("UPLOAD_PATH") {
                Ok(path) => path,
                Err(_) => DEFAULT_UPLOAD_PATH.to_string(),
            },
        }
    }

    #[cfg(feature = "qdrant")]
    pub fn qdrant_url(&self) -> String {
        match &self.qdrant_url {
            Some(url) => url.to_string(),
            None => match std::env::var("QDRANT_URL") {
                Ok(url) => url,
                Err(_) => {
                    panic!("Qdrant url not found; Pass --qdrant-url (-q) or set QDRANT_URL")
                }
            },
        }
    }

    #[cfg(feature = "weaviate")]
    pub fn weaviate_url(&self) -> String {
        match &self.weaviate_url {
            Some(url) => url.to_string(),
            None => match std::env::var("WEAVIATE_URL") {
                Ok(url) => url,
                Err(_) => {
                    panic!("Weaviate url not found; Pass --weaviate-url (-w) or set WEAVIATE_URL")
                }
            },
        }
    }

    pub fn address(&self) -> String {
        match &self.address {
            Some(addr) => addr.to_string(),
            None => match std::env::var("ADDRESS") {
                Ok(addr) => addr,
                Err(_) => DEFAULT_ADDRESS.to_string(),
            },
        }
    }

    pub fn allowed_origins(&self) -> Vec<String> {
        match &self.allowed_origins {
            Some(origins) => origins.split(',').map(String::from).collect(),
            None => match std::env::var("ALLOWED_ORIGINS") {
                Ok(origins) => origins.split(',').map(String::from).collect(),
                Err(_) => panic!(
                    "Allowed origins not found; Pass --allowed-origins (-c) or set ALLOWED_ORIGINS as a comma separated list"
                ),
            },
        }
    }

    #[cfg(feature = "openai")]
    pub fn open_ai_key(&self) -> String {
        std::env::var("OPENAI_KEY").expect("Missing OPENAI_KEY in env")
    }

    #[cfg(feature = "fe-remote")]
    pub fn fembed_url(&self) -> String {
        match &self.fembed_url {
            Some(url) => url.to_string(),
            None => match std::env::var("FEMBED_URL") {
                Ok(url) => url,
                Err(_) => {
                    panic!("Fembed url not found; Pass --fembed-url (-f) or set FEMBED_URL")
                }
            },
        }
    }
}
