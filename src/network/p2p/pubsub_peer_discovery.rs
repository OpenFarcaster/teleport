use std::{sync::Arc, time::Duration};

use libp2p::bytes::Bytes;
use libp2p::futures::{Future, FutureExt};
use libp2p::gossipsub::Event as GossipsubEvent;
use libp2p::swarm::SwarmEvent;
use libp2p::{gossipsub::IdentTopic, Swarm};
use std::pin::Pin;
use tokio::sync::Mutex;
use tokio::time;

use crate::core::errors::HubError;

use super::gossip_behaviour::{GossipBehaviour, GossipBehaviourEvent};
use super::handle_swarm_event::SwarmEventHandler;

use prost::Message;

#[derive(Message)]
pub struct Peer {
    #[prost(bytes, tag = "1")]
    pub public_key: Vec<u8>,
    #[prost(bytes, repeated, tag = "2")]
    pub addrs: Vec<Vec<u8>>,
}

pub struct PubSubPeerDiscovery {
    interval: Duration,
    listen_only: bool,
    is_started: bool,
    topic: IdentTopic,
    swarm: Arc<Mutex<Swarm<GossipBehaviour>>>,
    stop_signal: Arc<Mutex<bool>>,
}

impl PubSubPeerDiscovery {
    pub fn new(
        interval: Duration,
        listen_only: bool,
        swarm: Arc<Mutex<Swarm<GossipBehaviour>>>,
        topic: IdentTopic,
    ) -> Self {
        Self {
            interval,
            listen_only,
            is_started: false,
            topic,
            swarm,
            stop_signal: Arc::new(Mutex::new(false)),
        }
    }

    pub fn is_started(&self) -> bool {
        self.is_started
    }

    pub async fn start(&mut self) -> Result<(), HubError> {
        if self.is_started {
            return Ok(());
        }

        self.swarm
            .lock()
            .await
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.topic)
            .unwrap();

        self.is_started = true;

        if self.listen_only {
            return Ok(());
        }

        broadcast(self.swarm.clone(), &self.topic).await;

        let stop_signal = self.stop_signal.clone();
        let swarm = self.swarm.clone();
        let topic = self.topic.clone();
        let interval = self.interval;

        // Periodically call broadcast again
        tokio::spawn(async move {
            let mut interval = time::interval(interval);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if *stop_signal.lock().await {
                            break;
                        }

                        broadcast(swarm.clone(), &topic).await;
                    }

                    _ = tokio::signal::ctrl_c() => {
                        *stop_signal.lock().await = true;
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), HubError> {
        if !self.is_started {
            return Ok(());
        }

        // Unsubscribe from the topics
        self.swarm
            .lock()
            .await
            .behaviour_mut()
            .gossipsub
            .unsubscribe(&self.topic)
            .unwrap();

        self.is_started = false;

        Ok(())
    }
}

impl SwarmEventHandler for PubSubPeerDiscovery {
    fn handle<'a>(
        &'a self,
        event: &'a SwarmEvent<GossipBehaviourEvent, std::io::Error>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        async move {
            if !self.is_started {
                return;
            }

            if let SwarmEvent::Behaviour(event) = event {
                match event {
                    GossipBehaviourEvent::Gossipsub(event) => {
                        if let GossipsubEvent::Message {
                            propagation_source,
                            message_id: _,
                            message,
                        } = event
                        {
                            if self.topic.to_string() != message.topic.to_string() {
                                return;
                            }

                            let mut locked_swarm = self.swarm.lock().await;
                            let local_peer_id = locked_swarm.local_peer_id();

                            if local_peer_id == propagation_source {
                                return;
                            }

                            println!(
                                "Received message from {:?}: {:?}",
                                propagation_source, message
                            );

                            let decoded_peer =
                                Peer::decode(Bytes::from(message.data.clone())).unwrap();

                            for addr in decoded_peer.addrs {
                                let multi_addr = libp2p::Multiaddr::try_from(addr).unwrap();
                                locked_swarm.dial(multi_addr).unwrap();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        .boxed()
    }
}

pub async fn broadcast(swarm: Arc<Mutex<Swarm<GossipBehaviour>>>, topic: &IdentTopic) {
    let locked_swarm = swarm.lock().await;

    let peer_id_bytes = locked_swarm.local_peer_id().to_bytes();
    let listener_addresses_bytes: Vec<Vec<u8>> =
        locked_swarm.listeners().map(|la| la.to_vec()).collect();

    // Encode peer_id and listener_addresses the same way as JS
    let peer = Peer {
        public_key: peer_id_bytes.clone(), // TODO: this is likely wrong. JS does this over public key, not peer id
        addrs: listener_addresses_bytes,
    };

    let mut encoded_peer = Vec::new();
    peer.encode(&mut encoded_peer).unwrap();

    let _ = swarm
        .lock()
        .await
        .behaviour_mut()
        .gossipsub
        .publish(topic.clone(), encoded_peer);
}
