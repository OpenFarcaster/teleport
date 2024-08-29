use ed25519_dalek::{
    Signature, SignatureError, Signer, SigningKey, Verifier, VerifyingKey, PUBLIC_KEY_LENGTH,
    SECRET_KEY_LENGTH,
};

pub fn get_public_key(private_key: &[u8; SECRET_KEY_LENGTH]) -> [u8; PUBLIC_KEY_LENGTH] {
    let signing_key = SigningKey::from_bytes(private_key);
    let verifying_key = signing_key.verifying_key();

    verifying_key.to_bytes()
}

pub fn sign_message_hash(
    hash: &[u8],
    private_key: &[u8; SECRET_KEY_LENGTH],
) -> Result<Signature, SignatureError> {
    let signing_key = SigningKey::from_bytes(private_key);

    Ok(signing_key.sign(hash))
}

pub fn verify_message_hash_signature(
    signature: &[u8; 64],
    hash: &[u8],
    public_key_bytes: &[u8; PUBLIC_KEY_LENGTH],
) -> Result<(), SignatureError> {
    let sign = Signature::from_bytes(signature);
    let verifying_key = VerifyingKey::from_bytes(public_key_bytes).expect("Invalid public key");
    verifying_key.verify(hash, &sign)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_get_public_key() {
        let mut rng = rand::thread_rng();
        let mut private_key = [0u8; SECRET_KEY_LENGTH];
        rng.fill(&mut private_key);
        let public_key = get_public_key(&private_key);

        assert_eq!(public_key.len(), PUBLIC_KEY_LENGTH);
    }

    #[test]
    fn test_sign_and_verify_message_hash() {
        let mut rng = rand::thread_rng();
        let mut private_key = [0u8; SECRET_KEY_LENGTH];
        rng.fill(&mut private_key);
        let public_key = get_public_key(&private_key);

        let message = b"test message";
        let signature = sign_message_hash(message, &private_key).unwrap();

        let verify_result =
            verify_message_hash_signature(&signature.to_bytes(), message, &public_key);

        assert!(verify_result.is_ok());
    }
}
