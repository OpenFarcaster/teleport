use crate::id_registry;
use crate::id_registry::{
    CHANGE_RECOVERY_ADDRESS_SIGNATURE, RECOVERY_SIGNATURE, REGISTER_SIGNATURE, TRANSFER_SIGNATURE,
};
use crate::key_registry;
use crate::storage_registry;
use crate::utils::{get_block_timestamp, get_signature_topic};
use ethers::{
    providers::{JsonRpcClient, Middleware, Provider},
    types::{BlockNumber, Log, H256},
};
use futures::future::join_all;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use teleport_storage::{db, Store};
use tokio;

// todo: Is this right? IdRegistry seems to be deployed at 108869029u64
// const FARCASTER_START_BLOCK: u64 = 108864739u64;
const FARCASTER_START_BLOCK: u64 = 111816370u64;
// Number of blocks to read at a time
const BLOCK_INTERVAL: u64 = 2000;

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

    pub async fn get_block_timestamp(&mut self, block_hash: H256) -> Result<u32, Box<dyn Error>> {
        if let Some(timestamp) = self.block_timestamp_cache.get(&block_hash) {
            return Ok(*timestamp as u32);
        }

        let block = self.provider.get_block(block_hash).await?.unwrap();
        let timestamp = block.timestamp.as_u32().into();
        self.block_timestamp_cache.insert(block_hash, timestamp);
        Ok(timestamp as u32)
    }

    async fn collect_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let register_future = self.id_registry.get_register_logs(start_block, end_block);
        let transfer_future = self.id_registry.get_transfer_logs(start_block, end_block);
        let recovery_future = self.id_registry.get_recovery_logs(start_block, end_block);
        let change_recovery_address_future = self
            .id_registry
            .get_change_recovery_address_logs(start_block, end_block);
        // let add_future = self.key_registry.get_add_logs(start_block, end_block);
        // let remove_future = self.key_registry.get_remove_logs(start_block, end_block);
        // let admin_reset_future = self
        //     .key_registry
        //     .get_admin_reset_logs(start_block, end_block);
        // let migrated_future = self.key_registry.get_migrated_logs(start_block, end_block);
        // let rent_future = self.storage_registry.get_rent_logs(start_block, end_block);
        // let set_max_units_future = self
        //     .storage_registry
        //     .get_set_max_units_logs(start_block, end_block);
        // let deprecation_timestamp_future = self
        //     .storage_registry
        //     .get_deprecation_timestamp_logs(start_block, end_block);

        let (
            register_logs,
            transfer_logs,
            recovery_logs,
            change_recovery_address_logs,
            // add_logs,
            // remove_logs,
            // admin_reset_logs,
            // migrated_logs,
            // rent_logs,
            // set_max_units_logs,
            // deprecation_timestamp_logs,
        ) = tokio::try_join!(
            register_future,
            transfer_future,
            recovery_future,
            change_recovery_address_future,
            // add_future,
            // remove_future,
            // admin_reset_future,
            // migrated_future,
            // rent_future,
            // set_max_units_future,
            // deprecation_timestamp_future
        )?;

        let mut collected_logs = Vec::new();

        collected_logs.extend(register_logs);
        collected_logs.extend(transfer_logs);
        collected_logs.extend(recovery_logs);
        collected_logs.extend(change_recovery_address_logs);
        // collected_logs.extend(add_logs);
        // collected_logs.extend(remove_logs);
        // collected_logs.extend(admin_reset_logs);
        // collected_logs.extend(migrated_logs);
        // collected_logs.extend(rent_logs);
        // collected_logs.extend(set_max_units_logs);
        // collected_logs.extend(deprecation_timestamp_logs);

        Ok(collected_logs)
    }

    async fn collect_timestamps(
        &self,
        logs: &Vec<Log>,
    ) -> Result<HashMap<H256, u32>, Box<dyn Error>> {
        let unique_block_hashes: HashSet<H256> =
            logs.iter().filter_map(|log| log.block_hash).collect();
        let mut timestamps = HashMap::new();

        let timestamp_futures: Vec<_> = unique_block_hashes
            .iter()
            .map(|hash| {
                let provider = self.provider.clone();
                async move {
                    let timestamp = get_block_timestamp(provider, *hash).await;
                    (*hash, timestamp.unwrap())
                }
            })
            .collect();

        let start_time = std::time::Instant::now();
        // This will typically be slower the more timestamps there are due to RPC rate limits
        for result in join_all(timestamp_futures).await {
            timestamps.insert(result.0, result.1);
        }
        log::info!(
            "Awaiting and nserting timestamps took: {:?}",
            start_time.elapsed()
        );

        Ok(timestamps)
    }

    async fn sync_register_logs(
        &mut self,
        logs: &Vec<Log>,
        timestamps_map: &HashMap<H256, u32>,
    ) -> Result<(), Box<dyn Error>> {
        let register_logs: Vec<&Log> = logs
            .iter()
            .filter(|log| {
                log.topics
                    .contains(&get_signature_topic(REGISTER_SIGNATURE))
            })
            .collect();

        let mut timestamps = Vec::new();
        for log in &register_logs {
            if let Some(block_hash) = log.block_hash {
                if let Some(timestamp) = timestamps_map.get(&block_hash) {
                    timestamps.push(*timestamp);
                }
            }
        }

        self.id_registry
            .persist_many_register_logs(&self.store, register_logs, self.chain_id, &timestamps)
            .await
            .unwrap();

        Ok(())
    }

    async fn sync_transfer_logs(
        &mut self,
        logs: &Vec<Log>,
        timestamps_map: &HashMap<H256, u32>,
    ) -> Result<(), Box<dyn Error>> {
        let transfer_logs: Vec<&Log> = logs
            .iter()
            .filter(|log| {
                log.topics
                    .contains(&get_signature_topic(TRANSFER_SIGNATURE))
            })
            .collect();

        let mut timestamps = Vec::new();
        for log in &transfer_logs {
            if let Some(block_hash) = log.block_hash {
                if let Some(timestamp) = timestamps_map.get(&block_hash) {
                    timestamps.push(*timestamp);
                }
            }
        }

        self.id_registry
            .persist_many_transfer_logs(&self.store, transfer_logs, self.chain_id, &timestamps)
            .await
            .unwrap();

        Ok(())
    }

    async fn sync_recovery_logs(
        &mut self,
        logs: &Vec<Log>,
        timestamps_map: &HashMap<H256, u32>,
    ) -> Result<(), Box<dyn Error>> {
        let recovery_logs: Vec<&Log> = logs
            .iter()
            .filter(|log| {
                log.topics
                    .contains(&get_signature_topic(RECOVERY_SIGNATURE))
            })
            .collect();

        let mut timestamps = Vec::new();
        for log in &recovery_logs {
            if let Some(block_hash) = log.block_hash {
                if let Some(timestamp) = timestamps_map.get(&block_hash) {
                    timestamps.push(*timestamp);
                }
            }
        }

        self.id_registry
            .persist_many_recovery_logs(&self.store, recovery_logs, self.chain_id, &timestamps)
            .await
            .unwrap();

        Ok(())
    }

    async fn sync_change_recovery_address_logs(
        &mut self,
        logs: &Vec<Log>,
        timestamps_map: &HashMap<H256, u32>,
    ) -> Result<(), Box<dyn Error>> {
        let change_recovery_address_logs: Vec<&Log> = logs
            .iter()
            .filter(|log| {
                log.topics
                    .contains(&get_signature_topic(CHANGE_RECOVERY_ADDRESS_SIGNATURE))
            })
            .collect();

        let mut timestamps = Vec::new();
        for log in &change_recovery_address_logs {
            if let Some(block_hash) = log.block_hash {
                if let Some(timestamp) = timestamps_map.get(&block_hash) {
                    timestamps.push(*timestamp);
                }
            }
        }

        self.id_registry
            .persist_many_change_recovery_address_logs(
                &self.store,
                change_recovery_address_logs,
                self.chain_id,
                &timestamps,
            )
            .await
            .unwrap();

        Ok(())
    }

    async fn sync_add_logs(&mut self, start: u64, end: u64) -> Result<(), Box<dyn Error>> {
        let add_logs = self.key_registry.get_add_logs(start, end).await?;
        for log in add_logs {
            let block_hash = log.block_hash.unwrap();
            let timestamp = self.get_block_timestamp(block_hash).await?;

            self.key_registry
                .persist_add_log(&self.store, &log, self.chain_id, timestamp as i64)
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
                .persist_remove_log(&self.store, &log, self.chain_id, timestamp as i64)
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
                .persist_admin_reset_log(&self.store, &log, self.chain_id, timestamp as i64)
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
                .persist_migrated_log(&self.store, &log, self.chain_id, timestamp as i64)
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
                .persist_rent_log(&self.store, &log, self.chain_id, timestamp as i64)
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
                .persist_set_max_units_log(&self.store, &log, self.chain_id, timestamp as i64)
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
                .persist_deprecation_timestamp_log(
                    &self.store,
                    &log,
                    self.chain_id,
                    timestamp as i64,
                )
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
            let percent_complete = (current_block - FARCASTER_START_BLOCK) as f64
                / (end_block - FARCASTER_START_BLOCK) as f64;
            // Adding a progress bar
            let bar_width = 50;
            let progress = (percent_complete * bar_width as f64).round() as usize;
            let bar: String = "=".repeat(progress) + ">" + &" ".repeat(bar_width - progress - 1);
            log::info!("Syncing [{}] {:.2}%", bar, percent_complete * 100.0);

            // Clear block timestamp cache to avoid overloading it with useless data
            self.block_timestamp_cache.clear();

            let start = current_block;
            let end = current_block + BLOCK_INTERVAL;

            let collected_logs = self.collect_logs(start, end).await?;
            let timestamps = self.collect_timestamps(&collected_logs).await?;

            println!("Collected {} logs", collected_logs.len());
            println!("Collected {} timestamps", timestamps.iter().len());

            // id registry logs
            self.sync_register_logs(&collected_logs, &timestamps)
                .await?;
            self.sync_transfer_logs(&collected_logs, &timestamps)
                .await?;
            self.sync_recovery_logs(&collected_logs, &timestamps)
                .await?;
            self.sync_change_recovery_address_logs(&collected_logs, &timestamps)
                .await?;

            // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            // log::info!("synced recovery logs");
            // log::info!("synced change recovery address logs");

            // // key registry logs
            // self.sync_add_logs(start, end).await?;
            // log::info!("synced add logs");
            // self.sync_remove_logs(start, end).await?;
            // log::info!("synced remove logs");
            // self.sync_admin_reset_logs(start, end).await?;
            // log::info!("synced admin reset logs");
            // self.sync_migrated_logs(start, end).await?;
            // log::info!("synced migrated logs");

            // // storage registry logs
            // self.sync_rent_logs(start, end).await?;
            // log::info!("synced rent logs");
            // self.sync_set_max_units_logs(start, end).await?;
            // log::info!("synced set max units logs");
            // self.sync_deprecation_timestamp_logs(start, end).await?;
            // log::info!("synced deprecation timestamp logs");

            current_block = end + 1;
        }
        Ok(())
    }
}
