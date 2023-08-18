use std::str::FromStr;

use libp2p::Multiaddr;
use network::p2p::gossip_node::NodeOptions;

mod core;
mod network;
mod rpc;
mod storage;
mod teleport;

#[tokio::main]
async fn main() {
    env_logger::init();

    let dial_peer_addr = std::env::args().nth(1);

    let node_options = NodeOptions::new(
        core::protobufs::generated::FarcasterNetwork::Mainnet,
        None,
        None,
        None,
        None,
        None,
        None,
    );

    let (mut gossip_node, command_sender) =
        network::p2p::gossip_node::GossipNode::new(node_options);

    let mut bootstrap_addrs = Vec::new();
    if dial_peer_addr.is_some() {
        let addr = Multiaddr::from_str(dial_peer_addr.unwrap().as_str());

        bootstrap_addrs.push(addr.unwrap());
    }

    gossip_node.start(bootstrap_addrs).await;
}
