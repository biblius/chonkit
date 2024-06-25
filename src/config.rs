use crate::error::ChonkitError;
use clap::Parser;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Parser)]
pub struct StartArgs {
    #[arg(short, long, default_value = "config.json")]
    pub config_path: String,

    #[arg(short, long, default_value = "127.0.0.1")]
    pub address: String,

    #[arg(short, long, default_value = "3031")]
    pub port: u16,

    #[arg(short, long, default_value = "INFO")]
    pub log_level: tracing::Level,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub directory: PathBuf,
    pub hf: HfConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HfConfig {
    pub token: String,
    pub cache_dir: Option<PathBuf>,
}

impl Config {
    pub fn read(path: impl AsRef<Path>) -> Result<Self, ChonkitError> {
        let config = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&config)?)
    }
}
