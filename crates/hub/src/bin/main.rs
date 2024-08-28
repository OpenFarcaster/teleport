use clap::Parser;
use teleport_cli::{Cli, Commands};
use teleport_hub::Hub;

type HResult<T> = Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> HResult<()> {
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
