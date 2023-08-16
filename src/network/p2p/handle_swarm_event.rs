use std::io;

use libp2p::{futures::Future, swarm::SwarmEvent};

use super::gossip_behaviour::GossipBehaviourEvent;

use std::pin::Pin;

pub trait SwarmEventHandler {
    fn handle<'a>(
        &'a self,
        event: &'a SwarmEvent<GossipBehaviourEvent, io::Error>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>>;
}
