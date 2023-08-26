use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug)]
#[command(
    name = "storage",
    about = "Profile the storage layout of the hub, accounting for all the storage."
)]
pub struct StorageProfileCommand {
    #[arg(long, help = "The name of the RocksDB instance")]
    db_name: String,

    #[arg(short = 'c', long, help = "Path to a config file with options")]
    config: Option<PathBuf>,

    #[arg(short = 'o', long, help = "Path to a file to write the profile to")]
    output: Option<PathBuf>,
}
