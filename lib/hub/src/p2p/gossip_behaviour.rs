use libp2p::allow_block_list::{self, BlockedPeers};
use libp2p::gossipsub;
use libp2p::identify;
use libp2p::ping;
use libp2p::swarm::NetworkBehaviour;
use void::Void;

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "GossipBehaviourEvent")]
pub struct GossipBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub blocked_peers: allow_block_list::Behaviour<BlockedPeers>,
}

#[derive(Debug)]
pub enum GossipBehaviourEvent {
    Gossipsub(gossipsub::Event),
    Identify(identify::Event),
    Ping(ping::Event),
    BlockedPeer,
}

impl From<gossipsub::Event> for GossipBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        GossipBehaviourEvent::Gossipsub(event)
    }
}

impl From<identify::Event> for GossipBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        GossipBehaviourEvent::Identify(event)
    }
}

impl From<ping::Event> for GossipBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        GossipBehaviourEvent::Ping(event)
    }
}

impl From<Void> for GossipBehaviourEvent {
    fn from(_: Void) -> Self {
        GossipBehaviourEvent::BlockedPeer
    }
}
