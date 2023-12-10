use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
#[command(name = "events", about = "Clear L2 contract events from the database")]
pub struct EventsResetCommand {
    #[arg(short, long, help = "Path to the config file.")]
    config: Option<PathBuf>,

    #[arg(
        long,
        help = "The name of the RocksDB instance. (default: rocks.hub._default)"
    )]
    db_name: Option<String>,
}
