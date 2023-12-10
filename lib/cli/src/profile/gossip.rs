use clap::Args;

#[derive(Args, Debug)]
#[command(name = "gossip", about = "Profile the gossip server's performance.")]
pub struct GossipProfileCommand {
    #[arg(short='n', long, help = "Number of nodes to simulate. Total is threads * nodes", default_value_t = String::from("3:10"))]
    num_nodes: String,
}
