use crate::common::protobufs::generated::hub_service_server::HubServiceServer;
use crate::common::time::get_farcaster_time;
use crate::common::{
    crypto::blake3::blake3_20,
    protobufs::{
        self,
        generated::{HashScheme, SignatureScheme},
    },
};
use crate::rpc::server::HubServer;
use std::str::FromStr;

use clap::Parser;
use libp2p::{identity::ed25519, Multiaddr};
use network::p2p::gossip_node::NodeOptions;
use prost::Message;
use tonic::transport::Server;

mod cli;
mod common;
mod hub;
mod network;
mod rpc;
mod storage;

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = cli::Cli::parse();

    match args.command {
        cli::Commands::Start(start_args) => {
            println!("Starting hub with args {:#?}", start_args);
        }
        cli::Commands::Identity(identity_args) => match identity_args.command {
            cli::IdentityCommands::Create(create_args) => {
                println!("Create args {:#?}", create_args);
            }
            cli::IdentityCommands::Verify(verify_args) => {
                println!("Verify args {:#?}", verify_args);
            }
        },
        cli::Commands::Status(status_args) => {
            println!("Status args {:#?}", status_args);
        }
        cli::Commands::Profile(profile_args) => match profile_args.command {
            cli::ProfileCommands::Gossip(profile_gossip_args) => todo!(),
            cli::ProfileCommands::Rpc(profile_rpc_args) => todo!(),
            cli::ProfileCommands::Storage(profile_storage_args) => todo!(),
        },
        cli::Commands::Reset(reset_args) => match reset_args.command {
            cli::ResetCommands::Events(reset_events_args) => todo!(),
            cli::ResetCommands::Full(reset_full_args) => todo!(),
        },
        cli::Commands::Console(console_args) => {
            println!("Console args {:#?}", console_args);
        }
    }

    let priv_key_hex = std::env::var("FARCASTER_PRIV_KEY").unwrap();
    let mut secret_key_bytes = hex::decode(priv_key_hex).expect("Invalid hex string");
    let secret_key = ed25519::SecretKey::try_from_bytes(&mut secret_key_bytes).unwrap();
    let keypair = ed25519::Keypair::from(secret_key);
    let pub_key = keypair.public();

    let bootstrap_nodes = vec![
        Multiaddr::from_str("/ip4/44.196.72.233/tcp/2282").unwrap(),
        Multiaddr::from_str("/ip4/3.223.235.209/tcp/2282").unwrap(),
        Multiaddr::from_str("/ip4/52.20.72.19/tcp/2282").unwrap(),
    ];

    let id_keypair = libp2p::identity::Keypair::ed25519_from_bytes(&mut secret_key_bytes).unwrap();

    let node_options = NodeOptions::new(common::protobufs::generated::FarcasterNetwork::Mainnet)
        .with_keypair(id_keypair)
        .with_bootstrap_addrs(bootstrap_nodes);

    let mut gossip_node = network::p2p::gossip_node::GossipNode::new(node_options);

    let cast_add_body = protobufs::generated::CastAddBody {
        embeds_deprecated: vec![],
        mentions: vec![],
        text: "This message is from Teleport - test 2".to_string(),
        mentions_positions: vec![],
        embeds: vec![],
        parent: None,
    };

    let msg_body = protobufs::generated::message_data::Body::CastAddBody(cast_add_body);

    // println!("msg body {:#?}", serde_json::to_string(&msg_body).unwrap());

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

    let signature = keypair.sign(&blake_hash);

    let message = protobufs::generated::Message {
        data: Some(msg_data),
        hash: blake_hash.to_vec(),
        hash_scheme: HashScheme::Blake3 as i32,
        signature,
        signature_scheme: SignatureScheme::Ed25519 as i32,
        signer: pub_key.to_bytes().to_vec(),
    };

    // println!("message {:#?}", message);

    let mut buf = Vec::new();
    let _ = prost::Message::encode(&message, &mut buf);

    let hex_msg = hex::encode(buf);

    gossip_node.start().await;

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let state = gossip_node.get_state().await.unwrap();
    let gossip_addr_info = protobufs::generated::GossipAddressInfo {
        address: state.external_addrs[0].to_string(),
        family: 4,
        port: 2282,
        dns_name: "".to_string(),
    };

    // tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    // println!("Sending message");

    // let res = gossip_node.gossip_message(message);

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
