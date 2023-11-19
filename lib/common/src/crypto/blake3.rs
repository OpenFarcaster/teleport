use blake3::Hasher;

pub fn blake3_20(message: &[u8]) -> [u8; 20] {
    let mut hasher = Hasher::new();
    hasher.update(message);

    let mut result = [0u8; 20];

    hasher.finalize_xof().fill(&mut result);
    result
}
