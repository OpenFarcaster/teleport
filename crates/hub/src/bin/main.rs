use ethers::prelude::{Http, Provider};
use libp2p::PeerId;
use libp2p::{identity::ed25519, Multiaddr};
use log;
use p2p::gossip_node::NodeOptions;
use prost::Message;
use serde::Deserialize;
use std::fs::{self, canonicalize};
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use teleport_common::config::Config;
use teleport_common::peer_id::{create_ed25519_peer_id, write_peer_id};
use teleport_common::protobufs::generated::hub_service_server::HubServiceServer;
use teleport_common::protobufs::generated::{FarcasterNetwork, PeerIdProto};
use teleport_eth::indexer::Indexer;
use teleport_hub::{hub, p2p};
use teleport_rpc::server::HubServer;
use teleport_storage::Store;
use tonic::transport::Server;

const PEER_ID_FILENAME: &str = "id.protobuf";
const DEFAULT_PEER_ID_FILENAME: &str = "default_id.protobuf";

#[tokio::main]
async fn main() {
    // Load env vars from .env file

    env_logger::init();

    let config: Config = Config::new();

    let provider = Arc::new(Provider::<Http>::try_from(&config.optimism_l2_rpc_url).unwrap());

    let store = Store::new(config.clone()).await;
    store.migrate().await;

    let mut indexer = Indexer::new(config.clone(), store.clone(), provider).await.unwrap();

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

    let secret_key_hex = &config.farcaster_priv_key;
    let mut secret_key_bytes = hex::decode(secret_key_hex).expect("Invalid hex string");
    let secret_key = ed25519::SecretKey::try_from_bytes(&mut secret_key_bytes).unwrap();
    let keypair = ed25519::Keypair::from(secret_key);
    let pub_key = keypair.public();

    log::info!("Public Key: {}", hex::encode(pub_key.to_bytes()));

    let bootstrap_nodes: Vec<Multiaddr> = config
        .clone()
        .bootstrap_addrs
        .iter()
        .map(|addr| Multiaddr::from_str(addr).unwrap())
        .collect();

    let id_keypair = libp2p::identity::Keypair::ed25519_from_bytes(&mut secret_key_bytes).unwrap();

    let node_options =
        NodeOptions::new(teleport_common::protobufs::generated::FarcasterNetwork::Mainnet)
            .with_keypair(id_keypair)
            .with_bootstrap_addrs(bootstrap_nodes);

    let mut gossip_node = p2p::gossip_node::GossipNode::new(node_options, store.clone());

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

    let hub_options = hub::HubOptions {
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
