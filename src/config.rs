use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub struct StartArgs {
    /// Address to listen on.
    #[arg(short, long, default_value = "0.0.0.0")]
    pub address: String,

    /// Port to listen on.
    #[arg(short, long, default_value = "42069")]
    pub port: u16,

    #[arg(short, long, default_value = "INFO")]
    pub log_level: tracing::Level,

    /// Qdrant url.
    #[arg(short, long)]
    pub qdrant_url: Option<String>,

    /// Sets the database URL.
    #[arg(short, long)]
    pub db_url: Option<String>,

    /// If using the `FsDocumentStore`, sets its path.
    #[arg(short, long)]
    pub upload_path: Option<String>,
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

    pub fn qdrant_url(&self) -> String {
        match &self.qdrant_url {
            Some(url) => url.to_string(),
            None => match std::env::var("QDRANT_URL") {
                Ok(url) => url,
                Err(_) => panic!("Qdrant url not found; Pass --qdrant-url or set QDRANT_URL"),
            },
        }
    }
}
