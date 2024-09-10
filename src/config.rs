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
    pub vec_db_url: Option<String>,

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

    pub fn vec_db_url(&self) -> String {
        match &self.vec_db_url {
            Some(url) => url.to_string(),
            None => {
                match std::env::var("VEC_DATABASE_URL") {
                    Ok(url) => url,
                    Err(_) => {
                        panic!("Vector database url not found; Pass --vec-db-url or set VEC_DATABASE_URL")
                    }
                }
            }
        }
    }
}
