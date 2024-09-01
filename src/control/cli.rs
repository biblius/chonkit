use clap::{Args, Parser, Subcommand};

use crate::app::service::ServiceState;

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
    Meta(DocMetaArgs),
    Sync,
}

#[derive(Debug, Subcommand)]
enum VectorExec {}

#[derive(Debug, Args, Default, Clone)]
struct DocMetaArgs {
    #[arg(long, short)]
    id: uuid::Uuid,
}

pub async fn run(services: ServiceState) {
    let args = CliArgs::parse();
    match args.command {
        Execute::Doc(doc) => match doc {
            DocumentExec::Meta(DocMetaArgs { id }) => {
                let doc = services.document.get_metadata(id).await.unwrap();
                println!("{:?}", doc)
            }
            DocumentExec::Sync => services.document.sync().await.unwrap(),
        },
        Execute::Vec(_) => todo!(),
    }
}
