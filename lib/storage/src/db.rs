use prost::Message;
use teleport_common::protobufs::generated::on_chain_event::Body::*;
use teleport_common::protobufs::generated::{OnChainEvent, SignerEventBody};
use uuid::Uuid;

const MAX_ROWS_PER_BATCH: usize = 50;

pub struct ChainEventRow {
    pub block_timestamp: u64,
    pub fid: u64,
    pub chain_id: u32,
    pub block_number: u32,
    pub transaction_index: u32,
    pub log_index: u32,
    pub r#type: i32,
    pub block_hash: Vec<u8>,
    pub transaction_hash: Vec<u8>,
    pub body: Vec<u8>,
    pub raw: Vec<u8>,
}

impl ChainEventRow {
    pub fn new(onchain_event: &OnChainEvent, raw_event: Vec<u8>) -> Self {
        let serialized_body = match &onchain_event.body {
            Some(body) => match body {
                SignerEventBody(event_body) => event_body.encode_to_vec(),
                SignerMigratedEventBody(event_body) => event_body.encode_to_vec(),
                IdRegisterEventBody(event_body) => event_body.encode_to_vec(),
                StorageRentEventBody(event_body) => event_body.encode_to_vec(),
            },
            None => vec![],
        };

        Self {
            block_timestamp: onchain_event.block_timestamp,
            fid: onchain_event.fid,
            chain_id: onchain_event.chain_id,
            block_number: onchain_event.block_number,
            transaction_index: onchain_event.tx_index,
            log_index: onchain_event.log_index,
            r#type: onchain_event.r#type,
            block_hash: onchain_event.block_hash.clone(),
            transaction_hash: onchain_event.transaction_hash.clone(),
            body: serialized_body,
            raw: raw_event,
        }
    }

    pub fn generate_bulk_insert_queries(
        rows: &[ChainEventRow],
    ) -> Result<Vec<String>, sqlx::Error> {
        let mut query_strings = Vec::new();

        for chunk in rows.chunks(MAX_ROWS_PER_BATCH) {
            let mut params = Vec::new();
            let sql = "INSERT INTO chain_events (block_timestamp, fid, chain_id, block_number, transaction_index, log_index, type, block_hash, transaction_hash, body, raw) VALUES";
            let conflict_sql = "ON CONFLICT (transaction_hash, log_index) DO NOTHING";

            for row in chunk {
                let values = format!(
                    "({}, {}, {}, {}, {}, {}, {}, '{}', '{}', '{}', '{}')",
                    row.block_timestamp as i64,
                    row.fid as i64,
                    row.chain_id as i32,
                    row.block_number as i32,
                    row.transaction_index as i32,
                    row.log_index as i32,
                    row.r#type as i32,
                    hex::encode(&row.block_hash),
                    hex::encode(&row.transaction_hash),
                    hex::encode(&row.body),
                    hex::encode(&row.raw)
                );
                params.push(values);
            }

            let query_string = format!("{} {} {}", sql, params.join(", "), conflict_sql);
            query_strings.push(query_string);
        }

        Ok(query_strings)
    }
}

pub struct FidRow {
    pub fid: i64,
    pub registered_at: i64,
    pub transaction_hash: Vec<u8>,
    pub log_index: u32,
    pub custody_address: [u8; 20],
    pub recovery_address: [u8; 20],
}

pub struct FidTransfer {
    pub fid: u32,
    pub custody_address: [u8; 20],
}

pub struct FidRecoveryUpdate {
    pub fid: u32,
    pub recovery_address: [u8; 20],
}

impl FidRow {
    pub fn generate_bulk_insert_queries(rows: &[FidRow]) -> Result<Vec<String>, sqlx::Error> {
        let mut query_strings = Vec::new();

        for chunk in rows.chunks(MAX_ROWS_PER_BATCH) {
            let mut params = Vec::new();
            let sql = "INSERT INTO fids (fid, registered_at, transaction_hash, log_index, custody_address, recovery_address) VALUES";
            let conflict_sql = "ON CONFLICT (fid) DO NOTHING";

            for row in chunk {
                let values = format!(
                    "({}, {}, '{}', {}, '{}', '{}')",
                    row.fid,
                    row.registered_at,
                    hex::encode(&row.transaction_hash),
                    row.log_index,
                    hex::encode(&row.custody_address),
                    hex::encode(&row.recovery_address)
                );
                params.push(values);
            }

            let query_string = format!("{} {} {}", sql, params.join(", "), conflict_sql);
            query_strings.push(query_string);
        }

        Ok(query_strings)
    }

    pub fn generate_bulk_transfer_queries(
        transfers: &[FidTransfer],
    ) -> Result<Vec<String>, sqlx::Error> {
        let mut query_strings = Vec::new();

        let sql = String::from("UPDATE fids SET custody_address = CASE ");

        for chunk in transfers.chunks(MAX_ROWS_PER_BATCH) {
            let mut cases: Vec<String> = vec![];
            let mut where_in: Vec<String> = vec![];

            for transfer in chunk {
                cases.push(format!(
                    "WHEN fid = {} THEN '{}'",
                    transfer.fid,
                    hex::encode(&transfer.custody_address)
                ));

                where_in.push(format!("'{}'", transfer.fid));
            }

            let query_string =
                sql.clone() + &cases.join(" ") + " END WHERE fid IN (" + &where_in.join(", ") + ")";
            query_strings.push(query_string);
        }

        Ok(query_strings)
    }

