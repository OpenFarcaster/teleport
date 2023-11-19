pub struct UserNameProofClaim {
    name: String,
    owner: String,
    timestamp: u64,
}

impl UserNameProofClaim {
    fn new(name: String, owner: String, timestamp: u64) -> Self {
        Self {
            name,
            owner,
            timestamp,
        }
    }
}
