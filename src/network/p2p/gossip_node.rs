use std::{net::TcpListener, str::FromStr, time::Duration};

use libp2p::{
    core::upgrade,
    futures::{
        channel::{mpsc, oneshot},
        prelude::*,
    },
    gossipsub::{self, IdentTopic, Message as GossipSubMessage, MessageAuthenticity, MessageId},
    identify, identity, noise, ping,
    swarm::{derive_prelude::Either, SwarmBuilder, SwarmEvent},
    Multiaddr, PeerId, Swarm, Transport,
};
use prost::Message;
use tokio::time;
use void::Void;

use crate::{
    core::{
        errors::{BadRequestType, HubError, UnavailableType},
        protobufs::generated::{
            self,
            gossip_message::{self, Content},
            ContactInfoContent, FarcasterNetwork, GossipMessage, GossipVersion,
        },
    },
    teleport::AddrInfo,
};

use super::gossip_behaviour::{GossipBehaviour, GossipBehaviourEvent};

const MULTI_ADDR_LOCAL_HOST: &str = "/ip4/0.0.0.0";
const MAX_MESSAGE_QUEUE_SIZE: usize = 100_000;

type GossipNodeSwarmEvent = SwarmEvent<
    GossipBehaviourEvent,
    Either<Either<Either<Either<Void, std::io::Error>, Void>, Void>, Void>,
>;

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

pub enum Command {}

impl NodeOptions {
    pub fn new(
        network: FarcasterNetwork,
        keypair: Option<identity::Keypair>,
        ip_multi_addr: Option<String>,
        gossip_port: Option<u16>,
        allowed_peer_ids: Option<Vec<PeerId>>,
        denied_peer_ids: Option<Vec<PeerId>>,
        direct_peers: Option<Vec<AddrInfo>>,
    ) -> Self {
        NodeOptions {
            network,
            keypair,
            ip_multi_addr,
            gossip_port,
            allowed_peer_ids,
            denied_peer_ids,
            direct_peers,
        }
    }
}

pub(crate) struct GossipNode {
    network: FarcasterNetwork,
    swarm: Swarm<GossipBehaviour>,
    command_receiver: mpsc::Receiver<Command>,
}

impl GossipNode {
    pub fn new(options: NodeOptions) -> (Self, mpsc::Sender<Command>) {
        let swarm = create_node(options.clone()).expect("Failed to create node");

        let (command_sender, command_receiver) = mpsc::channel(0);

        (
            GossipNode {
                network: options.network,
                swarm,
                command_receiver,
            },
            command_sender,
        )
    }

    async fn handle_event(&mut self, event: GossipNodeSwarmEvent) {
        match event {
            SwarmEvent::Behaviour(behaviour_event) => {
                println!("Behaviour event: {:?}", behaviour_event)
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
                (connection_id, peer_id, error.to_string())
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

    pub async fn start(&mut self, bootstrap_addrs: Vec<Multiaddr>) -> Result<(), HubError> {
        let _ = self.bootstrap(bootstrap_addrs).await;
        let mut interval = time::interval(Duration::from_secs(10));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.peer_discovery_broadcast().await
                }
                _ = tokio::signal::ctrl_c() => {
                    break Ok(());
                }
                event = self.swarm.next() => self.handle_event(event.expect("fgsd")).await,
                command = self.command_receiver.next() => match command {
                    _ => {
                        todo!("Handle comamnd by user")
                    }
                }
            }
        }
    }

