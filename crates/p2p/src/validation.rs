use crate::errors::HubError;
use chrono::{DateTime, Utc};
use prost::Message;
use teleport_crypto::{blake3, ed25519};
use teleport_protobuf::protobufs::generated;

pub struct Validator<'a> {
    message: &'a generated::Message,
}

impl<'a> Validator<'a> {
    pub fn new(message: &'a generated::Message) -> Self {
        Self { message }
    }

    fn validate_msg_body(&self) -> Result<(), HubError> {
        let msg_data = self.message.data.as_ref().unwrap();
        let body = msg_data.body.as_ref().unwrap();
        match body {
            generated::message_data::Body::CastAddBody(cast) => {
                println!("Cast Add: {:?}", cast);
            }
            generated::message_data::Body::CastRemoveBody(cast) => {
                println!("Cast Remove: {:?}", cast);
            }
            generated::message_data::Body::VerificationRemoveBody(verification) => {
                println!("Verification Remove: {:?}", verification);
            }
            generated::message_data::Body::VerificationAddAddressBody(verification) => {
                println!("Verification Add Eth Address: {:?}", verification);
            }
            generated::message_data::Body::UsernameProofBody(proof) => {
                println!("Username Proof: {:?}", proof);
            }
            generated::message_data::Body::UserDataBody(user_data) => {
                println!("User Data: {:?}", user_data);
            }
            generated::message_data::Body::LinkBody(link) => {
                println!("Link: {:?}", link);
            }
            generated::message_data::Body::ReactionBody(reaction) => {
                println!("Reaction: {:?}", reaction);
            }
            generated::message_data::Body::FrameActionBody(frame_action) => {
                println!("Frame Action: {:?}", frame_action);
            }
        }
        Ok(())
    }

    fn validate_signature(&self) -> Result<(), HubError> {
        let signature_scheme = generated::SignatureScheme::from_i32(self.message.signature_scheme)
            .ok_or_else(|| {
                HubError::Unknown(format!(
                    "Unknown signature scheme: {:?}",
                    self.message.signature_scheme
                ))
            })?;

        match signature_scheme {
            generated::SignatureScheme::Eip712 => {
                // todo: validate EIP712 signatures
                todo!();
            }
            generated::SignatureScheme::Ed25519 => {
                let mut pub_key = [0; 32];
                pub_key.copy_from_slice(&self.message.signer.as_slice()[0..32]);
                let mut signature = [0; 64];
                signature.copy_from_slice(&self.message.signature.as_slice()[0..64]);
                match ed25519::verify_message_hash_signature(
                    &signature,
                    &self.message.hash,
                    &pub_key,
                ) {
                    Ok(_) => {
                        return Ok(());
                    }
                    Err(err) => {
                        return Err(HubError::Unknown(err.to_string()));
                    }
                }
            }
            _ => {
                return Err(HubError::Unknown(format!(
                    "Unknown signature scheme: {:?}",
                    self.message.signature_scheme
                )));
            }
        }
    }

    fn validate_hash(&self) -> Result<(), HubError> {
        if self.message.hash_scheme != generated::HashScheme::Blake3 as i32 {
            return Err(HubError::Unknown(format!(
                "Unknown hash scheme: {:?}",
                self.message.hash_scheme
            )));
        }
        let msg_hash;
        if self.message.data_bytes.is_none() {
            let data = self.message.data.as_ref().unwrap();
            let mut encoded_message_data = Vec::new();
            data.encode(&mut encoded_message_data).unwrap();
            msg_hash = blake3::blake3_20(&encoded_message_data);
        } else {
            msg_hash = blake3::blake3_20(self.message.data_bytes.as_ref().unwrap());
        }

        if self.message.hash != msg_hash {
            return Err(HubError::Unknown(format!(
                "Invalid message hash: {:?}",
                self.message.hash
            )));
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<(), HubError> {
        self.validate_hash()?;
        // todo: check if the signer belongs to the user?
        self.validate_signature()?;
        self.validate_timestamp()?;
        self.validate_msg_body()?;
        Ok(())
    }

    fn validate_timestamp(&self) -> Result<(), HubError> {
        let timestamp = self.message.data.as_ref().unwrap().timestamp as i64;
        let msg_datetime = DateTime::from_timestamp(timestamp, 0).unwrap();
        if (msg_datetime - Utc::now()).num_seconds() > 600 {
            return Err(HubError::Unknown(format!(
                "Invalid timestamp: {:?}",
                msg_datetime
            )));
        }
        Ok(())
    }
}
