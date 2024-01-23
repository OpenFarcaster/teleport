use crate::id_registry;
use crate::key_registry;
use crate::storage_registry;
use ethers::types::H256;
use ethers::{
    providers::{JsonRpcClient, Middleware, Provider},
    types::BlockNumber,
};
use std::collections::HashMap;
use std::error::Error;
use teleport_storage::{db, Store};
use tokio;

// todo: Is this right? IdRegistry seems to be deployed at 108869029u64
// const FARCASTER_START_BLOCK: u64 = 108864739u64;
const FARCASTER_START_BLOCK: u64 = 111816370u64;

pub struct Indexer<T> {
    store: Store,
    provider: Provider<T>,
    chain_id: u32,
    id_registry: id_registry::Contract<T>,
    key_registry: key_registry::Contract<T>,
    storage_registry: storage_registry::Contract<T>,
    block_timestamp_cache: HashMap<H256, i64>,
}

impl<T: JsonRpcClient + Clone> Indexer<T> {
    pub fn new(
        store: Store,
        provider: Provider<T>,
        chain_id: u32,
        id_reg_address: String,
        key_reg_address: String,
        storage_reg_address: String,
        abi_dir: String,
    ) -> Result<Self, Box<dyn Error>> {
        let id_registry = id_registry::Contract::new(
            provider.clone(),
            id_reg_address,
            format!("{}/IdRegistry.json", abi_dir),
        )?;
        let key_registry = key_registry::Contract::new(
            provider.clone(),
            key_reg_address,
            format!("{}/KeyRegistry.json", abi_dir),
        )?;
        let storage_registry = storage_registry::Contract::new(
            provider.clone(),
            storage_reg_address,
            format!("{}/StorageRegistry.json", abi_dir),
        )?;
        let block_timestamp_cache = HashMap::new();

        Ok(Indexer {
            store,
            provider,
            id_registry,
            key_registry,
            storage_registry,
            chain_id,
            block_timestamp_cache,
        })
    }

    pub async fn get_start_block(&self) -> u64 {
        let max_block_num = db::ChainEventRow::max_block_number(&self.store)
            .await
            .unwrap_or(FARCASTER_START_BLOCK as i64);

        if max_block_num == 0 {
            FARCASTER_START_BLOCK
        } else {
            max_block_num as u64 + 1
        }
    }

    pub async fn get_latest_block(&self) -> Result<u64, Box<dyn Error>> {
        let latest_block = self.provider.get_block(BlockNumber::Latest).await?.unwrap();
        Ok(latest_block.number.unwrap().as_u64())
    }

    pub async fn get_block_timestamp(&mut self, block_hash: H256) -> Result<i64, Box<dyn Error>> {
        if let Some(timestamp) = self.block_timestamp_cache.get(&block_hash) {
            return Ok(*timestamp);
        }

        let block = self.provider.get_block(block_hash).await?.unwrap();
        let timestamp = block.timestamp.as_u32().into();
        self.block_timestamp_cache.insert(block_hash, timestamp);
        Ok(timestamp)
    }

    async fn sync_register_logs(&mut self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let register_logs = self.id_registry.get_register_logs(start, end).await?;
        for log in register_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.id_registry
                .persist_register_log(&self.store, &log, self.chain_id, timestamp)
                .await?
        }

