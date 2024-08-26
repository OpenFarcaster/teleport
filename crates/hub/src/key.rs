use crate::key::ed25519::{Keypair, PublicKey, SecretKey};
use libp2p::identity::ed25519;
use teleport_common::config::Config;

pub struct Key {
    pubkey: PublicKey,
    privkey: SecretKey,
    id: Keypair,
}

impl Key {
    fn new(config: Config) -> Self {
        let secret_key_hex = config.farcaster_priv_key;
        let mut secret_key_bytes = hex::decode(secret_key_hex).expect("Invalid hex string");
        let privkey = ed25519::SecretKey::try_from_bytes(&mut secret_key_bytes).unwrap();
        let keypair = ed25519::Keypair::from(privkey);
        let pubkey = keypair.public();

        let id_keypair =
            libp2p::identity::Keypair::ed25519_from_bytes(&mut secret_key_bytes).unwrap();

        log::info!("Public Key: {}", hex::encode(pubkey.to_bytes()));

        Self {
            pubkey,
            privkey,
            id: id_keypair,
        }
    }
}
