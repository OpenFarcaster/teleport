// use ed25519_dalek::{
//     Signature, SignatureError, Signer, SigningKey, Verifier, VerifyingKey, PUBLIC_KEY_LENGTH,
//     SECRET_KEY_LENGTH,
// };

// pub fn get_public_key(private_key: &[u8; SECRET_KEY_LENGTH]) -> [u8; PUBLIC_KEY_LENGTH] {
//     let signing_key = SigningKey::from_bytes(private_key);
//     let verifying_key = signing_key.verifying_key();

//     verifying_key.to_bytes()
// }

// pub fn sign_message_hash(
//     hash: &[u8],
//     private_key: &[u8; SECRET_KEY_LENGTH],
// ) -> Result<Signature, SignatureError> {
//     let signing_key = SigningKey::from_bytes(private_key);

//     Ok(signing_key.sign(hash))
// }

// pub fn verify_message_hash_signature(
//     signature: &Signature,
//     hash: &[u8],
//     public_key_bytes: &[u8; PUBLIC_KEY_LENGTH],
// ) -> Result<(), SignatureError> {
//     let verifying_key = VerifyingKey::from_bytes(public_key_bytes).expect("Invalid public key");
//     verifying_key.verify(hash, signature)
// }
