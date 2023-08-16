use std::str::FromStr;

use libp2p::{
    core::upgrade,
    gossipsub::Message as GossipSubMessage,
    gossipsub::{self, Topic},
    gossipsub::{MessageAuthenticity, MessageId},
    identity, noise,
    swarm::{SwarmBuilder, SwarmEvent},
    tcp, Multiaddr, PeerId, Swarm, Transport,
};
use prost::Message;
use tokio::{runtime::Runtime, select};

use crate::{
    core::{
        errors::{BadRequestType, HubError, UnavailableType},
        protobufs::generated::{gossip_message, FarcasterNetwork, GossipMessage, GossipVersion},
    },
    teleport::AddrInfo,
};

use super::{gossip_behaviour::GossipBehaviour, utils::check_node_addrs};

const MULTI_ADDR_LOCAL_HOST: &str = "/ip4/127.0.0.1";
pub const MAX_MESSAGE_QUEUE_SIZE: usize = 100_000;

pub struct NodeOptions {
    keypair: Option<identity::Keypair>,
    ip_multi_addr: Option<String>,
    gossip_port: Option<u16>,
    pub allowed_peer_ids: Option<Vec<PeerId>>,
    pub denied_peer_ids: Option<Vec<PeerId>>,
    pub direct_peers: Option<Vec<AddrInfo>>,
}

pub(crate) struct GossipNode {
    network: FarcasterNetwork,
    pub(crate) swarm: Swarm<GossipBehaviour>,
}

impl GossipNode {
    async fn pubsub_peer_discovery(&mut self) -> Result<(), HubError> {
        let peer_discovery_topic_str =
            format!("_farcaster.{}.peer_discovery", self.network.as_str_name());

        let peer_discovery_topic = gossipsub::IdentTopic::new(peer_discovery_topic_str);

        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&peer_discovery_topic)
            .unwrap();

        Ok(())
    }

    async fn create_node(&mut self, options: NodeOptions) -> Result<(), HubError> {
        let local_key = options
            .keypair
            .unwrap_or(identity::Keypair::generate_ed25519());
        let local_peer_id = PeerId::from(local_key.public());

        let listen_ip_multi_addr = options
            .ip_multi_addr
            .unwrap_or(MULTI_ADDR_LOCAL_HOST.to_string());
        let listen_port = options.gossip_port.unwrap_or(0);
        let listen_multi_addr_str = format!("{}/tcp/{}", listen_ip_multi_addr, listen_port);
        let listen_multi_addr =
            Multiaddr::from_str(&listen_multi_addr_str).expect("invalid multiaddr");

        let check_result = check_node_addrs(listen_ip_multi_addr, listen_multi_addr_str);
        if check_result.is_err() {
            return Err(HubError::Unavailable(
                UnavailableType::Generic,
                check_result.unwrap_err().to_string(),
            ));
        }

        let primary_topic = self.primary_topic();
        let message_authenticity = MessageAuthenticity::Signed(local_key.clone());

        let gossipsub_config = libp2p::gossipsub::ConfigBuilder::default()
            .validation_mode(libp2p::gossipsub::ValidationMode::Strict)
            .message_id_fn(move |message: &GossipSubMessage| {
                get_message_id(&primary_topic, message)
            })
            .build()
            .expect("Valid config");

        let mut gossipsub: gossipsub::Behaviour =
            gossipsub::Behaviour::new(message_authenticity, gossipsub_config)
                .expect("Valid config");

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
            allowed_peers,
            blocked_peers,
        };

        let tcp_transport = libp2p::tcp::tokio::Transport::default()
            .upgrade(upgrade::Version::V1Lazy)
            .authenticate(
                noise::Config::new(&local_key).expect("signing libp2p-noise static keypair"),
            )
            .multiplex(libp2p_mplex::MplexConfig::new())
            .boxed();

        let mut swarm =
            SwarmBuilder::with_tokio_executor(tcp_transport, behaviour, local_peer_id).build();

        swarm.listen_on(listen_multi_addr).unwrap();

        self.swarm = swarm;

        Ok(())
    }

    fn primary_topic(&self) -> String {
        format!("f_network_{}_primary", self.network.as_str_name())
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