        Ok(())
    }

    async fn sync_transfer_logs(&mut self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let transfer_logs = self.id_registry.get_transfer_logs(start, end).await?;
        for log in transfer_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.id_registry
                .persist_transfer_log(&self.store, &log, self.chain_id, timestamp)
                .await?;
        }

        Ok(())
    }

    async fn sync_recovery_logs(&mut self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let recovery_logs = self.id_registry.get_recovery_logs(start, end).await?;
        for log in recovery_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.id_registry
                .persist_recovery_log(&self.store, &log, self.chain_id, timestamp)
                .await?
        }

        Ok(())
    }

    async fn sync_change_recovery_address_logs(
        &mut self,
        start: u64,
        end: u64,
    ) -> Result<(), Box<dyn Error>> {
        let change_recovery_address_logs = self
            .id_registry
            .get_change_recovery_address_logs(start, end)
            .await?;
        for log in change_recovery_address_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.id_registry
                .persist_change_recovery_address_log(&self.store, &log, self.chain_id, timestamp)
                .await?
        }

        Ok(())
    }

    async fn sync_add_logs(&mut self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let add_logs = self.key_registry.get_add_logs(start, end).await?;
        for log in add_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.key_registry
                .persist_add_log(&self.store, &log, self.chain_id, timestamp)
                .await?
        }

        Ok(())
    }

    async fn sync_remove_logs(&mut self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let remove_logs = self.key_registry.get_remove_logs(start, end).await?;
        for log in remove_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.key_registry
                .persist_remove_log(&self.store, &log, self.chain_id, timestamp)
                .await?
        }

        Ok(())
    }

    async fn sync_admin_reset_logs(&mut self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let admin_reset_logs = self.key_registry.get_admin_reset_logs(start, end).await?;
        for log in admin_reset_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.key_registry
                .persist_admin_reset_log(&self.store, &log, self.chain_id, timestamp)
                .await?
        }

        Ok(())
    }

    async fn sync_migrated_logs(&mut self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let migrated_logs = self.key_registry.get_migrated_logs(start, end).await?;
        for log in migrated_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.key_registry
                .persist_migrated_log(&self.store, &log, self.chain_id, timestamp)
                .await?
        }

        Ok(())
    }

    async fn sync_rent_logs(&mut self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let rent_logs = self.storage_registry.get_rent_logs(start, end).await?;
        for log in rent_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.storage_registry
                .persist_rent_log(&self.store, &log, self.chain_id, timestamp)
                .await?
        }

        Ok(())
    }

    async fn sync_set_max_units_logs(
        &mut self,
        start: u64,
        end: u64,
    ) -> Result<(), Box<dyn Error>> {
        let set_max_units_logs = self
            .storage_registry
            .get_set_max_units_logs(start, end)
            .await?;
        for log in set_max_units_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.storage_registry
                .persist_set_max_units_log(&self.store, &log, self.chain_id, timestamp)
                .await?
        }

        Ok(())
    }

    async fn sync_deprecation_timestamp_logs(
        &mut self,
        start: u64,
        end: u64,
    ) -> Result<(), Box<dyn Error>> {
        let deprecation_timestamp_logs = self
            .storage_registry
            .get_deprecation_timestamp_logs(start, end)
            .await?;
        for log in deprecation_timestamp_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.storage_registry
                .persist_deprecation_timestamp_log(&self.store, &log, self.chain_id, timestamp)
                .await?
        }

        Ok(())
    }

    pub async fn subscribe(
        &mut self,
        start_block: u64,
        interval_in_secs: u64,
    ) -> Result<(), Box<dyn Error>> {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(interval_in_secs));
        let mut current_block = start_block;
        loop {
            interval.tick().await;
            let latest_block = self.get_latest_block().await?;
            self.sync(current_block, latest_block).await.unwrap();
            current_block = latest_block + 1;
        }
    }

    pub async fn sync(&mut self, start_block: u64, end_block: u64) -> Result<(), Box<dyn Error>> {
        let mut current_block = start_block;

        log::info!(
            "syncing events from block {} to {}",
            current_block,
            end_block
        );

        // fetch logs in range [current_block_num, current_block_num + 2000]
        while current_block <= end_block {
            let percent_complete =
                (current_block - start_block) as f64 / (end_block - start_block) as f64;
            log::info!("events sync progress = {:.2}%", percent_complete * 100.0);

            // Clear block timestamp cache to avoid overloading it with useless data
            self.block_timestamp_cache.clear();

            let start = current_block;
            let end = current_block + 2000;

            // id registry logs
            self.sync_register_logs(start, end).await?;
            self.sync_transfer_logs(start, end).await?;
            self.sync_recovery_logs(start, end).await?;
            self.sync_change_recovery_address_logs(start, end).await?;

            // key registry logs
            self.sync_add_logs(start, end).await?;
            self.sync_remove_logs(start, end).await?;
            self.sync_admin_reset_logs(start, end).await?;
            self.sync_migrated_logs(start, end).await?;

            // storage registry logs
            self.sync_rent_logs(start, end).await?;
            self.sync_set_max_units_logs(start, end).await?;
            self.sync_deprecation_timestamp_logs(start, end).await?;

            current_block = end + 1;
        }
        Ok(())
    }
}
