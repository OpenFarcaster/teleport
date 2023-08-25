use std::{net::TcpListener, str::FromStr};

use libp2p::{
    core::upgrade,
    futures::{
        channel::{mpsc, oneshot},
        prelude::*,
    },
    gossipsub::{self, Message as GossipSubMessage, MessageAuthenticity, MessageId},
    identify, identity, noise, ping,
    swarm::{derive_prelude::Either, SwarmBuilder, SwarmEvent},
    Multiaddr, PeerId, Swarm, Transport,
};
use prost::Message;
use tokio::spawn;
use void::Void;

use crate::{
    core::{
        errors::{BadRequestType, HubError},
        protobufs::generated::{
            self,
            gossip_message::{self},
            ContactInfoContent, FarcasterNetwork, GossipMessage, GossipVersion,
        },
    },
    teleport::AddrInfo,
};

use super::{
    event_loop::{EventLoop, EventLoopState},
    gossip_behaviour::{GossipBehaviour, GossipBehaviourEvent},
};

const MULTI_ADDR_LOCAL_HOST: &str = "/ip4/127.0.0.1";
const MAX_MESSAGE_QUEUE_SIZE: usize = 100_000;

type GossipNodeSwarmEvent =
    SwarmEvent<GossipBehaviourEvent, Either<Either<Either<Void, std::io::Error>, Void>, Void>>;

#[derive(Message)]
pub struct Peer {
    #[prost(bytes, tag = "1")]
    pub public_key: Vec<u8>,
    #[prost(bytes, repeated, tag = "2")]
    pub addrs: Vec<Vec<u8>>,
}

#[derive(Clone)]
pub struct NodeOptions {
    network: FarcasterNetwork,
    keypair: Option<identity::Keypair>,
    ip_multi_addr: Option<String>,
    gossip_port: Option<u16>,
    allowed_peer_ids: Option<Vec<PeerId>>,
    denied_peer_ids: Option<Vec<PeerId>>,
    direct_peers: Option<Vec<AddrInfo>>,
}

impl NodeOptions {
    pub fn new(network: FarcasterNetwork) -> Self {
        NodeOptions {
            network,
            keypair: None,
            ip_multi_addr: None,
            gossip_port: None,
            allowed_peer_ids: None,
            denied_peer_ids: None,
            direct_peers: None,
        }
    }

    pub fn with_keypair(mut self, keypair: identity::Keypair) -> Self {
        self.keypair = Some(keypair);
        self
    }

    pub fn with_ip_multi_addr(mut self, ip_multi_addr: String) -> Self {
        self.ip_multi_addr = Some(ip_multi_addr);
        self
    }

    pub fn with_gossip_port(mut self, gossip_port: u16) -> Self {
        self.gossip_port = Some(gossip_port);
        self
    }

    pub fn with_allowed_peer_ids(mut self, allowed_peer_ids: Vec<PeerId>) -> Self {
        self.allowed_peer_ids = Some(allowed_peer_ids);
        self
    }

    pub fn with_denied_peer_ids(mut self, denied_peer_ids: Vec<PeerId>) -> Self {
        self.denied_peer_ids = Some(denied_peer_ids);
        self
    }

    pub fn with_direct_peers(mut self, direct_peers: Vec<AddrInfo>) -> Self {
        self.direct_peers = Some(direct_peers);
        self
    }
}

pub(crate) struct GossipNode {
    command_sender: mpsc::Sender<super::event_loop::Command>,
    event_loop: Option<EventLoop>,
}

impl GossipNode {
    pub fn new(options: NodeOptions) -> Self {
        let swarm = create_node(options.clone()).expect("Failed to create node");
        let (command_sender, command_receiver) = mpsc::channel(MAX_MESSAGE_QUEUE_SIZE);

        let event_loop = EventLoop::new(options.network, swarm, command_receiver);

        Self {
            command_sender,
            event_loop: Some(event_loop),
        }
    }

    pub async fn start(&mut self, bootstrap_addrs: Vec<Multiaddr>) -> Result<(), HubError> {
        let listen_ip_multi_addr = MULTI_ADDR_LOCAL_HOST.to_string();
        let listen_port = {
            let tcp_listener = TcpListener::bind("127.0.0.1:0").unwrap();
            tcp_listener.local_addr().unwrap().port()
        };
        let listen_multi_addr_str = format!("{}/tcp/{}", listen_ip_multi_addr, listen_port);
        println!("listen_multi_addr_str: {}", listen_multi_addr_str,);
        let listen_multi_addr = Multiaddr::from_str(&listen_multi_addr_str).unwrap();

        let mut event_loop = self
            .event_loop
            .take()
            .expect("Event loop is already running or uninitialized");

        spawn(async move {
            event_loop.run().await;
        });

        self.command_sender
            .start_send(super::event_loop::Command::StartListening {
                addr: listen_multi_addr,
            })
            .expect("Failed to send StartListening command");

        println!("bootstrapping");
        let _ = self.bootstrap(bootstrap_addrs);

        Ok(())
    }

    pub fn gossip_message(&mut self, message: generated::Message) -> Result<(), HubError> {
        self.command_sender
            .start_send(super::event_loop::Command::GossipMessage { message })
            .expect("Failed to send GossipMessage command");
        Ok(())
    }

    pub fn gossip_contact_info(
        &mut self,
        contact_info: ContactInfoContent,
    ) -> Result<(), HubError> {
        self.command_sender
            .start_send(super::event_loop::Command::GossipContactInfo { contact_info })
            .expect("Failed to send GossipContactInfo command");
        Ok(())
    }

