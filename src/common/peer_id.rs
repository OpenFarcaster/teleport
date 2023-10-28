use std::{fs, path::Path};

use libp2p::{
    identity::{ed25519::Keypair, PublicKey},
    PeerId,
};
use prost::Message;

use super::protobufs::generated::PeerIdProto;

impl From<&PeerIdProto> for PeerId {
    fn from(peer_id: &PeerIdProto) -> Self {
        PeerId::from_bytes(&peer_id.id.as_ref().unwrap()).unwrap()
    }
}

impl From<PeerId> for PeerIdProto {
    fn from(peer_id: PeerId) -> Self {
        PeerIdProto {
            id: Some(peer_id.to_bytes()),
            pub_key: None,
            priv_key: None,
        }
    }
}

pub fn create_ed25519_peer_id(exclude_priv_key: bool) -> PeerIdProto {
    let keypair = Keypair::generate();
    let pub_key = PublicKey::from(keypair.public());
    let peer_id = PeerId::from_public_key(&pub_key);

    PeerIdProto {
        id: Some(peer_id.to_bytes()),
        pub_key: Some(keypair.public().to_bytes().to_vec()),
        priv_key: if exclude_priv_key {
            None
        } else {
            Some(keypair.secret().as_ref().to_vec())
        },
    }
}

pub fn write_peer_id(peer_id: &PeerIdProto, filepath: &str) {
    let path = Path::new(filepath);
    let prefix = path.parent().unwrap();

    fs::create_dir_all(prefix);

    let bytes = peer_id.encode_to_vec();
    fs::write(path, bytes);
}
