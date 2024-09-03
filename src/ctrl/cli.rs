use clap::{Args, Parser, Subcommand};

use crate::{
    app::service::ServiceState,
    core::{chunk::ChunkConfig, repo::Pagination},
};

#[derive(Debug, Parser)]
#[command(name = "chonkit-cli", author = "biblius", version = "0.1", about = "Chunk documents", long_about = None)]
struct CliArgs {
    #[clap(subcommand)]
    command: Execute,
}

#[derive(Debug, Subcommand)]
enum Execute {
    #[clap(subcommand)]
    Doc(DocumentExec),
    #[clap(subcommand)]
    Vec(VectorExec),
}

#[derive(Debug, Subcommand)]
enum DocumentExec {
    Meta(IdArg),
    Sync,
    List(ListArgs),
    #[clap(subcommand)]
    Chunkp(ChunkMode),
}

#[derive(Debug, Subcommand)]
enum ChunkMode {
    Sw(IdArg),
    Ssw(ChunkpArg),
    Rec(IdArg),
}

#[derive(Debug, Subcommand)]
enum VectorExec {}

#[derive(Debug, Args, Default, Clone)]
struct IdArg {
    #[arg(long, short)]
    id: uuid::Uuid,
}

#[derive(Debug, Args, Default, Clone)]
struct ChunkpArg {
    /// Document ID.
    #[arg(long, short)]
    id: uuid::Uuid,

    /// The start of the range of chunks to print.
    #[arg(long, short, default_value = "usize::MIN")]
    start: usize,

    /// The end of the range of chunks to print.
    #[arg(long, short, default_value = "usize::MAX")]
    end: usize,
}

#[derive(Debug, Args, Default, Clone)]
struct ListArgs {
    #[arg(long, short, default_value = "10")]
    limit: usize,
    #[arg(long, short, default_value = "1")]
    offset: usize,
}

pub async fn run(services: ServiceState) {
    let args = CliArgs::parse();
    match args.command {
        Execute::Doc(doc) => match doc {
            DocumentExec::Meta(IdArg { id }) => {
                let doc = services.document.get_metadata(id).await.unwrap();
                println!("{:#?}", doc);
            }
            DocumentExec::Sync => services.document.sync().await.unwrap(),
            DocumentExec::List(ListArgs { limit, offset }) => {
                let docs = services
                    .document
                    .list_documents(Pagination::new(limit, offset))
                    .await
                    .unwrap();
                println!("{:#?}", docs);
            }
            DocumentExec::Chunkp(mode) => match mode {
                ChunkMode::Sw(IdArg { id }) => {
                    let preview = services
                        .document
                        .chunk_preview(id, ChunkConfig::sw(500, 100))
                        .await
                        .unwrap();
                    println!("{:#?}", preview);
                    for (i, preview) in preview.into_iter().enumerate() {
                        println!("Chunk {i} {{");
                        println!("{preview}");
                        println!("}}");
                    }
                }
                ChunkMode::Ssw(ChunkpArg { id, start, mut end }) => {
                    if end == 0 {
                        end = usize::MAX;
                    }

                    let preview = services
                        .document
                        .chunk_preview(
                            id,
                            ChunkConfig::ssw(
                                1000,
                                10,
                                vec![
                                    "o.".to_string(),
                                    "d.o".to_string(),
                                    "d.".to_string(),
                                    "0".to_string(),
                                    "1".to_string(),
                                    "2".to_string(),
                                    "3".to_string(),
                                    "4".to_string(),
                                    "5".to_string(),
                                    "6".to_string(),
                                    "7".to_string(),
                                    "8".to_string(),
                                    "9".to_string(),
                                ],
                                vec![
                                    "com".to_string(),
                                    "www".to_string(),
                                    "o.".to_string(),
                                    "o.o.".to_string(),
                                    "d.".to_string(),
                                ],
                            ),
                        )
                        .await
                        .unwrap();

                    for (i, preview) in preview.iter().enumerate() {
                        if i < start.saturating_sub(1) {
                            continue;
                        }
                        if i > end {
                            break;
                        }
                        println!("Chunk {i} {:=>60}", "v");
                        println!();
                        println!("{preview}");
                        println!();
                    }

                    println!("Total chunks: {}", preview.len());
                }

                ChunkMode::Rec(IdArg { id }) => {
                    let preview = services
                        .document
                        .chunk_preview(
                            id,
                            ChunkConfig::rec(
                                500,
                                100,
                                vec![
                                    "\n\n".to_string(),
                                    "\n".to_string(),
                                    " ".to_string(),
                                    "".to_string(),
                                ],
                            ),
                        )
                        .await
                        .unwrap();
                    for (i, preview) in preview.into_iter().enumerate() {
                        println!("Chunk {i} {{");
                        println!("{preview}");
                        println!("}}");
                    }
                }
            },
        },
        Execute::Vec(_) => todo!(),
    }
}
