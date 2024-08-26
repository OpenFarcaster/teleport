// use ed25519_dalek::{Signature, SignatureError, SECRET_KEY_LENGTH};

// use crate::core::{crypto::ed25519, protobufs::generated};

// use super::signer::Signer;

// pub struct Ed25519Signer {
//     private_key: [u8; SECRET_KEY_LENGTH],
//     pub scheme: generated::SignatureScheme,
// }

// impl Ed25519Signer {
//     pub fn new(private_key: [u8; SECRET_KEY_LENGTH]) -> Ed25519Signer {
//         Ed25519Signer {
//             private_key,
//             scheme: generated::SignatureScheme::Ed25519,
//         }
//     }
// }

// impl Signer for Ed25519Signer {
//     fn get_signer_key(&self) -> [u8; 32] {
//         ed25519::get_public_key(&self.private_key)
//     }

//     fn sign_message(&self, message: &[u8]) -> Result<Signature, SignatureError> {
//         ed25519::sign_message_hash(message, &self.private_key)
//     }
// }
