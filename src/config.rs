use clap::Parser;

const DEFAULT_UPLOAD_PATH: &str = "upload";

#[cfg(feature = "http")]
const DEFAULT_ADDRESS: &str = "0.0.0.0:42069";

#[derive(Debug, Parser)]
#[command(name = "chonkit", author = "biblius", version = "0.1", about = "Chunk documents", long_about = None)]
pub struct StartArgs {
    /// Database URL.
    #[arg(short, long)]
    pub db_url: Option<String>,

    /// RUST_LOG string to use as the env filter.
    #[arg(short, long)]
    pub log: Option<String>,

    /// If using the `FsDocumentStore`, sets its path.
    #[arg(short, long)]
    pub upload_path: Option<String>,

    /// Address to listen on.
    #[cfg(feature = "http")]
    #[arg(short, long)]
    pub address: Option<String>,

    /// Qdrant URL.
    #[cfg(feature = "qdrant")]
    #[arg(short, long)]
    pub qdrant_url: Option<String>,

    /// Weaviate URL.
    #[cfg(feature = "weaviate")]
    #[arg(short, long)]
    pub weaviate_url: Option<String>,

    /// If using the [OpenAiEmbeddings][crate::app::embedder::openai::OpenAiEmbeddings] module, set its endpoint.
    #[cfg(feature = "openai")]
    #[arg(short, long)]
    pub openai_endpoint: Option<String>,

    /// CLI mode execution command
    #[cfg(feature = "cli")]
    #[clap(subcommand)]
    pub command: crate::cli::Execute,
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

    #[cfg(feature = "http")]
    pub fn address(&self) -> String {
        match &self.address {
            Some(addr) => addr.to_string(),
            None => match std::env::var("ADDRESS") {
                Ok(addr) => addr,
                Err(_) => DEFAULT_ADDRESS.to_string(),
            },
        }
    }

    // #[cfg(feature = "openai")]
    pub fn open_ai_key(&self) -> String {
        std::env::var("OPENAI_KEY").expect("Missing OPENAI_KEY in env")
    }
}
