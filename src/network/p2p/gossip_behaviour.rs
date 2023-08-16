use libp2p::allow_block_list::{self, AllowedPeers, BlockedPeers};
use libp2p::gossipsub::{self};
use libp2p::identify;
use libp2p::swarm::NetworkBehaviour;

#[derive(NetworkBehaviour)]
pub struct GossipBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub identify: identify::Behaviour,
    pub allowed_peers: allow_block_list::Behaviour<AllowedPeers>,
    pub blocked_peers: allow_block_list::Behaviour<BlockedPeers>,
}
