use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub struct StartArgs {
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
