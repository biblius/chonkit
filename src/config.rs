use crate::error::ChonkitError;
use clap::Parser;
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Parser)]
pub struct StartArgs {
    #[arg(short, long, default_value = "config.json")]
    pub config_path: String,

    #[arg(short, long, default_value = "0.0.0.0")]
    pub address: String,

    #[arg(short, long, default_value = "42069")]
    pub port: u16,

    #[arg(short, long, default_value = "INFO")]
    pub log_level: tracing::Level,

    #[arg(short, long, default_value = "http://localhost:6334")]
    pub qdrant_url: String,

    #[arg(
        short,
        long,
        default_value = "postgresql://postgres:postgres@localhost:5433/chonkit"
    )]
    pub db_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub directory: PathBuf,
}

impl Config {
    pub fn read(path: impl AsRef<Path>) -> Result<Self, ChonkitError> {
        let config = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&config)?)
    }
}
