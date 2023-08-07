extern crate prost_build;
use std::fs;
use std::path::PathBuf;

fn main() {
    let mut prost_config = prost_build::Config::new();
    prost_config.protoc_arg("--experimental_allow_proto3_optional");

    let out = std::env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(out).join("generated");

    // Create the out_path dir if doesn't exist
    fs::create_dir_all(out_path.as_path()).expect("Could not create output dir");

    prost_config.out_dir(out_path);

    let src = PathBuf::from("protobufs");

    prost_config
        .compile_protos(
            &[
                src.join("gossip.proto"),
                src.join("hub_event.proto"),
                src.join("hub_state.proto"),
                src.join("id_registry_event.proto"),
                src.join("job.proto"),
                src.join("message.proto"),
                src.join("name_registry_event.proto"),
                src.join("onchain_event.proto"),
                src.join("request_response.proto"),
                src.join("rpc.proto"),
                src.join("sync_trie.proto"),
                src.join("username_proof.proto"),
            ],
            &["protobufs"],
        )
        .unwrap();
}
