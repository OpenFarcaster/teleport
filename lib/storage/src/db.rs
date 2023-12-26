use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use uuid::Uuid;

use prost::Message;
use teleport_common::protobufs::generated::on_chain_event::Body::*;
use teleport_common::protobufs::generated::OnChainEvent;

pub struct ChainEventRow {
    pub id: String,
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
    pub fn new(onchain_event: OnChainEvent, raw_event: Vec<u8>) -> Self {
        let id = Uuid::new_v4().to_string();

        let serialized_body = match onchain_event.body {
            Some(body) => match body {
                SignerEventBody(event_body) => event_body.encode_to_vec(),
                SignerMigratedEventBody(event_body) => event_body.encode_to_vec(),
                IdRegisterEventBody(event_body) => event_body.encode_to_vec(),
                StorageRentEventBody(event_body) => event_body.encode_to_vec(),
            },
            None => vec![],
        };

        Self {
            id,
            block_timestamp: onchain_event.block_timestamp,
            fid: onchain_event.fid,
            chain_id: onchain_event.chain_id,
            block_number: onchain_event.block_number,
            transaction_index: onchain_event.tx_index,
            log_index: onchain_event.log_index,
            r#type: onchain_event.r#type,
            block_hash: onchain_event.block_hash,
            transaction_hash: onchain_event.transaction_hash,
            body: serialized_body,
            raw: raw_event,
        }
    }

    pub async fn insert(&self, store: &crate::Store) -> Result<String, sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let query = "insert into chain_events (
            id,
            block_timestamp,
            fid,
            chain_id,
            block_number,
            transaction_index,
            log_index,
            type,
            block_hash,
            transaction_hash,
            body,
            raw
        ) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
        ";
        sqlx::query(query)
            .bind(self.id.clone())
            .bind(self.block_timestamp as i64)
            .bind(self.fid as i64)
            .bind(self.chain_id)
            .bind(self.block_number)
            .bind(self.transaction_index)
            .bind(self.log_index)
            .bind(self.r#type)
            .bind(self.block_hash.clone().as_slice())
            .bind(self.transaction_hash.clone())
            .bind(self.body.clone())
            .bind(self.raw.clone())
            .execute(&mut *conn)
            .await?;
        Ok(self.id.clone())
    }

    pub async fn max_block_number(store: &crate::Store) -> Result<i64, sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let query = "SELECT max(block_number) from chain_events;";
        let row = sqlx::query(query).fetch_one(&mut *conn).await?;
        let max_block_number: i64 = row.get(0);
        Ok(max_block_number)
    }
}

pub struct FidRow {
    pub fid: i64,
    pub registered_at: i64,
    pub chain_event_id: String,
    pub custody_address: [u8; 20],
    pub recovery_address: [u8; 20],
}

impl FidRow {
    pub async fn insert(&self, store: &crate::Store) -> Result<(), sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let query = "insert into fids (
            fid,
            registered_at,
            chain_event_id,
            custody_address,
            recovery_address
        ) 
        VALUES (?, ?, ?, ?, ?);
        ";
        sqlx::query(query)
            .bind(self.fid)
            .bind(self.registered_at)
            .bind(self.chain_event_id.clone())
            .bind(self.custody_address.clone().as_slice())
            .bind(self.recovery_address.clone().as_slice())
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn update_recovery_address(
        store: &crate::Store,
        fid: u64,
        to: [u8; 20],
    ) -> Result<(), sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let query = "update fids set recovery_address = ? where fid = ?;";
        sqlx::query(query)
            .bind(to.as_slice())
            .bind(fid as i64)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn update_custody_address(
        store: &crate::Store,
        fid: u64,
        to: [u8; 20],
    ) -> Result<(), sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let query = "update fids set custody_address = ? where fid = ?;";
        sqlx::query(query)
            .bind(to.as_slice())
            .bind(fid as i64)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}

pub struct SignerRow {
    pub id: String,
    pub added_at: String,
    pub removed_at: Option<String>,
    pub fid: u64,
    pub requester_fid: u64,
    pub add_chain_event_id: String,
    pub remove_chain_event_id: Option<String>,
    pub key_type: i16,
    pub metadata_type: i16,
    pub key: Vec<u8>,
    pub metadata: String,
}

impl SignerRow {
    pub fn new(
        fid: u64,
        requester_fid: u64,
        add_chain_event_id: String,
        remove_chain_event_id: Option<String>,
        key_type: i16,
        metadata_type: i16,
        key: Vec<u8>,
        metadata: String,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let added_at = "0".to_string();
        let removed_at = None;
        Self {
            id,
            added_at,
            removed_at,
            fid,
            requester_fid,
            add_chain_event_id,
            remove_chain_event_id,
            key_type,
            metadata_type,
            key,
            metadata,
        }
    }

    pub async fn insert(&self, store: &crate::Store) -> Result<(), sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let query = "insert into signers (
            id,
            added_at,
            removed_at,
            fid,
            requester_fid,
            add_chain_event_id,
            remove_chain_event_id,
            key_type,
            metadata_type,
            key,
            metadata
        ) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
        ";
        sqlx::query(query)
            .bind(self.id.clone())
            .bind(self.added_at.clone())
            .bind(self.removed_at.clone())
            .bind(self.fid as i64)
            .bind(self.requester_fid as i64)
            .bind(self.add_chain_event_id.clone())
            .bind(self.remove_chain_event_id.clone())
            .bind(self.key_type)
            .bind(self.metadata_type)
            .bind(self.key.clone())
            .bind(self.metadata.clone())
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn update_remove_chain_event(
        store: &crate::Store,
        key: Vec<u8>,
        remove_chain_event_id: String,
    ) -> Result<u64, sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let query = "update signers set remove_chain_event_id = ? where key = ? and key_type = ?;";
        let result = sqlx::query(query)
            .bind(remove_chain_event_id)
            .bind(key)
            .bind(1 as i16)
            .execute(&mut *conn)
            .await?;

        Ok(result.rows_affected())
    }

    pub async fn get_by_key(store: &crate::Store, key: Vec<u8>) -> Result<SqliteRow, sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let query = "select * from signers where key = ?;";
        let row = sqlx::query(query)
            .bind(key)
            .fetch_optional(&mut *conn)
            .await?;
        row.ok_or(sqlx::Error::RowNotFound)
    }
}

pub struct StorageAllocationRow {
    pub id: String,
    pub rented_at: i64,
    pub expires_at: u32,
    pub chain_event_id: String,
    pub fid: u64,
    pub units: u32,
    pub payer: Vec<u8>,
}

impl StorageAllocationRow {
    pub fn new(
        rented_at: i64,
        expires_at: u32,
        chain_event_id: String,
        fid: u64,
        units: u32,
        payer: Vec<u8>,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id,
            rented_at,
            expires_at,
            chain_event_id,
            fid,
            units,
            payer,
        }
    }

    pub async fn insert(&self, store: &crate::Store) -> Result<(), sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let query = "insert into storage_allocations (
            id,
            rented_at,
            expires_at,
            chain_event_id,
            fid,
            units,
            payer
        ) 
        VALUES (?, ?, ?, ?, ?, ?, ?);
        ";
        sqlx::query(query)
            .bind(self.id.clone())
            .bind(self.rented_at)
            .bind(self.expires_at)
            .bind(self.chain_event_id.clone())
            .bind(self.fid as i64)
            .bind(self.units as i64)
            .bind(self.payer.clone())
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}
