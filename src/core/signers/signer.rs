use ed25519_dalek::{Signature, SignatureError};

pub trait Signer {
    fn get_signer_key(&self) -> [u8; 32];
    fn sign_message(&self, message: &[u8]) -> Result<Signature, SignatureError>;
}
