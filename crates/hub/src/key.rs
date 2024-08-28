use libp2p::identity::{
    ed25519::{self, SecretKey},
    Keypair, PublicKey,
};
use teleport_common::config::Config;

pub struct Key {
    pub pubkey: PublicKey,
    pub privkey: SecretKey,
    pub id: Keypair,
}

//TODO: replace with using alloy, as all farcaster addresses are evm based
//TODO: create a test framework using alloy address generators
// https://alloy.rs/examples/wallets/mnemonic_signer.html
//TODO: keys should also be used for node networks
impl Key {
    pub fn new(config: Config) -> Self {
        let secret_key_hex = config.farcaster_priv_key;
        println!("config key: {:?}", secret_key_hex);
        let mut secret_key_bytes = hex::decode(secret_key_hex).expect("Invalid hex string");
        let privkey = SecretKey::try_from_bytes(&mut secret_key_bytes).unwrap();
        let keypair = ed25519::Keypair::from(privkey.clone());
        let pubkey = keypair.public();

        let id_keypair =
            libp2p::identity::Keypair::ed25519_from_bytes(&mut secret_key_bytes).unwrap();

        log::info!("Public Key: {}", hex::encode(pubkey.to_bytes()));

        Self {
            pubkey: pubkey.into(),
            privkey,
            id: id_keypair,
        }
    }
}
