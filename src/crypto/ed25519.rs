use ed25519_dalek::{
    Digest, Keypair, PublicKey, Sha512, Signature, SignatureError, PUBLIC_KEY_LENGTH,
    SECRET_KEY_LENGTH,
};

pub fn get_public_key(private_key: &[u8; SECRET_KEY_LENGTH]) -> [u8; 32] {
    let keypair =
        Keypair::from_bytes(private_key).expect("Failed to create a keypair from private key");

    let public_key: PublicKey = keypair.public;

    public_key.to_bytes()
}

pub fn sign_message(
    message: &[u8],
    private_key: &[u8; SECRET_KEY_LENGTH],
) -> Result<Signature, SignatureError> {
    let keypair =
        Keypair::from_bytes(private_key).expect("Failed to create a keypair from private key");

    let mut prehashed: Sha512 = Sha512::new();
    prehashed.update(message);

    keypair.sign_prehashed(prehashed, None)
}

pub fn verify_message_hash_signature(
    signature: &Signature,
    hash: Sha512,
    public_key_bytes: &[u8; PUBLIC_KEY_LENGTH],
) -> Result<(), SignatureError> {
    let public_key: PublicKey =
        PublicKey::from_bytes(public_key_bytes).expect("Invalid public_key_bytes");

    public_key.verify_prehashed(hash, None, signature)
}
