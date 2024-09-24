use clap::Parser;

#[tokio::main]
async fn main() {
    let args = chonkit::config::StartArgs::parse();
    let state = chonkit::app::state::AppState::new(&args).await;
    chonkit::cli::run(args.command, state).await;
}