    pub fn generate_bulk_update_recovery_address_queries(
        updates: &[FidRecoveryUpdate],
    ) -> Result<Vec<String>, sqlx::Error> {
        let mut query_strings: Vec<String> = Vec::new();
        let sql = String::from("UPDATE fids SET recovery_address = CASE ");

        for chunk in updates.chunks(MAX_ROWS_PER_BATCH) {
            let mut cases: Vec<String> = vec![];
            let mut where_in: Vec<String> = vec![];

            for update in chunk {
                cases.push(format!(
                    "WHEN fid = {} THEN '{}'",
                    update.fid,
                    hex::encode(&update.recovery_address)
                ));

                where_in.push(update.fid.to_string());
            }

            let query_string =
                sql.clone() + &cases.join(" ") + " END WHERE fid IN (" + &where_in.join(", ") + ")";

            query_strings.push(query_string);
        }

        Ok(query_strings)
    }
}

#[derive(Debug)]
pub struct SignerRow {
    pub id: String,
    pub added_at: u64,
    pub removed_at: Option<u64>,
    pub fid: u64,
    pub requester_fid: u64,
    pub add_transaction_hash: Vec<u8>,
    pub add_log_index: u32,
    pub remove_transaction_hash: Option<Vec<u8>>,
    pub remove_log_index: Option<u32>,
    pub key_type: i64,
    pub metadata_type: i64,
    pub key: Vec<u8>,
    pub metadata: String,
}

pub struct SignerRemoved {
    pub fid: u64,
    pub key: Vec<u8>,
    pub remove_transaction_hash: Vec<u8>,
    pub remove_log_index: u32,
    pub removed_at: u64,
}

impl SignerRow {
    pub fn new(
        signer_event_body: &SignerEventBody,
        onchain_event: &OnChainEvent,
        requester_fid: u64,
        metadata: String,
    ) -> Self {
        let id = Uuid::new_v4().to_string();

        let added_at = onchain_event.block_timestamp * 1000; // block_timestamp is in seconds
        let removed_at = None;

        Self {
            id,
            added_at,
            removed_at,
            fid: onchain_event.fid,
            requester_fid,
            add_transaction_hash: onchain_event.transaction_hash.clone(),
            add_log_index: onchain_event.log_index,
            remove_transaction_hash: None,
            remove_log_index: None,
            key_type: signer_event_body.key_type.into(),
            metadata_type: signer_event_body.metadata_type.into(),
            key: signer_event_body.key.clone(),
            metadata,
        }
    }

    pub fn generate_bulk_insert_queries(rows: &[Self]) -> Result<Vec<String>, sqlx::Error> {
        let mut query_strings = Vec::new();

        let sql_prefix = "INSERT INTO signers (id, added_at, fid, requester_fid, add_transaction_hash, add_log_index, key_type, metadata_type, key, metadata) VALUES";
        let conflict_sql = "ON CONFLICT (fid, key) DO UPDATE SET remove_transaction_hash = NULL, remove_log_index = NULL";

        for chunk in rows.chunks(MAX_ROWS_PER_BATCH) {
            let mut values_list = Vec::new();

            for row in chunk {
                let values = format!(
                    "('{}', {}, {}, {}, '{}', {}, {}, {}, '{}', '{}')",
                    row.id,
                    row.added_at,
                    row.fid,
                    row.requester_fid,
                    hex::encode(&row.add_transaction_hash),
                    row.add_log_index,
                    row.key_type,
                    row.metadata_type,
                    hex::encode(&row.key),
                    row.metadata
                );
                values_list.push(values);
            }

            let query = format!("{} {} {}", sql_prefix, values_list.join(", "), conflict_sql);
            query_strings.push(query);
        }

        Ok(query_strings)
    }

    pub fn generate_bulk_remove_signer_queries(
        updates: &[SignerRemoved],
    ) -> Result<Vec<String>, sqlx::Error> {
        let mut query_strings = Vec::new();

        for update in updates {
            let query_str = format!("UPDATE signers SET remove_transaction_hash = '{}', remove_log_index = {}, removed_at = {} WHERE key = '{}' AND fid = {}", hex::encode(&update.remove_transaction_hash), update.remove_log_index, update.removed_at, hex::encode(&update.key), update.fid);
            query_strings.push(query_str);
        }

        Ok(query_strings)
    }
}

pub struct StorageAllocationRow {
    pub id: String,
    pub rented_at: i64,
    pub expires_at: u32,
    pub transaction_hash: Vec<u8>,
    pub log_index: u32,
    pub fid: u64,
    pub units: u32,
    pub payer: Vec<u8>,
}

impl StorageAllocationRow {
    pub fn new(
        rented_at: i64,
        expires_at: u32,
        transaction_hash: Vec<u8>,
        log_index: u32,
        fid: u64,
        units: u32,
        payer: Vec<u8>,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id,
            rented_at,
            expires_at,
            transaction_hash,
            log_index,
            fid,
            units,
            payer,
        }
    }

    pub fn generate_bulk_insert_queries(rows: &[Self]) -> Result<Vec<String>, sqlx::Error> {
        let mut queries = Vec::new();

        for chunk in rows.chunks(MAX_ROWS_PER_BATCH) {
            let mut values = Vec::new();
            for row in chunk {
                let value = format!(
                    "('{}', {}, {}, '{}', {}, {}, {}, '{}')",
                    row.id,
                    row.rented_at,
                    row.expires_at,
                    hex::encode(&row.transaction_hash),
                    row.log_index,
                    row.fid,
                    row.units,
                    hex::encode(&row.payer)
                );
                values.push(value);
            }
            let query = format!(
                "INSERT INTO storage_allocations (id, rented_at, expires_at, transaction_hash, log_index, fid, units, payer) VALUES {} {}",
                values.join(", "),
                "ON CONFLICT (transaction_hash, log_index) DO NOTHING",
            );
            queries.push(query);
        }

        Ok(queries)
    }
}
