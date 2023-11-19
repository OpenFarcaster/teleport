pub mod hub;
pub mod p2p;
pub mod storage;
pub mod sync;

use teleport_common::protobufs::generated::hub_service_server::HubServiceServer;
use teleport_common::protobufs::generated::{cast_add_body, CastId, FarcasterNetwork, PeerIdProto};
use teleport_common::time::get_farcaster_time;
use teleport_common::{
    crypto::blake3::blake3_20,
    protobufs::{
        self,
        generated::{HashScheme, SignatureScheme},
    },
};
//use crate::{HubOptions};
use std::fs::{self, canonicalize};
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use teleport_rpc::server::HubServer;

use libp2p::PeerId;
use libp2p::{identity::ed25519, Multiaddr};
use log::info;
use p2p::gossip_node::NodeOptions;
use prost::Message;
use teleport_common::peer_id::{create_ed25519_peer_id, write_peer_id};
use tonic::transport::Server;

const PEER_ID_FILENAME: &str = "id.protobuf";
const DEFAULT_PEER_ID_FILENAME: &str = "default_id.protobuf";

#[tokio::main]
async fn main() {
    env_logger::init();

    // run migrations

    // let args = cli::Cli::parse();

    // match args.command {
    //     cli::Commands::Start(start_args) => start(start_args),
    //     cli::Commands::Identity(identity_args) => match identity_args.command {
    //         cli::IdentityCommands::Create(create_args) => {
    //             identity_create(create_args);
    //         }
    //         cli::IdentityCommands::Verify(verify_args) => {
    //             identity_verify(verify_args);
    //         }
    //     },
    //     cli::Commands::Status(status_args) => {
    //         println!("Status args {:#?}", status_args);
    //     }
    //     cli::Commands::Profile(profile_args) => match profile_args.command {
    //         cli::ProfileCommands::Gossip(profile_gossip_args) => todo!(),
    //         cli::ProfileCommands::Rpc(profile_rpc_args) => todo!(),
    //         cli::ProfileCommands::Storage(profile_storage_args) => todo!(),
    //     },
    //     cli::Commands::Reset(reset_args) => match reset_args.command {
    //         cli::ResetCommands::Events(reset_events_args) => todo!(),
    //         cli::ResetCommands::Full(reset_full_args) => todo!(),
    //     },
    //     cli::Commands::Console(console_args) => {
    //         println!("Console args {:#?}", console_args);
    //     }
    // }

    let priv_key_hex = std::env::var("FARCASTER_PRIV_KEY").unwrap();
    let mut secret_key_bytes = hex::decode(priv_key_hex).expect("Invalid hex string");
    let secret_key = ed25519::SecretKey::try_from_bytes(&mut secret_key_bytes).unwrap();
    let keypair = ed25519::Keypair::from(secret_key);
    let pub_key = keypair.public();

    let bootstrap_nodes = vec![
        // Multiaddr::from_str("/ip4/3.17.4.160/tcp/2282").unwrap(),
        Multiaddr::from_str("/ip4/23.20.92.219/tcp/2282").unwrap(), // Multiaddr::from_str("/ip4/3.223.235.209/tcp/2282").unwrap(),
                                                                    // Multiaddr::from_str("/ip4/52.20.72.19/tcp/2282").unwrap(),
    ];

    let id_keypair = libp2p::identity::Keypair::ed25519_from_bytes(&mut secret_key_bytes).unwrap();

    let node_options =
        NodeOptions::new(teleport_common::protobufs::generated::FarcasterNetwork::Mainnet)
            .with_keypair(id_keypair)
            .with_bootstrap_addrs(bootstrap_nodes);

    let mut gossip_node = p2p::gossip_node::GossipNode::new(node_options);

    let cast_add_body = protobufs::generated::CastAddBody {
        embeds_deprecated: vec![],
        mentions: vec![],
        text: "Not an app, but this message is from Teleport :) Finally got it to work again!"
            .to_string(),
        mentions_positions: vec![],
        embeds: vec![],
        parent: Some(cast_add_body::Parent::ParentCastId(CastId {
            fid: 3,
            hash: hex::decode("08d1da2e082d25814bb6c3550f43d8925f5d0726").unwrap(),
        })),
    };

    let msg_body = protobufs::generated::message_data::Body::CastAddBody(cast_add_body);

    println!("msg body {:#?}", serde_json::to_string(&msg_body).unwrap());

    let fc_time = get_farcaster_time().unwrap();
    let msg_data = protobufs::generated::MessageData {
        r#type: 1,
        fid: 8113,
        timestamp: fc_time,
        network: 1,
        body: Some(msg_body),
    };

    let data_bytes = msg_data.encode_to_vec();

    let blake_hash = blake3_20(&data_bytes);

    println!("blake hash {:#?}", hex::encode(blake_hash));

    let signature = keypair.sign(&blake_hash);

    let message = protobufs::generated::Message {
        data: None,
        hash: blake_hash.to_vec(),
        hash_scheme: HashScheme::Blake3 as i32,
        signature,
        signature_scheme: SignatureScheme::Ed25519 as i32,
        signer: pub_key.to_bytes().to_vec(),
        data_bytes: Some(data_bytes),
    };

    println!("message {:#?}", message);

    let mut buf = Vec::new();
    let _ = prost::Message::encode(&message, &mut buf);

    let hex_msg = hex::encode(buf);

    println!("hex msg {:#?}", hex_msg);

    gossip_node.start().await;

    tokio::time::sleep(std::time::Duration::from_secs(0)).await;

    let state = gossip_node.get_state().await.unwrap();
    let gossip_addr_info = protobufs::generated::GossipAddressInfo {
        address: state.external_addrs[0].to_string(),
        family: 4,
        port: 2282,
        dns_name: "".to_string(),
    };

    tokio::time::sleep(std::time::Duration::from_secs(150)).await;

    println!("Sending message");

    let res = gossip_node.gossip_message(message);

    println!("res {:#?}", res);

    // println!("broadcast msg successfully");

    let addr = "[::1]:2883".parse().unwrap();
    println!("gRPC Server Listening on {}", addr);
    let server = HubServer::default();

    let svc = HubServiceServer::new(server);

    let shutdown = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl+c handler");
    };

    let _ = Server::builder()
        .add_service(svc)
        .serve_with_shutdown(addr, shutdown)
        .await;
}

fn start(args: teleport_cli::start::StartCommand) {
    info!("Teleport Starting...");

    // TODO: Handle reading a TOML config file

    let peer_id: PeerId;

    // TODO: Read peerid from files
    if args.teleport_options.id.is_some() {
    } else if std::env::var("IDENTITY_B64").is_ok() {
    } else {
    }

    peer_id = PeerId::random();

    let rpc_auth: String;
    if args.debugging_options.rpc_auth.is_some() {
    } else if std::env::var("RPC_AUTH").is_ok() {
    } else {
    }

    let rpc_rate_limit: i32;
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

    info!("Hub Options: {:#?}", hub_options);
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

    info!(
        "Successfully read peer_id: {} from {}",
        peer_id.to_base58(),
        args.id
    );

    exit(0);
}
