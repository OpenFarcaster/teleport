use prost::Message;
use sqlx::QueryBuilder;
use teleport_common::protobufs::generated::on_chain_event::Body::*;
use teleport_common::protobufs::generated::OnChainEvent;
use uuid::Uuid;

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
        let id = self.id.clone();
        let block_timestamp = self.block_timestamp as i64;
        let fid = self.fid as i64;
        let block_hash = self.block_hash.clone();
        let transaction_hash = self.transaction_hash.clone();
        let body = self.body.clone();
        let raw = self.raw.clone();
        sqlx::query_file!(
            "src/queries/insert_chain_event.sql",
            id,
            block_timestamp,
            fid,
            self.chain_id,
            self.block_number,
            self.transaction_index,
            self.log_index,
            self.r#type,
            block_hash,
            transaction_hash,
            body,
            raw
        )
        .execute(&mut *conn)
        .await?;

        Ok(id)
    }

    pub async fn bulk_insert(
        store: &crate::Store,
        rows: &[ChainEventRow],
    ) -> Result<(), sqlx::Error> {
        const MAX_PARAMS: usize = 999;
        let params_per_row = 13; // Number of fields in ChainEventRow
        let max_rows_per_batch = MAX_PARAMS / params_per_row;

        for chunk in rows.chunks(max_rows_per_batch) {
            let mut query_builder = QueryBuilder::new(
                "INSERT INTO chain_events (id, block_timestamp, fid, chain_id, block_number, transaction_index, log_index, type, block_hash, transaction_hash, body, raw) ",
            );

            query_builder.push_values(chunk.iter(), |mut b, row| {
                b.push_bind(&row.id)
                    .push_bind(row.block_timestamp as i64)
                    .push_bind(row.fid as i64)
                    .push_bind(row.chain_id as i32)
                    .push_bind(row.block_number as i32)
                    .push_bind(row.transaction_index as i32)
                    .push_bind(row.log_index as i32)
                    .push_bind(row.r#type as i32)
                    .push_bind(&row.block_hash)
                    .push_bind(&row.transaction_hash)
                    .push_bind(&row.body)
                    .push_bind(&row.raw);
            });

            let query = query_builder.build();

            let mut conn = store.conn.acquire().await.unwrap();
            query.execute(&mut *conn).await?;
        }

        Ok(())
    }

    pub async fn max_block_number(store: &crate::Store) -> Result<i64, sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let row = sqlx::query_file!("src/queries/max_block_number.sql")
            .fetch_one(&mut *conn)
            .await?;

        Ok(row.block_number)
    }
}

