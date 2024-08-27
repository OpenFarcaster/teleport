use teleport_cli::{Cli, Commands};
use teleport_hub::Hub;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Commands::Start(_) => Hub::start().await,
        Commands::Identity(_) => Ok(()),
        Commands::Status(_) => Ok(()),
        Commands::Profile(_) => Ok(()),
        Commands::Reset(_) => Ok(()),
        Commands::Console(_) => Ok(()),
    };

    Ok(())
}
