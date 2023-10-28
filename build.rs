extern crate prost_build;
use std::path::PathBuf;

fn main() {
    let src = PathBuf::from("protobufs");

    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(
            &[
                src.join("gossip.proto"),
                src.join("hub_event.proto"),
                src.join("hub_state.proto"),
                src.join("job.proto"),
                src.join("message.proto"),
                src.join("onchain_event.proto"),
                src.join("request_response.proto"),
                src.join("rpc.proto"),
                src.join("sync_trie.proto"),
                src.join("username_proof.proto"),
                // Not a Hub schema, but we need it to serialize PeerIds the same way as Hubble
                // This is something part of js-libp2p that rust-libp2p doesn't have
                src.join("peer_id.proto"),
            ],
            &["protobufs"],
        )
        .unwrap();
}
