use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
#[command(name = "full", about = "Completely remove the database")]
pub struct FullResetCommand {
    #[arg(short, long, help = "Path to the config file.")]
    config: Option<PathBuf>,

    #[arg(
        long,
        help = "The name of the RocksDB instance. (default: rocks.hub._default)"
    )]
    db_name: Option<String>,
}