    async fn peer_discovery_broadcast(&mut self) {
        let peer_id_bytes = self.swarm.local_peer_id().to_bytes();
        let listener_addresses_bytes: Vec<Vec<u8>> =
            self.swarm.listeners().map(|la| la.to_vec()).collect();

        // Encode peer_id and listener_addresses the same way as JS
        let peer = Peer {
            public_key: peer_id_bytes.clone(), // TODO: this is likely wrong. JS does this over public key, not peer id
            addrs: listener_addresses_bytes,
        };

        let mut encoded_peer = Vec::new();
        peer.encode(&mut encoded_peer).unwrap();

        let topic = IdentTopic::new(self.peer_discovery_topic());

        let _ = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, encoded_peer);
    }

    pub async fn gossip_message(&mut self, message: generated::Message) -> Result<(), HubError> {
        let gossip_message = GossipMessage {
            topics: vec![self.primary_topic()],
            peer_id: self.swarm.local_peer_id().to_bytes(),
            version: GossipVersion::V11.into(),
            content: Some(Content::Message(message)),
        };

        self.publish(gossip_message).await
    }

    pub async fn gossip_contact_info(
        &mut self,
        contact_info: ContactInfoContent,
    ) -> Result<(), HubError> {
        let gossip_message = GossipMessage {
            topics: vec![self.contact_info_topic()],
            peer_id: self.swarm.local_peer_id().to_bytes(),
            version: GossipVersion::V11.into(),
            content: Some(Content::ContactInfoContent(contact_info)),
        };

        self.publish(gossip_message).await
    }

    pub async fn publish(&mut self, message: GossipMessage) -> Result<(), HubError> {
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

    pub async fn bootstrap(&mut self, bootstrap_addrs: Vec<Multiaddr>) -> Result<(), HubError> {
        if bootstrap_addrs.len() == 0 {
            return Ok(());
        }

        for addr in bootstrap_addrs {
            let dial_result = self.dial_multi_addr(addr).await;

            if let Err(err) = dial_result {
                return Err(HubError::Unavailable(
                    UnavailableType::Generic,
                    err.to_string(),
                ));
            }
        }

        Ok(())
    }

    async fn dial_multi_addr(&mut self, multi_addr: Multiaddr) -> Result<(), HubError> {
        let res = self.swarm.dial(multi_addr);

        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(HubError::Unavailable(
                UnavailableType::Generic,
                err.to_string(),
            )),
        }
    }

    async fn has_inbound_connections(&self) -> bool {
        self.swarm
            .network_info()
            .connection_counters()
            .num_established_incoming()
            > 0
    }

    fn primary_topic(&self) -> String {
        format!("f_network_{}_primary", self.network.as_str_name())
    }

    fn contact_info_topic(&self) -> String {
        format!("f_network_{}_contact_info", self.network.as_str_name())
    }

    fn peer_discovery_topic(&self) -> String {
        format!("f_network_{}_peer_discovery", self.network.as_str_name())
    }

    fn gossip_topics(&self) -> [String; 3] {
        [
            self.primary_topic(),
            self.contact_info_topic(),
            self.peer_discovery_topic(),
        ]
    }

    async fn all_peer_ids(&self) -> Vec<PeerId> {
        self.swarm
            .behaviour()
            .gossipsub
            .all_peers()
            .map(|peer| peer.0.clone())
            .collect()
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

    let listen_ip_multi_addr = options
        .ip_multi_addr
        .unwrap_or(MULTI_ADDR_LOCAL_HOST.to_string());
    let listen_port = options.gossip_port.unwrap_or({
        let tcp_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        tcp_listener.local_addr().unwrap().port()
    });
    let listen_multi_addr_str = format!("{}/tcp/{}", listen_ip_multi_addr, listen_port);
    println!("listen_multi_addr_str: {}", listen_multi_addr_str,);
    let listen_multi_addr = Multiaddr::from_str(&listen_multi_addr_str).unwrap();

    let primary_topic = format!("f_network_{}_primary", options.network.as_str_name());
    let message_authenticity = MessageAuthenticity::Signed(local_key.clone());

    let gossipsub_config = libp2p::gossipsub::ConfigBuilder::default()
        .validation_mode(libp2p::gossipsub::ValidationMode::Strict)
        .message_id_fn(move |message: &GossipSubMessage| get_message_id(&primary_topic, message))
        .build()
        .expect("Valid config");

    let mut gossipsub: gossipsub::Behaviour =
        gossipsub::Behaviour::new(message_authenticity, gossipsub_config).expect("Valid config");

    let identify: identify::Behaviour = identify::Behaviour::new(identify::Config::new(
        "farcaster/teleport".to_owned(),
        local_key.public(),
    ));

    let ping: ping::Behaviour = ping::Behaviour::new(ping::Config::new());

    let mut allowed_peers: libp2p::allow_block_list::Behaviour<
        libp2p::allow_block_list::AllowedPeers,
    > = libp2p::allow_block_list::Behaviour::default();

    let mut blocked_peers: libp2p::allow_block_list::Behaviour<
        libp2p::allow_block_list::BlockedPeers,
    > = libp2p::allow_block_list::Behaviour::default();

    if options.direct_peers.is_some() {
        for peer in options.direct_peers.unwrap() {
            gossipsub.add_explicit_peer(&peer.id);
        }
    }

    if let Some(allowed_peer_ids) = options.allowed_peer_ids {
        for peer_id in allowed_peer_ids {
            allowed_peers.allow_peer(peer_id)
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
        allowed_peers,
        blocked_peers,
    };

    let tcp_transport = libp2p::tcp::tokio::Transport::default()
        .upgrade(upgrade::Version::V1Lazy)
        .authenticate(noise::Config::new(&local_key).expect("signing libp2p-noise static keypair"))
        .multiplex(libp2p_mplex::MplexConfig::new())
        .boxed();

    let mut swarm =
        SwarmBuilder::with_tokio_executor(tcp_transport, behaviour, local_peer_id).build();

    swarm.listen_on(listen_multi_addr.clone()).unwrap();
    swarm.add_external_address(listen_multi_addr);

    Ok(swarm)
}
