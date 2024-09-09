use clap::Args;

#[derive(Args, Debug)]
#[command(name = "status", about = "Reports the db and sync status of the hub")]
pub struct StatusCommand {
    #[arg(
        short = 's',
        long,
        help = "Farcaster RPC server address:port to connect to (eg. 127.0.0.1:2283)",
        default_value_t = String::from("127.0.0.1:2283")
    )]
    server: String,

    #[arg(
        long,
        help = "Allow insecure connections to the RPC server",
        default_value_t = false
    )]
    insecure: bool,

    #[arg(
        long,
        help = "Keep running and periodically report status",
        default_value_t = false
    )]
    watch: bool,

    #[arg(
        short = 'p',
        long,
        help = "Peer ID of the hub to compare with (defaults to bootstrap peers)"
    )]
    peer_id: Option<String>,
}
