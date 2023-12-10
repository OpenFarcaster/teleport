use crate::p2p::{event_loop::Command, gossip_node::GossipNode};
use libp2p::{futures::channel::mpsc, Multiaddr, PeerId};
use teleport_common::protobufs::generated::*;
use teleport_storage::Store;

enum HubSubmitSource {
    Gossip,
    RPC,
    EthProvider,
    L2Provider,
    Sync,
    FNameRegistry,
}

#[derive(Debug, Clone)]
pub struct TestUser {
    fid: u64,
    mnemonic: String,
}

#[derive(Debug, Clone)]
pub struct AddrInfo {
    pub id: PeerId,
    pub addrs: Vec<Multiaddr>,
}

#[derive(Debug, Clone)]
pub struct HubOptions {
    pub network: FarcasterNetwork,
    pub peer_id: Option<PeerId>,
    pub bootstrap_addrs: Option<Vec<Multiaddr>>,
    pub allowed_peers: Option<Vec<PeerId>>,
    pub denied_peers: Option<Vec<PeerId>>,
    pub ip_multi_addr: Option<String>,
    pub rpc_server_host: Option<String>,
    pub anounce_ip: Option<String>,
    pub announce_server_name: Option<String>,
    pub gossip_port: Option<u16>,
    pub rpc_port: Option<u16>,
    pub rpc_auth: Option<String>,
    pub rpc_rate_limit: Option<u128>,
    pub rank_rpcs: Option<bool>,
    pub eth_rpc_url: Option<String>,
    pub eth_mainnet_rpc_url: Option<String>,
    pub fname_server_url: Option<String>,
    pub l2_rpc_url: Option<String>,
    pub id_registry_address: Option<String>,
    pub name_registry_address: Option<String>,
    pub l2_id_registry_address: Option<String>,
    pub l2_key_registry_address: Option<String>,
    pub l2_storage_registry_address: Option<String>,
    pub first_block: Option<u64>,
    pub chunk_size: Option<u64>,
    pub l2_first_block: Option<u64>,
    pub l2_chunk_size: Option<u64>,
    pub l2_chain_id: Option<u64>,
    pub l2_rent_expiry_override: Option<u64>,
    pub l2_resync_events: Option<bool>,
    pub eth_resync_events: Option<bool>,
    pub resync_name_events: Option<bool>,
    pub db_name: Option<String>,
    pub reset_db: Option<bool>,
    pub profile_sync: Option<bool>,
    pub rebuild_sync_trie: Option<bool>,
    pub commit_lock_timeout: u64,
    pub commit_lock_max_pending: u64,
    pub admin_server_enabled: Option<bool>,
    pub admin_server_host: Option<String>,
    pub test_users: Option<Vec<TestUser>>,
    pub local_ip_addrs_only: Option<bool>,
    pub prune_messages_job_cron: Option<String>,
    pub prune_events_job_cron: Option<String>,
    pub gossip_metrics_enabled: Option<bool>,
    pub direct_peers: Option<Vec<AddrInfo>>,
    pub hub_operator_fid: Option<u64>,
}

pub struct Hub {
    options: HubOptions,
    gossip_node: GossipNode,
    command_sender: mpsc::Sender<Command>,
    // TODO: rpc_server: Server,
    // TODO: admin_server
    rocks_db: Store,
    // TODO: Sync Engine
    // TODO: Job Schedulers
    // TODO: DB Engine
    // TODO: Chain Events
}

// impl Hub {
//     pub fn new(options: HubOptions) -> Self {
//         let gossip_node_opts = NodeOptions::new(
//             options.network,
//             None,
//             None,
//             None,
//             options.allowed_peers.clone(),
//             options.denied_peers.clone(),
//             options.direct_peers.clone(),
//         );
//         let (gossip_node, command_sender) = GossipNode::new(gossip_node_opts);
//         let rocks_db = RocksDB::new(options.db_name.clone());

//         Hub {
//             options,
//             gossip_node,
//             command_sender,
//             rocks_db,
//         }
//     }
// }
