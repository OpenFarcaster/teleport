use std::collections::HashMap;
use std::time::Duration;

use super::gossip_behaviour::{GossipBehaviour, GossipBehaviourEvent};
use libp2p::futures::channel::oneshot;
use libp2p::futures::StreamExt;
use libp2p::gossipsub::IdentTopic;
use libp2p::swarm::derive_prelude::Either;
use libp2p::swarm::SwarmEvent;
use libp2p::{futures::channel::mpsc, Swarm};
use libp2p::{Multiaddr, PeerId};
use prost::Message;
use teleport_common::errors::{BadRequestType, HubError, UnavailableType};
use teleport_common::protobufs::{self, generated};
use tokio::time::{interval, Interval};
use void::Void;

type GossipNodeSwarmEvent =
    SwarmEvent<GossipBehaviourEvent, Either<Either<Either<Void, std::io::Error>, Void>, Void>>;

pub enum Command {
    StartListening {
        addr: Multiaddr,
    },
    Bootstrap,
    GossipMessage {
        message: protobufs::generated::Message,
    },
    GossipContactInfo {
        contact_info: protobufs::generated::ContactInfoContent,
    },
    DialMultiAddr {
        addr: Multiaddr,
    },
    GetState {
        sender: oneshot::Sender<EventLoopState>,
    },
}

#[derive(Debug, Clone)]
pub struct EventLoopState {
    pub is_listening: bool,
    pub subscribed_topics: Vec<IdentTopic>,
    pub connected_peers: HashMap<Multiaddr, PeerId>,
    pub external_addrs: Vec<Multiaddr>,
    pub primary_topic: IdentTopic,
    pub contact_info_topic: IdentTopic,
    pub peer_discovery_topic: IdentTopic,
    pub bootstrap_addrs: Vec<Multiaddr>,
}

pub struct EventLoop {
    swarm: Swarm<GossipBehaviour>,
    command_receiver: mpsc::Receiver<Command>,
    peer_discovery_interval: Option<Interval>,
    peer_check_interval: Option<Interval>,
    state: EventLoopState,
}

impl EventLoop {
    pub fn new(
        network: generated::FarcasterNetwork,
        swarm: Swarm<GossipBehaviour>,
        command_receiver: mpsc::Receiver<Command>,
    ) -> Self {
        let network_num: i32 = network as i32;

        let primary_topic =
            IdentTopic::new(format!("f_network_{}_primary", network_num.to_string()));
        let contact_info_topic = IdentTopic::new(format!(
            "f_network_{}_contact_info",
            network_num.to_string()
        ));
        let peer_discovery_topic = IdentTopic::new(format!(
            "f_network_{}_peer_discovery",
            network_num.to_string()
        ));

        let state = EventLoopState {
            is_listening: false,
            subscribed_topics: vec![],
            connected_peers: HashMap::new(),
            external_addrs: vec![],
            primary_topic,
            contact_info_topic,
            peer_discovery_topic,
            bootstrap_addrs: vec![],
        };

        EventLoop {
            swarm,
            command_receiver,
            peer_discovery_interval: None,
            peer_check_interval: None,
            state,
        }
    }

    pub fn with_peer_discovery_interval(mut self, interval: Interval) -> Self {
        self.peer_discovery_interval = Some(interval);
        self
    }

    pub fn with_peer_check_interval(mut self, interval: Interval) -> Self {
        self.peer_check_interval = Some(interval);
        self
    }

    pub fn with_primary_topic(mut self, topic: IdentTopic) -> Self {
        self.state.primary_topic = topic;
        self
    }

    pub fn with_contact_info_topic(mut self, topic: IdentTopic) -> Self {
        self.state.contact_info_topic = topic;
        self
    }

    pub fn with_peer_discovery_topic(mut self, topic: IdentTopic) -> Self {
        self.state.peer_discovery_topic = topic;
        self
    }

    pub fn with_bootstrap_addrs(mut self, addrs: Vec<Multiaddr>) -> Self {
        self.state.bootstrap_addrs = addrs;
        self
    }

    pub async fn run(&mut self) {
        let peer_discovery_interval = interval(Duration::from_secs(10));
        let periodic_peer_check_interval = interval(Duration::from_secs(4 * 60 * 60));

        let _ = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.state.peer_discovery_topic);

