use blake3::Hasher;

pub fn blake3_20(message: &[u8]) -> [u8; 20] {
    let mut hasher = Hasher::new();
    hasher.update(message);

    let mut result = [0u8; 20];

    hasher.finalize_xof().fill(&mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_20() {
        let message = b"hello world";
        let hash = blake3_20(message);

        // Convert hash to hexadecimal string
        let hash_hex = hex::encode(hash);

        // This is the expected hash for "hello world" using Blake3 and taking the first 20 bytes.
        let expected_hash_hex = "d74981efa70a0c880b8d8c1985d075dbcbf679b9";

        assert_eq!(hash_hex, expected_hash_hex);
    }
}
