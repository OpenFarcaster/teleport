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
                src.join("name_registry_event.proto"),
                src.join("onchain_event.proto"),
                src.join("request_response.proto"),
                src.join("rpc.proto"),
                src.join("sync_trie.proto"),
                src.join("username_proof.proto"),
                src.join("peer_id.proto"),
            ],
            &["protobufs"],
        )
        .unwrap();
}
