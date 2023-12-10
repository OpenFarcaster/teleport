use std::collections::HashMap;


enum TrieNodeLike {
    Serialized(SerializedTrieNode),
    Full(TrieNode),
}

struct SerializedTrieNode {
    hash: Option<[u8; 32]>,
}

impl SerializedTrieNode {
    pub fn new(hash: Option<[u8; 32]>) -> Self {
        SerializedTrieNode { hash }
    }
}

struct TrieNode {
    hash: [u8; 32],
    items: u128,
    children: HashMap<u128, TrieNodeLike>,
    key: Option<Vec<u8>>,
}

impl TrieNode {
    pub fn new() -> Self {
        Self {
            hash: [0; 32],
            items: 0,
            children: HashMap::new(),
            key: None,
        }
    }
}
