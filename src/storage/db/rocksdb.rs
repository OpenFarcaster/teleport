use rocksdb::DB;

use crate::core::errors::HubError;

pub const DB_DIRECTORY: &str = ".rocks";
pub const MAX_DB_ITERATOR_OPEN_MS: u64 = 60 * 1000;

const DB_NAME_DEFAULT: &str = "farcaster";

struct RocksDB {
    db: DB,
}

impl RocksDB {
    pub fn new(name: Option<String>) -> Self {
        let path = format!(
            "{}/{}",
            DB_DIRECTORY,
            name.unwrap_or(DB_NAME_DEFAULT.to_string())
        );

        RocksDB {
            db: DB::open_default(path).unwrap(),
        }
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), HubError> {
        let put_res = self.db.put(key, value);

        if let Err(e) = put_res {
            return Err(parse_db_error(e));
        }

        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, HubError> {
        let get_res = self.db.get(key);

        if let Err(e) = get_res {
            return Err(parse_db_error(e));
        }

        Ok(get_res.unwrap())
    }

    pub fn get_many(&self, keys: &[&[u8]]) -> Result<Vec<Option<Vec<u8>>>, HubError> {
        let get_many_res = self.db.multi_get(keys);

        let mut values = Vec::new();

        for v in get_many_res {
            if let Err(e) = v {
                return Err(parse_db_error(e));
            }

            values.push(v.unwrap());
        }

        Ok(values)
    }

    pub fn del(&self, key: &[u8]) -> Result<(), HubError> {
        let del_res = self.db.delete(key);

        if let Err(e) = del_res {
            return Err(parse_db_error(e));
        }

        Ok(())
    }
}

fn parse_db_error(e: rocksdb::Error) -> HubError {
    if e.kind() == rocksdb::ErrorKind::NotFound {
        return HubError::NotFound(e.to_string());
    }

    return HubError::Unavailable(
        crate::core::errors::UnavailableType::StorageFailure,
        e.to_string(),
    );
}
