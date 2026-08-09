use crate::cli::Args;
use clap::Parser;
use crate::networking::Server;

mod cli;
mod constants;
mod networking;
mod util;
mod messages;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let server = Server::new(&args).await?;
    server.run().await;

    Ok(())
}