    pub fn bootstrap(&mut self, bootstrap_addrs: Vec<Multiaddr>) -> Result<(), HubError> {
        println!("bootstrap_addr len is: {}", bootstrap_addrs.len());
        if bootstrap_addrs.len() == 0 {
            return Ok(());
        }

        for addr in bootstrap_addrs {
            println!("bootstrapping addr: {}", addr);
            self.command_sender
                .start_send(super::event_loop::Command::DialMultiAddr { addr })
                .expect("Failed to send DialMultiAddr command");

            // if let Err(err) = dial_result {
            //     return Err(HubError::Unavailable(
            //         UnavailableType::Generic,
            //         err.to_string(),
            //     ));
            // }
        }

        Ok(())
    }

    pub async fn get_state(&mut self) -> Result<EventLoopState, HubError> {
        let (sender, receiver) = oneshot::channel::<EventLoopState>();
        self.command_sender
            .start_send(super::event_loop::Command::GetState { sender })
            .expect("Failed to send GetState command");

        Ok(receiver.await.unwrap())
    }
}

pub fn decode_message(message: &[u8]) -> Result<GossipMessage, HubError> {
    let gossip_message = GossipMessage::decode(message)
        .map_err(|_| {
            HubError::BadRequest(BadRequestType::ParseFailure, "Invalid failure".to_owned())
        })
        .unwrap();

    let supported_versions = vec![GossipVersion::V1, GossipVersion::V11];
    if gossip_message.topics.len() == 0 || !supported_versions.contains(&gossip_message.version()) {
        return Err(HubError::BadRequest(
            BadRequestType::ParseFailure,
            "Invalid failure".to_owned(),
        ));
    }

    PeerId::from_bytes(&gossip_message.peer_id).map_err(|_| {
        HubError::BadRequest(BadRequestType::ParseFailure, "Invalid failure".to_owned())
    })?;

    Ok(gossip_message)
}

pub fn get_message_id(primary_topic: &str, message: &GossipSubMessage) -> MessageId {
    // topic is NOT hashed, regardless of it's data type based on our gossipsub config
    let message_topic = message.topic.as_str();

    if message_topic.contains(primary_topic) {
        let protocol_message = decode_message(&message.data);

        if let Ok(message) = protocol_message {
            if message.version() == GossipVersion::V11 {
                if let Some(content) = message.content {
                    match content {
                        gossip_message::Content::Message(message_content) => {
                            return MessageId::from(message_content.hash);
                        }
                        gossip_message::Content::IdRegistryEvent(event_content) => {
                            return MessageId::from(event_content.transaction_hash);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    default_message_id_fn(message)
}

// This function is not exported from libp2p::gossipsub::config
pub fn default_message_id_fn(message: &GossipSubMessage) -> MessageId {
    let mut source_string = if let Some(peer_id) = message.source.as_ref() {
        peer_id.to_base58()
    } else {
        PeerId::from_bytes(&[0, 1, 0])
            .expect("Valid peer id")
            .to_base58()
    };
    source_string.push_str(&message.sequence_number.unwrap_or_default().to_string());
    MessageId::from(source_string)
}

fn create_node(options: NodeOptions) -> Result<Swarm<GossipBehaviour>, HubError> {
    let local_key = options
        .keypair
        .unwrap_or(identity::Keypair::generate_ed25519());
    let local_peer_id = PeerId::from(local_key.public());

    let primary_topic = format!("f_network_{}_primary", options.network.as_str_name());
    let message_authenticity = MessageAuthenticity::Signed(local_key.clone());

    let gossipsub_config = libp2p::gossipsub::ConfigBuilder::default()
        .validation_mode(libp2p::gossipsub::ValidationMode::Strict)
        .message_id_fn(move |message: &GossipSubMessage| get_message_id(&primary_topic, message))
        .mesh_n(1)
        .mesh_n_low(1)
        .mesh_n_high(3)
        .mesh_outbound_min(0)
        .build()
        .expect("Valid config");

    let mut gossipsub: gossipsub::Behaviour =
        gossipsub::Behaviour::new(message_authenticity, gossipsub_config).expect("Valid config");

    let identify: identify::Behaviour = identify::Behaviour::new(identify::Config::new(
        "farcaster/teleport".to_owned(),
        local_key.public(),
    ));

    let ping: ping::Behaviour = ping::Behaviour::new(ping::Config::new());

    let mut blocked_peers: libp2p::allow_block_list::Behaviour<
        libp2p::allow_block_list::BlockedPeers,
    > = libp2p::allow_block_list::Behaviour::default();

    if options.direct_peers.is_some() {
        for peer in options.direct_peers.unwrap() {
            gossipsub.add_explicit_peer(&peer.id);
        }
    }

    if let Some(denied_peer_ids) = options.denied_peer_ids {
        for peer_id in denied_peer_ids {
            blocked_peers.block_peer(peer_id)
        }
    }

    let behaviour = GossipBehaviour {
        gossipsub,
        identify,
        ping,
        blocked_peers,
    };

    let tcp_transport = libp2p::tcp::tokio::Transport::default()
        .upgrade(upgrade::Version::V1Lazy)
        .authenticate(noise::Config::new(&local_key).expect("signing libp2p-noise static keypair"))
        .multiplex(libp2p_mplex::MplexConfig::new())
        .boxed();

    let mut swarm =
        SwarmBuilder::with_tokio_executor(tcp_transport, behaviour, local_peer_id).build();

    Ok(swarm)
}
