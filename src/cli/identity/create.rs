use clap::Args;

#[derive(Args, Debug)]
#[command(
    name = "create",
    about = "Create a new Peer ID and write it to a file. This will overwrite the default file."
)]
pub struct CreateIdentityCommand {
    #[arg(
        short='O',
        long,
        help="Path to where the generated PeerIds should be stored",
        default_value_t=String::from("./.hub")
    )]
    output: String,

    #[arg(
        short = 'N',
        long,
        help = "Number of PeerIds to generate",
        default_value_t = 1
    )]
    count: u32,
}
