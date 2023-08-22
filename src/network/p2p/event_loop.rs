use super::gossip_behaviour::{GossipBehaviour, GossipBehaviourEvent};
use crate::core::errors::{BadRequestType, HubError};
use crate::core::protobufs::{self, generated};
use libp2p::futures::{FutureExt, StreamExt};
use libp2p::gossipsub::IdentTopic;
use libp2p::swarm::derive_prelude::Either;
use libp2p::swarm::SwarmEvent;
use libp2p::Multiaddr;
use libp2p::{futures::channel::mpsc, Swarm};
use prost::Message;
use void::Void;

type GossipNodeSwarmEvent =
    SwarmEvent<GossipBehaviourEvent, Either<Either<Either<Void, std::io::Error>, Void>, Void>>;

pub enum Command {
    StartListening { addr: Multiaddr },
    BroadcastPeerDiscovery,
    SubscribeTopic { topic: IdentTopic },
    GossipMessage(protobufs::generated::Message),
}

pub struct EventLoop {
    swarm: Swarm<GossipBehaviour>,
    command_receiver: mpsc::Receiver<Command>,
}

impl EventLoop {
    pub fn new(swarm: Swarm<GossipBehaviour>, command_receiver: mpsc::Receiver<Command>) -> Self {
        EventLoop {
            swarm,
            command_receiver,
        }
    }

    async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.next().fuse() => self.handle_event(event.expect("Failed to handle event")).await,
                command = self.command_receiver.next().fuse() => match command {
                    Some(Command::GossipMessage(message)) => {
                        self.gossip_message(message).await.unwrap();
                    }
                    _ => {
                    },
                }
            }
        }
    }

    async fn handle_event(&mut self, event: GossipNodeSwarmEvent) {
        match event {
            SwarmEvent::Behaviour(behaviour_event) => {
                self.handle_peer_discovery_pubsub_event(&behaviour_event)
                    .await;

                match behaviour_event {
                    GossipBehaviourEvent::Gossipsub(gossipsub_event) => match gossipsub_event {
                        Event::Message {
                            propagation_source,
                            message_id,
                            message,
                        } => {
                            let raw_data = message.data;
                            let decoded_message = GossipMessage::decode(raw_data.as_ref()).unwrap();

                            match decoded_message.content {
                                Some(Content::Message(data)) => {
                                    // println!("Message: {:?}", data);
                                }
                                _ => {
                                    println!("Unknown message");
                                }
                            }
                        }
                        Event::Subscribed { peer_id, topic } => {
                            println!("Subscribed: {:?}", (peer_id, topic))
                        }
                        Event::Unsubscribed { peer_id, topic } => {
                            println!("Unsubscribed: {:?}", (peer_id, topic))
                        }
                        Event::GossipsubNotSupported { peer_id } => {
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
            } => println!("Connection closed: {:?}", (peer_id, connection_id)),
            SwarmEvent::IncomingConnection {
                connection_id,
                local_addr,
                send_back_addr,
            } => println!("Incoming connection: {:?}", (connection_id, local_addr)),
            SwarmEvent::IncomingConnectionError {
                connection_id,
                local_addr,
                send_back_addr,
                error,
            } => println!(
                "Incoming connection error: {:?}",
                (connection_id, local_addr, error.to_string())
            ),
            SwarmEvent::OutgoingConnectionError {
                connection_id,
                peer_id,
                error,
            } => println!(
                "Outgoing connection error: {:?}",
                (connection_id, peer_id, error)
            ),
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => println!("New listen addr: {:?}", (listener_id, address)),
            SwarmEvent::ExpiredListenAddr {
                listener_id,
                address,
            } => println!("Expired listen addr: {:?}", (listener_id, address)),
            SwarmEvent::ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => println!("Listener closed: {:?}", (listener_id, addresses, reason)),
            SwarmEvent::ListenerError { listener_id, error } => {
                println!("Listener error: {:?}", (listener_id, error))
            }
            SwarmEvent::Dialing {
                peer_id,
                connection_id,
            } => println!("Dialing: {:?}", (peer_id, connection_id)),
        }
    }

    async fn handle_command(&mut self, command: Command) {
        match command {
            Command::StartListening { addr } => {
                self.swarm.listen_on(addr.clone()).unwrap();
                self.swarm.add_external_address(addr);
            }
            Command::BroadcastPeerDiscovery => todo!(),
            Command::SubscribeTopic { topic } => {
                self.swarm.behaviour_mut().gossipsub.subscribe(&topic);
            }
            Command::GossipMessage(_) => todo!(),
        }
    }

    async fn gossip_message(&mut self, message: generated::Message) -> Result<(), HubError> {
        let gossip_message = generated::GossipMessage {
            topics: vec![self.primary_topic()],
            peer_id: self.swarm.local_peer_id().to_bytes(),
            version: generated::GossipVersion::V11 as i32,
            content: Some(generated::gossip_message::Content::Message(message)),
        };

        self.publish(gossip_message).await
    }

    async fn gossip_contact_info(
        &mut self,
        contact_info: generated::ContactInfoContent,
    ) -> Result<(), HubError> {
        let gossip_message = generated::GossipMessage {
            topics: vec![self.contact_info_topic()],
            peer_id: self.swarm.local_peer_id().to_bytes(),
            version: generated::GossipVersion::V11.into(),
            content: Some(generated::gossip_message::Content::ContactInfoContent(
                contact_info,
            )),
        };

        self.publish(gossip_message).await
    }

    async fn publish(&mut self, message: generated::GossipMessage) -> Result<(), HubError> {
        let mut encoded_message = Vec::new();
        let encode_result = message.encode(&mut encoded_message);

        if let Err(err) = encode_result {
            return Err(HubError::BadRequest(
                BadRequestType::Generic,
                err.to_string(),
            ));
        }

        for topic_str in message.topics {
            let topic = IdentTopic::new(topic_str);
            let publish_result = self
                .swarm
                .behaviour_mut()
                .gossipsub
                .publish(topic, encoded_message.clone());

            if let Err(err) = publish_result {
                return Err(HubError::BadRequest(
                    BadRequestType::Duplicate,
                    err.to_string(),
                ));
            }
        }

        Ok(())
    }
}
