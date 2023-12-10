use sqlx::Row;
use uuid::Uuid;

pub struct ChainEventRow {
    pub id: String,
    pub block_timestamp: i64,
    pub fid: i64,
    pub chain_id: i64,
    pub block_number: i64,
    pub transaction_index: i64,
    pub log_index: i64,
    pub r#type: i64,
    pub block_hash: [u8; 32],
    pub transaction_hash: String,
    pub body: String,
    pub raw: Vec<u8>,
}

impl ChainEventRow {
    pub fn new(
        fid: u64,
        chain_id: u64,
        block_number: u64,
        transaction_index: u64,
        log_index: u64,
        r#type: u64,
        block_hash: [u8; 32],
        transaction_hash: String,
        body: String,
        raw: Vec<u8>,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        // TODO: there is no efficient way to get the timestamp from the block
        // without fetching the block itself in another RPC call
        let block_timestamp = 0;

        ChainEventRow {
            id,
            fid: fid as i64,
            block_timestamp,
            block_number: block_number as i64,
            block_hash,
            chain_id: chain_id as i64,
            transaction_index: transaction_index as i64,
            transaction_hash,
            log_index: log_index as i64,
            r#type: r#type as i64,
            body,
            raw,
        }
    }

    pub async fn insert(&self, store: &crate::Store) -> Result<(), sqlx::Error> {
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
            .bind(self.block_timestamp)
            .bind(self.fid)
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
        Ok(())
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
        from: [u8; 20],
        to: [u8; 20],
    ) -> Result<(), sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let query = "update fids set recovery_address = ? where recovery_address = ? and fid = ?;";
        sqlx::query(query)
            .bind(from.as_slice())
            .bind(to.as_slice())
            .bind(fid as i64)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn update_custody_address(
        store: &crate::Store,
        fid: u64,
        from: [u8; 20],
        to: [u8; 20],
    ) -> Result<(), sqlx::Error> {
        let mut conn = store.conn.acquire().await.unwrap();
        let query = "update fids set custody_address = ? where custody_address = ? and fid = ?;";
        sqlx::query(query)
            .bind(from.as_slice())
            .bind(to.as_slice())
            .bind(fid as i64)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}
