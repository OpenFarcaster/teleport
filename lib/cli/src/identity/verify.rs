use clap::Args;

#[derive(Args, Debug)]
#[command(name = "verify", about = "Verify a PeerID file.")]
pub struct VerifyIdentityCommand {
    #[arg(
        short = 'i',
        long,
        help="Path to the PeerId file",
        default_value_t=String::from("./.hub/default_id.protobuf")
    )]
    pub id: String,
}