pub struct FidRow {
    pub fid: i64,
    pub registered_at: i64,
    pub chain_event_id: String,
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
    pub async fn insert(&self, store: &crate::Store) -> Result<(), sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let recovery_address = self.recovery_address.clone();
        let recovery_address = recovery_address.as_slice();
        let chain_event_id = self.chain_event_id.clone();
        let custody_address = self.custody_address.clone();
        let custody_address = custody_address.as_slice();
        sqlx::query_file!(
            "src/queries/insert_fid.sql",
            self.fid,
            self.registered_at,
            chain_event_id,
            custody_address,
            recovery_address
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    pub async fn update_recovery_address(
        store: &crate::Store,
        update: &FidRecoveryUpdate,
    ) -> Result<(), sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let fid = update.fid as i64;
        let to = update.recovery_address.as_slice();
        sqlx::query_file!("src/queries/update_recovery_address.sql", to, fid)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn transfer(store: &crate::Store, update: &FidTransfer) -> Result<(), sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let fid = update.fid as i64;
        let to = update.custody_address.as_slice();
        sqlx::query_file!("src/queries/update_custody_address.sql", to, fid)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn bulk_insert(store: &crate::Store, rows: &[FidRow]) -> Result<(), sqlx::Error> {
        const MAX_PARAMS: usize = 999;
        let params_per_row = 5; // TODO: derive this from number of fields in FidRow rather than a hardcoded size
        let max_rows_per_batch = MAX_PARAMS / params_per_row;

        for chunk in rows.chunks(max_rows_per_batch) {
            let mut query_builder = QueryBuilder::new(
            "INSERT INTO fids (fid, registered_at, chain_event_id, custody_address, recovery_address) ",
        );

            query_builder.push_values(chunk.iter(), |mut b, row| {
                b.push_bind(row.fid as u32)
                    .push_bind(row.registered_at as u32)
                    .push_bind(&row.chain_event_id)
                    .push_bind(row.custody_address.as_slice())
                    .push_bind(row.recovery_address.as_slice());
            });

            query_builder.push(" ON CONFLICT (fid) DO NOTHING");

            let query = query_builder.build();

            let mut conn = store.conn.acquire().await.unwrap();
            query.execute(&mut *conn).await?;
        }

        Ok(())
    }

    pub async fn bulk_transfer(
        store: &crate::Store,
        transfers: &[FidTransfer],
    ) -> Result<(), sqlx::Error> {
        const MAX_PARAMS: usize = 999;
        let params_per_transfer = 2; // Each transfer requires two parameters (fid and custody_address)
        let max_transfers_per_batch = MAX_PARAMS / params_per_transfer;

        for chunk in transfers.chunks(max_transfers_per_batch) {
            let mut sql = String::from("UPDATE fids SET custody_address = CASE fid ");
            let mut params: Vec<(i64, Vec<u8>)> = Vec::new();

            for transfer in chunk {
                sql.push_str(&format!(" WHEN ? THEN ? "));
                params.push((
                    transfer.fid as i64,
                    transfer.custody_address.clone().to_vec(),
                ));
            }

            sql.push_str(" END WHERE fid IN (");
            sql.push_str(&"?,".repeat(chunk.len()).trim_end_matches(','));
            sql.push_str(")");

            let mut query = sqlx::query(&sql);

            for (fid, custody_address) in &params {
                query = query.bind(*fid).bind(custody_address);
            }

            for transfer in chunk {
                query = query.bind(transfer.fid as i64);
            }

            query
                .execute(&mut *store.conn.acquire().await.unwrap())
                .await?;
        }

        Ok(())
    }

    pub async fn bulk_update_recovery_address(
        store: &crate::Store,
        updates: &[FidRecoveryUpdate],
    ) -> Result<(), sqlx::Error> {
        const MAX_PARAMS: usize = 999;
        let params_per_update = 2; // Each update requires two parameters (fid and recovery_address)
        let max_updates_per_batch = MAX_PARAMS / params_per_update;

        for chunk in updates.chunks(max_updates_per_batch) {
            let mut sql = String::from("UPDATE fids SET recovery_address = CASE fid ");
            let mut params: Vec<(i64, Vec<u8>)> = Vec::new();

            for update in chunk {
                sql.push_str(&format!(" WHEN ? THEN ? "));
                params.push((update.fid as i64, update.recovery_address.clone().to_vec()));
            }

            sql.push_str(" END WHERE fid IN (");
            sql.push_str(&"?,".repeat(chunk.len()).trim_end_matches(','));
            sql.push_str(")");

            let mut query = sqlx::query(&sql);

            for (fid, recovery_address) in &params {
                query = query.bind(*fid).bind(recovery_address);
            }

            for update in chunk {
                query = query.bind(update.fid as i64);
            }

            query
                .execute(&mut *store.conn.acquire().await.unwrap())
                .await?;
        }

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
    pub key_type: i64,
    pub metadata_type: i64,
    pub key: Vec<u8>,
    pub metadata: String,
}

impl SignerRow {
    pub fn new(
        fid: u64,
        requester_fid: u64,
        add_chain_event_id: String,
        remove_chain_event_id: Option<String>,
        key_type: i64,
        metadata_type: i64,
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
        let fid = self.fid as i64;
        let requester_fid = self.requester_fid as i64;
        let add_chain_event_id = self.add_chain_event_id.clone();
        let remove_chain_event_id = self.remove_chain_event_id.clone();
        let metadata = self.metadata.clone();
        let key = self.key.clone();
        sqlx::query_file!(
            "src/queries/insert_signer.sql",
            self.id,
            self.added_at,
            self.removed_at,
            fid,
            requester_fid,
            add_chain_event_id,
            remove_chain_event_id,
            self.key_type,
            self.metadata_type,
            key,
            metadata
        )
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
        let result = sqlx::query_file!(
            "src/queries/update_remove_chain_event.sql",
            remove_chain_event_id,
            key,
            1i16
        )
        .execute(&mut *conn)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn get_by_key(
        store: &crate::Store,
        key: Vec<u8>,
    ) -> Result<(i64, String), sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let record = sqlx::query_file!("src/queries/signer_metadata_by_key.sql", key)
            .fetch_one(&mut *conn)
            .await?;

        Ok((record.key_type, record.metadata))
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
        let payer = self.payer.clone();
        let payer = payer.as_slice();
        let fid = self.fid as i64;
        let units = self.units as i64;
        let chain_event_id = self.chain_event_id.clone();
        sqlx::query_file!(
            "src/queries/insert_storage_allocation.sql",
            self.id,
            self.rented_at,
            self.expires_at,
            chain_event_id,
            fid,
            units,
            payer
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }
}
