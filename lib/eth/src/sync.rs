use std::error::Error;

use ethers::{
    providers::{Http, Middleware, Provider},
    types::BlockNumber,
};
use teleport_storage::{db, Store};

use crate::id_registry::IdRegistry;

// todo: Is this right? IdRegistry seems to be deployed at 108869029u64
const FARCASTER_START_BLOCK: u64 = 108864739u64;

pub struct Syncer {
    pub store: Store,
    pub provider: Provider<Http>,
    chain_id: Option<u64>,
    id_registry: IdRegistry,
    key_registry: crate::key_registry::KeyRegistry,
    storage_registry: crate::storage_registry::StorageRegistry,
}

impl Syncer {
    pub fn new(
        http_rpc_url: String,
        store: Store,
        id_registry: IdRegistry,
        key_registry: crate::key_registry::KeyRegistry,
        storage_registry: crate::storage_registry::StorageRegistry,
    ) -> Result<Self, Box<dyn Error>> {
        let provider = Provider::<Http>::try_from(http_rpc_url)?
            .interval(std::time::Duration::from_millis(2000));

        Ok(Syncer {
            store,
            provider,
            id_registry,
            key_registry,
            storage_registry,
            chain_id: None,
        })
    }

    pub async fn with_chain_id(&mut self) -> Result<&mut Self, Box<dyn Error>> {
        let chain_id = self.provider.get_chainid().await?;
        let chain_id_u64 = chain_id.as_u64();
        self.chain_id = Some(chain_id_u64);
        Ok(self)
    }

    async fn get_start_block(&self) -> u64 {
        let max_block_num = db::ChainEventRow::max_block_number(&self.store)
            .await
            .unwrap_or(FARCASTER_START_BLOCK as i64);

        let start_block_num = if max_block_num == 0 {
            FARCASTER_START_BLOCK
        } else {
            max_block_num as u64 + 1
        };

        start_block_num
    }

    async fn get_latest_block(&self) -> u64 {
        // todo: error handling? we cannot proceed if this fails
        let latest_block = self
            .provider
            .get_block(BlockNumber::Latest)
            .await
            .unwrap()
            .unwrap();
        let latest_block_number = latest_block.number.unwrap().as_u64();
        latest_block_number
    }

    async fn sync_register_logs(&self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let register_logs = self.id_registry.get_register_logs(start, end).await?;
        let chain_id = self.chain_id.unwrap();
        for log in register_logs {
            self.id_registry
                .persist_register_log(log.clone(), chain_id as u32)
                .await?
        }

        Ok(())
    }

    async fn sync_transfer_logs(&self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let transfer_logs = self.id_registry.get_transfer_logs(start, end).await?;
        let chain_id = self.chain_id.unwrap();
        for log in transfer_logs {
            self.id_registry
                .persist_transfer_log(log.clone(), chain_id as u32)
                .await?
        }

        Ok(())
    }

    async fn sync_recovery_logs(&self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let recovery_logs = self.id_registry.get_recovery_logs(start, end).await?;
        let chain_id = self.chain_id.unwrap();
        for log in recovery_logs {
            self.id_registry
                .persist_recovery_log(log.clone(), chain_id as u32)
                .await?
        }

        Ok(())
    }

    async fn sync_add_logs(&self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let add_logs = self.key_registry.get_add_logs(start, end).await?;
        let chain_id = self.chain_id.unwrap();
        for log in add_logs {
            self.key_registry
                .persist_add_log(log.clone(), chain_id as u32)
                .await?
        }

        Ok(())
    }

    async fn sync_remove_logs(&self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let remove_logs = self.key_registry.get_remove_logs(start, end).await?;
        let chain_id = self.chain_id.unwrap();
        for log in remove_logs {
            self.key_registry
                .persist_remove_log(log.clone(), chain_id as u32)
                .await?
        }

        Ok(())
    }

    async fn sync_admin_reset_logs(&self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let admin_reset_logs = self.key_registry.get_admin_reset_logs(start, end).await?;
        let chain_id = self.chain_id.unwrap();
        for log in admin_reset_logs {
            self.key_registry
                .persist_admin_reset_log(log.clone(), chain_id as u32)
                .await?
        }

        Ok(())
    }

    async fn sync_migrated_logs(&self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let migrated_logs = self.key_registry.get_migrated_logs(start, end).await?;
        let chain_id = self.chain_id.unwrap();
        for log in migrated_logs {
            self.key_registry
                .persist_migrated_log(log.clone(), chain_id as u32)
                .await?
        }

        Ok(())
    }

    async fn sync_rent_logs(&self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let rent_logs = self.storage_registry.get_rent_logs(start, end).await?;
        let chain_id = self.chain_id.unwrap();
        for log in rent_logs {
            self.storage_registry
                .persist_rent_log(log.clone(), chain_id as u32)
                .await?
        }

        Ok(())
    }

    async fn sync_set_max_units_logs(&self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let set_max_units_logs = self
            .storage_registry
            .get_set_max_units_logs(start, end)
            .await?;
        let chain_id = self.chain_id.unwrap();
        for log in set_max_units_logs {
            self.storage_registry
                .persist_set_max_units_log(log.clone(), chain_id as u32)
                .await?
        }

        Ok(())
    }

    async fn sync_deprecation_timestamp_logs(
        &self,
        start: u64,
        end: u64,
    ) -> Result<(), Box<dyn Error>> {
        let deprecation_timestamp_logs = self
            .storage_registry
            .get_deprecation_timestamp_logs(start, end)
            .await?;
        let chain_id = self.chain_id.unwrap();
        for log in deprecation_timestamp_logs {
            self.storage_registry
                .persist_deprecation_timestamp_log(log.clone(), chain_id as u32)
                .await?
        }

        Ok(())
    }

    pub async fn sync(&self) -> Result<(), Box<dyn Error>> {
        let start_block_num = self.get_start_block().await;
        let latest_block_number = self.get_latest_block().await;
        let mut current_block_num = start_block_num;

        log::info!(
            "syncing events from block {} to {}",
            current_block_num,
            latest_block_number
        );

        // fetch logs in range [current_block_num, current_block_num + 2000]
        while current_block_num <= latest_block_number {
            let percent_complete = (current_block_num - start_block_num) as f64
                / (latest_block_number - start_block_num) as f64;
            log::info!("events sync progress = {:.2}%", percent_complete * 100.0);

            let start = current_block_num;
            let end = current_block_num + 2000;

            self.sync_register_logs(start, end).await?;
            self.sync_transfer_logs(start, end).await?;
            self.sync_recovery_logs(start, end).await?;
            self.sync_add_logs(start, end).await?;
            self.sync_remove_logs(start, end).await?;
            self.sync_admin_reset_logs(start, end).await?;
            self.sync_migrated_logs(start, end).await?;
            self.sync_rent_logs(start, end).await?;
            self.sync_set_max_units_logs(start, end).await?;
            self.sync_deprecation_timestamp_logs(start, end).await?;

            current_block_num = end + 1;
        }
        Ok(())
    }
}
