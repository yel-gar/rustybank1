use crate::cli::Args;
use crate::networking::Server;
use clap::Parser;
use tracing::warn;

mod cli;
mod constants;
mod messages;
mod networking;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    if args.disable_ip_limit {
        warn!("IP limit disabled")
    }
    let server = Server::new(&args).await?;
    server.run().await;

    Ok(())
}
