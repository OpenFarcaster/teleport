use crate::{
    key::Key,
    p2p::{
        event_loop::Command,
        gossip_node::{GossipNode, NodeOptions},
    },
};
use libp2p::{futures::channel::mpsc, Multiaddr, PeerId};
use prost::Message;
use teleport_common::protobufs::generated::*;
use teleport_storage::Store;

use ethers::{prelude::Provider, providers::Http};
use log;
use std::fs::{self, canonicalize};
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use teleport_cli;
use teleport_common::config::Config;
use teleport_common::peer_id::{create_ed25519_peer_id, write_peer_id};
use teleport_common::protobufs::generated::hub_service_server::HubServiceServer;
use teleport_common::protobufs::generated::{FarcasterNetwork, PeerIdProto};
use teleport_eth::indexer::Indexer;
use teleport_rpc::server::HubServer;
use tonic::transport::Server;

const PEER_ID_FILENAME: &str = "id.protobuf";
const DEFAULT_PEER_ID_FILENAME: &str = "default_id.protobuf";

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
    //
}

impl Hub {
    pub fn new(options: HubOptions) -> Self {
        todo!()
    }

    pub async fn start() {
        // Load env vars from .env file

        env_logger::init();

        let config: Config = Config::new();

        let provider = Arc::new(Provider::<Http>::try_from(&config.optimism_l2_rpc_url).unwrap());

        let store = Store::new(config.clone()).await;
        store.migrate().await.expect("failed to migrate files");

        let mut indexer = Indexer::new(config.clone(), store.clone(), provider)
            .await
            .expect("failed to load indexer");

        let keys = Key::new(config.clone());

        // Fill in all registration events
        // syncs upto `latest_block_number`.
        // block until back filling is complete.
        let latest_block_num = indexer.get_latest_block().await.unwrap();
        let start_block_num = indexer.get_start_block().await;
        indexer
            .sync(start_block_num, latest_block_num)
            .await
            .unwrap();

        // Subscribe to new events asynchronously
        let subscribe_task = indexer.subscribe(latest_block_num + 1, config.indexer_interval);

        let bootstrap_nodes: Vec<Multiaddr> = config
            .clone()
            .bootstrap_addrs
            .iter()
            .map(|addr| Multiaddr::from_str(addr).unwrap())
            .collect();

        let node_options =
            NodeOptions::new(teleport_common::protobufs::generated::FarcasterNetwork::Mainnet)
                .with_keypair(keys.id)
                .with_bootstrap_addrs(bootstrap_nodes);

        let mut gossip_node = GossipNode::new(node_options, store.clone());

        gossip_node.start().await.unwrap();

        // tokio::time::sleep(std::time::Duration::from_secs(0)).await;

        let _state = gossip_node.get_state().await.unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(150)).await;

        let addr = "[::1]:2883".parse().unwrap();
        println!("gRPC Server Listening on {}", addr);
        let server = HubServer::default();

        let svc = HubServiceServer::new(server);

        let shutdown = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install ctrl+c handler");
        };

        Server::builder()
            .add_service(svc)
            .serve_with_shutdown(addr, shutdown)
            .await
            .unwrap();

        subscribe_task.await.unwrap();
    }
}

fn start(args: teleport_cli::start::StartCommand) {
    log::info!("Teleport Starting...");

    let peer_id: PeerId;

    // TODO: Read peerid from files
    if args.teleport_options.id.is_some() {
    } else if std::env::var("IDENTITY_B64").is_ok() {
    } else {
    }

    peer_id = PeerId::random();

    let _rpc_auth: String;
    if args.debugging_options.rpc_auth.is_some() {
    } else if std::env::var("RPC_AUTH").is_ok() {
    } else {
    }

    let _rpc_rate_limit: i32;
    if args.networking_options.rpc_rate_limit.is_some() {
    } else {
    }

    // TODO: Reset DB Check

    // TODO: Metrics

    let hub_options = HubOptions {
        network: FarcasterNetwork::from_str_name(
            args.teleport_options
                .network
                .expect("Missing network name")
                .as_str(),
        )
        .expect("Invalid network name"),
        peer_id: Some(peer_id),
        bootstrap_addrs: None,
        allowed_peers: None,
        denied_peers: None,
        ip_multi_addr: None,
        rpc_server_host: None,
        anounce_ip: None,
        announce_server_name: None,
        gossip_port: None,
        rpc_port: None,
        rpc_auth: None,
        rpc_rate_limit: None,
        rank_rpcs: None,
        eth_rpc_url: None,
        eth_mainnet_rpc_url: None,
        fname_server_url: None,
        l2_rpc_url: None,
        id_registry_address: None,
        name_registry_address: None,
        l2_id_registry_address: None,
        l2_key_registry_address: None,
        l2_storage_registry_address: None,
        first_block: None,
        chunk_size: None,
        l2_first_block: None,
        l2_chunk_size: None,
        l2_chain_id: None,
        l2_rent_expiry_override: None,
        l2_resync_events: None,
        eth_resync_events: None,
        resync_name_events: None,
        db_name: None,
        reset_db: None,
        profile_sync: None,
        rebuild_sync_trie: None,
        commit_lock_timeout: args.debugging_options.commit_lock_timeout,
        commit_lock_max_pending: args.debugging_options.commit_lock_max_pending,
        admin_server_enabled: None,
        admin_server_host: None,
        test_users: None,
        local_ip_addrs_only: None,
        prune_messages_job_cron: None,
        prune_events_job_cron: None,
        gossip_metrics_enabled: None,
        direct_peers: None,
        hub_operator_fid: None,
    };

    log::info!("Hub Options: {:#?}", hub_options);
}

fn identity_create(args: teleport_cli::identity::create::CreateIdentityCommand) {
    for i in 0..args.count {
        let peer_id_proto = create_ed25519_peer_id(false);

        if i == 0 {
            let path_buf = PathBuf::from(format!("{}/{}", args.output, DEFAULT_PEER_ID_FILENAME));
            let canonical_path = canonicalize(path_buf).unwrap();
            let filepath = canonical_path.to_str().unwrap();
            write_peer_id(&peer_id_proto, filepath)
        }

        let peer_id = PeerId::from(&peer_id_proto);
        let path_buf = PathBuf::from(format!(
            "{}/{}_{}",
            args.output,
            peer_id.to_base58(),
            PEER_ID_FILENAME
        ));
        let canonical_path = canonicalize(path_buf).unwrap();
        let filepath = canonical_path.to_str().unwrap();

        write_peer_id(&peer_id_proto, filepath);
    }

    exit(0);
}

fn identity_verify(args: teleport_cli::identity::verify::VerifyIdentityCommand) {
    println!("Verify args {:#?}", args);

    let filepath = canonicalize(PathBuf::from(args.id.as_str())).unwrap();
    let contents = fs::read(filepath).unwrap();

    let peer_id_proto = PeerIdProto::decode(contents.as_slice()).unwrap();
    let peer_id = PeerId::from(&peer_id_proto);

    log::info!(
        "Successfully read peer_id: {} from {}",
        peer_id.to_base58(),
        args.id
    );

    exit(0);
}
