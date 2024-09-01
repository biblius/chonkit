use clap::Parser;

use crate::{DB_URL, VEC_DB_URL};

#[derive(Debug, Clone, Parser)]
pub struct StartArgs {
    #[arg(short, long, default_value = "0.0.0.0")]
    pub address: String,

    #[arg(short, long, default_value = "42069")]
    pub port: u16,

    #[arg(short, long, default_value = "INFO")]
    pub log_level: tracing::Level,

    #[arg(short, long)]
    pub qdrant_url: Option<String>,

    #[arg(short, long)]
    pub db_url: Option<String>,
}

impl StartArgs {
    pub fn db_url(&self) -> String {
        match &self.db_url {
            Some(url) => url.to_string(),
            None => match std::env::var("DATABASE_URL") {
                Ok(url) => url,
                Err(_) => DB_URL.to_owned(),
            },
        }
    }

    pub fn qdrant_url(&self) -> String {
        match &self.qdrant_url {
            Some(url) => url.to_string(),
            None => match std::env::var("QDRANT_URL") {
                Ok(url) => url,
                Err(_) => VEC_DB_URL.to_owned(),
            },
        }
    }
}
