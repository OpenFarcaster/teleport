use anyhow::Result;
use clap::Parser;
use teleport_cli::{Cli, Commands};
use teleport_hub::Hub;

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::Start(_) => Hub::start().await,
        Commands::Identity(_) => todo!(),
        Commands::Status(_) => todo!(),
        Commands::Profile(_) => todo!(),
        Commands::Reset(_) => todo!(),
        Commands::Console(_) => todo!(),
    };

    Ok(())
}
