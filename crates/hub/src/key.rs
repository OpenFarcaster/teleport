use libp2p::identity::{
    ed25519::{self, SecretKey},
    Keypair, PublicKey,
};
use super::config::Config;

pub struct Key {
    pub pubkey: PublicKey,
    pub privkey: SecretKey,
    pub id: Keypair,
}

impl Key {
    pub fn new(config: Config) -> Self {
        let secret_key_hex = config.farcaster_priv_key;
        let mut secret_key_bytes = hex::decode(secret_key_hex).expect("Invalid hex string");
        let privkey = SecretKey::try_from_bytes(&mut secret_key_bytes).unwrap();
        let keypair = ed25519::Keypair::from(privkey.clone());
        let pubkey = keypair.public();

        let id_keypair = Keypair::ed25519_from_bytes(&mut secret_key_bytes).unwrap();

        log::info!("Public Key: {}", hex::encode(pubkey.to_bytes()));

        Self {
            pubkey: pubkey.into(),
            privkey,
            id: id_keypair,
        }
    }
}


//Key::from_mnemonic()
