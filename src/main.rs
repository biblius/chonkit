use crate::config::StartArgs;
use app::service::ServiceState;
use core::document::parser::pdf::PdfParser;
use core::document::parser::{DocumentParser, ParseConfig};
use qdrant_client::Qdrant;
use tracing::info;
use tracing_subscriber::EnvFilter;

pub mod app;
pub mod config;
pub mod control;
pub mod core;
pub mod error;

pub const DEFAULT_COLLECTION_NAME: &str = "__default__";
pub const DEFAULT_COLLECTION_MODEL: &str = "Qdrant/all-MiniLM-L6-v2-onnx";

#[tokio::main]
async fn main() {
    let StartArgs {
        address: host,
        port,
        log_level: level,
        qdrant_url,
        db_url,
    } = <StartArgs as clap::Parser>::parse();

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_env_filter(EnvFilter::new("info,h2=off,lopdf=off,chonkit=debug"))
        .init();

    // let file_docx = std::fs::read("test_docs/test.docx").unwrap();
    let file_pdf = std::fs::read("test_docs/test.pdf").unwrap();

    // let out_docx = core::document::load_docx(&file_docx).unwrap();
    let cfg = ParseConfig::default()
        .skip_f(11)
        .skip_b(1)
        .filter(regex::Regex::new("raywenderlich.com").unwrap());

    let pdf_parser = PdfParser::new(cfg);

    let out_pdf = pdf_parser.parse(&file_pdf).unwrap();

    // std::fs::write("test_docs/parsed_docx.txt", out_docx).unwrap();
    std::fs::write("test_docs/parsed_pdf.txt", out_pdf).unwrap();

    let db_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            info!("DATABASE_URL not set, falling back to {db_url}");
            db_url
        }
    };

    let qdrant_url = match std::env::var("QDRANT_URL") {
        Ok(url) => url,
        Err(_) => {
            info!("QDRANT_URL not set, falling back to {qdrant_url}");
            qdrant_url
        }
    };

    let db_pool = app::repo::pg::init(&db_url).await;
    let qdrant = Qdrant::from_url(&qdrant_url).build().unwrap();

    let services = ServiceState::init(db_pool, qdrant).await;

    let addr = format!("{host}:{port}");

    control::http::server(&addr, services).await;
}