        let _ = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.state.primary_topic);

        let _ = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.state.contact_info_topic);

        self.state.is_listening = true;
        self.state
            .subscribed_topics
            .push(self.state.peer_discovery_topic.clone());
        self.state
            .subscribed_topics
            .push(self.state.primary_topic.clone());
        self.state
            .subscribed_topics
            .push(self.state.contact_info_topic.clone());
        self.peer_discovery_interval = Some(peer_discovery_interval);
        self.peer_check_interval = Some(periodic_peer_check_interval);

        loop {
            tokio::select! {
                event = self.swarm.next() => self.handle_event(event.expect("Failed to handle event")),
                command = self.command_receiver.next() => match command {
                    Some(command) => self.handle_command(command),
                    None => return,
                },
                _ = self.peer_discovery_interval.as_mut().unwrap().tick() => {
                    self.peer_discovery_broadcast();
                }
                _ = self.peer_check_interval.as_mut().unwrap().tick() => {
                    self.bootstrap();
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("Received ctrl-c");
                    break;
                }
            }
        }
    }

    fn handle_event(&mut self, event: GossipNodeSwarmEvent) {
        match event {
            SwarmEvent::Behaviour(behaviour_event) => {
                match behaviour_event {
                    GossipBehaviourEvent::Gossipsub(gossipsub_event) => match gossipsub_event {
                        libp2p::gossipsub::Event::Message {
                            propagation_source,
                            message_id,
                            message,
                        } => {
                            // Handle pubsub peer discovery event
                            if message.topic.to_string()
                                == self.state.peer_discovery_topic.to_string()
                            {
                                let local_external_addrs: Vec<String> = self
                                    .swarm
                                    .external_addresses()
                                    .map(|a| a.to_string())
                                    .collect();

                                let decoded_peer = super::gossip_node::Peer::decode(
                                    libp2p::bytes::Bytes::from(message.data.clone()),
                                )
                                .unwrap();

                                for addr in decoded_peer.addrs {
                                    let multi_addr = libp2p::Multiaddr::try_from(addr).unwrap();

                                    // Do not connect to self
                                    if local_external_addrs.contains(&multi_addr.to_string()) {
                                        continue;
                                    }

                                    // TODO: Do not connect to addresses we have already connected to

                                    println!("Dialing pubsub peer disc {:?}", multi_addr);
                                    self.dial_multi_addr(multi_addr).unwrap();
                                }
                                return;
                            }

                            // Handle other events

                            let raw_data = message.data;
                            let decoded_message =
                                generated::GossipMessage::decode(raw_data.as_ref()).unwrap();

                            match decoded_message.content {
                                Some(generated::gossip_message::Content::Message(data)) => {
                                    // println!("Message: {:?}", data);
                                }
                                _ => {
                                    println!("Unknown content {:?}", decoded_message.content);
                                }
                            }
                        }
                        libp2p::gossipsub::Event::Subscribed { peer_id, topic } => {
                            println!("Subscribed: {:?}", (peer_id, topic))
                        }
                        libp2p::gossipsub::Event::Unsubscribed { peer_id, topic } => {
                            println!("Unsubscribed: {:?}", (peer_id, topic))
                        }
                        libp2p::gossipsub::Event::GossipsubNotSupported { peer_id } => {
                            println!("GossipsubNotSupported: {:?}", (peer_id))
                        }
                    },
                    GossipBehaviourEvent::Identify(_) => println!("Identify"),
                    GossipBehaviourEvent::Ping(_) => println!("Ping"),
                    GossipBehaviourEvent::BlockedPeer => println!("BlockedPeer"),
                }
            }
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                num_established,
                concurrent_dial_errors,
                established_in,
            } => println!("Connection established: {:?}", (peer_id, connection_id)),
            SwarmEvent::ConnectionClosed {
                peer_id,
                connection_id,
                endpoint,
                num_established,
                cause,
            } => {
                println!("Connection closed: {:?}", (peer_id, connection_id))
            }
            SwarmEvent::IncomingConnection {
                connection_id,
                local_addr,
                send_back_addr,
            } => {
                println!("Incoming connection: {:?}", (connection_id, local_addr))
            }
            SwarmEvent::IncomingConnectionError {
                connection_id,
                local_addr,
                send_back_addr,
                error,
            } => {
                println!(
                    "Incoming connection error: {:?}",
                    (connection_id, local_addr, error.to_string())
                )
            }
            SwarmEvent::OutgoingConnectionError {
                connection_id,
                peer_id,
                error,
            } => {
                println!(
                    "Outgoing connection error: {:?}",
                    (connection_id, peer_id, error)
                )
            }
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => {
                println!("New listen addr: {:?}", (listener_id, address))
            }
            SwarmEvent::ExpiredListenAddr {
                listener_id,
                address,
            } => {
                println!("Expired listen addr: {:?}", (listener_id, address))
            }
            SwarmEvent::ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => {
                println!("Listener closed: {:?}", (listener_id, addresses, reason))
            }
            SwarmEvent::ListenerError { listener_id, error } => {
                println!("Listener error: {:?}", (listener_id, error))
            }
            SwarmEvent::Dialing {
                peer_id,
                connection_id,
            } => println!("Dialing: {:?}", (peer_id, connection_id)),
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::StartListening { addr } => {
                self.swarm.listen_on(addr.clone()).unwrap();
                self.swarm.add_external_address(addr);
            }
            Command::Bootstrap => {
                self.bootstrap();
            }
            Command::GossipMessage { message } => {
                let res = self.gossip_message(message);

                if let Err(err) = res {
                    println!("Failed to gossip: {:?}", err);
                }
            }
            Command::GossipContactInfo { contact_info } => {
                self.gossip_contact_info(contact_info);
            }
            Command::DialMultiAddr { addr } => {
                let res = self.dial_multi_addr(addr);

                if let Err(err) = res {
                    println!("Failed to dial: {:?}", err);
                }
            }
            Command::GetState { sender } => {
                let mut state = self.state.clone();
                state.external_addrs = self.swarm.external_addresses().map(|a| a.clone()).collect();

                sender.send(state);
            }
        }
    }

    fn bootstrap(&mut self) {
        if self.state.bootstrap_addrs.len() == 0 {
            return;
        }

        let bootstrap_addrs = self.state.bootstrap_addrs.clone();

        for addr in bootstrap_addrs {
            self.dial_multi_addr(addr);
        }
    }

    fn peer_discovery_broadcast(&mut self) {
        println!("Broadcasting peer discovery");
        let peer_id_bytes = self.swarm.local_peer_id().to_bytes();
        let listener_addresses_bytes: Vec<Vec<u8>> =
            self.swarm.listeners().map(|la| la.to_vec()).collect();

        // Encode peer_id and listener_addresses the same way as JS
        let peer = super::gossip_node::Peer {
            public_key: peer_id_bytes.clone(), // TODO: this is likely wrong. JS does this over public key, not peer id
            addrs: listener_addresses_bytes,
        };

        let mut encoded_peer = Vec::new();
        peer.encode(&mut encoded_peer).unwrap();

        let topic = self.state.peer_discovery_topic.clone();

        let _ = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, encoded_peer);
    }

    fn gossip_message(&mut self, message: generated::Message) -> Result<(), HubError> {
        let gossip_message = generated::GossipMessage {
            topics: vec![self.state.primary_topic.to_string()],
            peer_id: self.swarm.local_peer_id().to_bytes(),
            version: 1,
            content: Some(generated::gossip_message::Content::Message(message)),
        };

        self.publish(gossip_message)
    }

    fn gossip_contact_info(
        &mut self,
        contact_info: generated::ContactInfoContent,
    ) -> Result<(), HubError> {
        let gossip_message = generated::GossipMessage {
            topics: vec![self.state.contact_info_topic.to_string()],
            peer_id: self.swarm.local_peer_id().to_bytes(),
            version: generated::GossipVersion::V11.into(),
            content: Some(generated::gossip_message::Content::ContactInfoContent(
                contact_info,
            )),
        };

        self.publish(gossip_message)
    }

    fn publish(&mut self, message: generated::GossipMessage) -> Result<(), HubError> {
        let encode_result = message.encode_to_vec();

        for topic_str in message.topics {
            let topic = IdentTopic::new(topic_str);
            let publish_result = self
                .swarm
                .behaviour_mut()
                .gossipsub
                .publish(topic, encode_result.clone());

            if let Err(err) = publish_result {
                return Err(HubError::BadRequest(
                    BadRequestType::Duplicate,
                    err.to_string(),
                ));
            }
        }

        Ok(())
    }

    fn dial_multi_addr(&mut self, multi_addr: Multiaddr) -> Result<(), HubError> {
        println!("dialing {:?}", multi_addr);
        let res = self.swarm.dial(multi_addr);

        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(HubError::Unavailable(
                UnavailableType::Generic,
                err.to_string(),
            )),
        }
    }
}
