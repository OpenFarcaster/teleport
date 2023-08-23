use crate::core::time::get_farcaster_time;
use crate::core::{
    crypto::blake3::blake3_20,
    protobufs::{
        self,
        generated::{HashScheme, MessageData, SignatureScheme},
    },
};
use std::io::Write;
use std::{fs::File, str::FromStr};

use libp2p::{identity::ed25519, Multiaddr};
use network::p2p::gossip_node::{Command, NodeOptions};
use prost::Message;

mod cli;
mod core;
mod network;
mod rpc;
mod storage;
mod teleport;

#[tokio::main]
async fn main() {
    env_logger::init();

    let priv_key_hex = std::env::var("FARCASTER_PRIV_KEY").unwrap();
    let mut secret_key_bytes = hex::decode(priv_key_hex).expect("Invalid hex string");
    let secret_key = ed25519::SecretKey::try_from_bytes(&mut secret_key_bytes).unwrap();
    let keypair = ed25519::Keypair::from(secret_key);
    let pub_key = keypair.public();

    let bootstrap_nodes = vec![
        "/ip4/44.196.72.233/tcp/2282",
        "/ip4/3.223.235.209/tcp/2282",
        "/ip4/52.20.72.19/tcp/2282",
    ];

    let id_keypair = libp2p::identity::Keypair::ed25519_from_bytes(&mut secret_key_bytes).unwrap();

    let node_options = NodeOptions::new(
        core::protobufs::generated::FarcasterNetwork::Mainnet,
        Some(id_keypair),
        None,
        None,
        None,
        None,
        None,
    );

    let (mut gossip_node, mut command_sender) =
        network::p2p::gossip_node::GossipNode::new(node_options);

    let mut bootstrap_addrs = Vec::new();
    for node in bootstrap_nodes {
        let addr = Multiaddr::from_str(node);
        bootstrap_addrs.push(addr.unwrap());
    }

    let cast_add_body = protobufs::generated::CastAddBody {
        embeds_deprecated: vec![],
        mentions: vec![],
        text: "This message is from Teleport".to_string(),
        mentions_positions: vec![],
        embeds: vec![],
        parent: None,
    };

    // print cast_add_body as JSON
    println!(
        "cast add body {:#?}",
        serde_json::to_string(&cast_add_body).unwrap()
    );

    println!(
        "cast add body hex {:#?}",
        hex::encode(&cast_add_body.encode_to_vec())
    );

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

    println!("data bytes {:#?}", hex::encode(&data_bytes));

    let blake_hash = blake3_20(&data_bytes);

    println!("blake hash {:#?}", blake_hash);

    let signature = keypair.sign(&blake_hash);

    println!("signature {:#?}", signature);

    let message = protobufs::generated::Message {
        data: Some(msg_data),
        hash: blake_hash.to_vec(),
        hash_scheme: HashScheme::Blake3 as i32,
        signature,
        signature_scheme: SignatureScheme::Ed25519 as i32,
        signer: pub_key.to_bytes().to_vec(),
    };

    println!("message {:#?}", message);

    let mut buf = Vec::new();
    let _ = prost::Message::encode(&message, &mut buf);

    let hex_msg = hex::encode(buf);
    println!("Encoded message: {}", hex_msg);

    tokio::spawn(async move {
        // Sleep for 5 seconds
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        println!("Sending message");
        let res = command_sender.start_send(Command::GossipMessage(message));

        if let Err(err) = res {
            println!("Failed to send message: {:?}", err);
        }

        println!("broadcast msg successfully")
    });

    gossip_node.start(bootstrap_addrs).await;
}
