use std::{fs::File, io::BufReader, path::Path};

use serde::{Deserialize, Serialize};

const STATE_FILENAME: &str = "state.json";

#[derive(Serialize, Deserialize)]
pub struct PersistentState {
    pub last_synced_block: u64,
}

impl PersistentState {
    pub fn load() -> Self {
        let path = Path::new(STATE_FILENAME);
        if path.exists() && path.is_file() {
            let file = File::open(path).unwrap();
            serde_json::from_reader(BufReader::new(file)).unwrap()
        } else {
            Self::default()
        }
    }

    pub fn store(&self) {
        let file = File::create(STATE_FILENAME).unwrap();
        serde_json::to_writer(file, self).unwrap();
    }
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            last_synced_block: 0,
        }
    }
}
