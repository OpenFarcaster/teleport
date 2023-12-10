use clap::Args;

#[derive(Args, Debug)]
#[command(name = "console", about = "Start a REPL console")]
pub struct ConsoleCommand {
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
}
