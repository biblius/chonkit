use crate::{
    app::state::AppState,
    core::{
        document::parser::ParseConfig,
        model::{document::DocumentType, Pagination},
        service::document::{dto::DocumentUpload, DocumentService},
    },
};
use clap::{Args, Subcommand};
use std::{collections::HashMap, path::PathBuf};
use validify::Validate;

#[derive(Debug, Subcommand)]
pub enum Execute {
    /// Invoke document service functions.
    #[clap(subcommand)]
    Doc(DocumentExec),

    /// Invoke vector service functions.
    #[clap(subcommand)]
    Vec(VectorExec),
}

#[derive(Debug, Subcommand)]
pub enum DocumentExec {
    /// Get full config details for a single document.
    Meta(IdArg),

    /// Sync the documents repository with the document storage.
    Sync,

    /// List document metadata.
    List(ListArgs),

    /// Preview chunks for a document using its parsing and chunking config.
    Chunkp(ChunkpArg),

    /// Preview text for a document using the given parsing config.
    Parsep(ParseArg),

    /// Upload a document from the file system.
    Upload(UploadArg),
}

#[derive(Debug, Args, Default, Clone)]
pub struct UploadArg {
    #[arg(long, short)]
    pub name: String,
    #[arg(long, short)]
    pub path: PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum ChunkMode {
    /// Sliding window
    Sl(IdArg),
    /// Snapping window
    Sn(ChunkpArg),
}

#[derive(Debug, Subcommand)]
pub enum VectorExec {}

#[derive(Debug, Args, Default, Clone)]
pub struct IdArg {
    /// Document ID.
    #[arg(long, short)]
    id: uuid::Uuid,
}

#[derive(Debug, Args, Default, Clone)]
pub struct ParseArg {
    /// Document ID.
    #[arg(long, short)]
    id: uuid::Uuid,

    /// Skip the first `start` elements.
    #[arg(long, short)]
    start: usize,

    /// Omit the last `end` elements.
    #[arg(long, short)]
    end: usize,

    #[arg(long, short, action)]
    range: bool,

    /// Parsing specific text elements filters.
    #[arg(long, short)]
    filter: Option<String>,
}

#[derive(Debug, Args, Default, Clone)]
pub struct ChunkpArg {
    /// Document ID.
    #[arg(long, short)]
    id: uuid::Uuid,

    /// The start of the range of chunks to print.
    #[arg(long, short, default_value = "0")]
    start: usize,

    /// The end of the range of chunks to print.
    #[arg(long, short, default_value = "10")]
    end: usize,

    /// If given, writes the range of chunks as json to the path.
    #[arg(long, short)]
    out: Option<PathBuf>,
}

#[derive(Debug, Args, Default, Clone)]
pub struct ListArgs {
    #[arg(long, short, default_value = "10")]
    limit: usize,
    #[arg(long, short, default_value = "1")]
    offset: usize,
}

pub async fn run(command: Execute, state: AppState) {
    match command {
        Execute::Doc(doc) => {
            let store = state.store("fs".try_into().unwrap());

            let service = DocumentService::new(state.postgres.clone());
            match doc {
                DocumentExec::Meta(IdArg { id }) => {
                    let doc = service.get_config(id).await.unwrap();
                    println!("{:#?}", doc);
                }
                DocumentExec::Sync => service.sync(&*store).await.unwrap(),
                DocumentExec::List(ListArgs { limit, offset }) => {
                    let p = Pagination::new(limit, offset);
                    p.validate().unwrap();
                    let docs = service.list_documents(p, None).await.unwrap();
                    println!("{:#?}", docs);
                }
                DocumentExec::Chunkp(ChunkpArg {
                    id,
                    start,
                    end,
                    out,
                }) => {
                    let document = service.get_config(id).await.unwrap();
                    let content = service
                        .parse_preview(&*store, id, document.parse_config.unwrap())
                        .await
                        .unwrap();
                    let chunks = service
                        .chunk_preview(&content, document.chunk_config.unwrap(), None)
                        .await
                        .unwrap();

                    let chunks = match chunks {
                        crate::core::chunk::ChunkedDocument::Ref(chunks) => {
                            chunks.into_iter().map(String::from).collect()
                        }
                        crate::core::chunk::ChunkedDocument::Owned(chunks) => chunks,
                    };
                    print_chunks(start, end, &chunks);
                    if let Some(out) = out {
                        write_chunks(chunks, start, end, out);
                    }
                }
                DocumentExec::Parsep(ParseArg {
                    id,
                    start,
                    end,
                    filter,
                    range,
                }) => {
                    let filters = filter.map(csv_to_vec).unwrap_or_default();
                    let mut cfg = ParseConfig::new(start, end);
                    if range {
                        cfg = cfg.use_range();
                    }
                    for filter in filters {
                        cfg = cfg.with_filter(&filter).unwrap();
                    }
                    let parsed = service.parse_preview(&*store, id, cfg).await.unwrap();
                    println!("{parsed}");
                }
                DocumentExec::Upload(UploadArg { name, path }) => {
                    let file = std::fs::read(path).unwrap();
                    let ty = DocumentType::try_from_file_name(&name).unwrap();
                    let upload = DocumentUpload::new(name, ty, &file);
                    let doc = service.upload(&*store, upload).await.unwrap();
                    println!("{:#?}", doc);
                }
            }
        }
        Execute::Vec(_) => todo!(),
    }
}

fn print_chunks(start: usize, end: usize, chunks: &[String]) {
    for (i, chunk) in chunks.iter().enumerate() {
        if i < start.saturating_sub(1) {
            continue;
        }
        if i > end {
            break;
        }
        println!("Chunk {i} {:=>60}", "v");
        println!();
        println!("{chunk}");
        println!();

        println!("Total chunks: {}", chunks.len());
    }
}

fn write_chunks(chunks: Vec<String>, start: usize, end: usize, out: PathBuf) {
    let total = chunks.len();
    let chunks = chunks
        .into_iter()
        .enumerate()
        .skip(start)
        .rev()
        .skip(total - end)
        .rev()
        .collect::<HashMap<usize, String>>();

    std::fs::write(out, serde_json::to_string(&chunks).unwrap()).unwrap();
}

fn csv_to_vec(csv: String) -> Vec<String> {
    csv.split(',').map(String::from).collect()
}
