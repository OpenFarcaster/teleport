use crate::id_registry;
use crate::id_registry::{
    CHANGE_RECOVERY_ADDRESS_SIGNATURE, RECOVERY_SIGNATURE, REGISTER_SIGNATURE, TRANSFER_SIGNATURE,
};
use crate::key_registry::{
    self, ADD_SIGNER_SIGNATURE, ADMIN_RESET_SIGNATURE, MIGRATED_SIGNATURE, REMOVE_SIGNER_SIGNATURE,
};
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
// Number of blocks to read at a time (max 2000)
const BLOCK_INTERVAL: u64 = 2000;

struct CollectedLogs {
    register: Vec<Log>,
    transfer: Vec<Log>,
    recovery: Vec<Log>,
    change_recovery_address: Vec<Log>,
    add: Vec<Log>,
    remove: Vec<Log>,
    admin_reset: Vec<Log>,
    migrated: Vec<Log>,
}

pub struct Indexer<T> {
    store: Store,
    provider: Provider<T>,
    chain_id: u32,
    id_registry: id_registry::Contract<T>,
    key_registry: key_registry::Contract<T>,
    storage_registry: storage_registry::Contract<T>,
    block_timestamp_cache: HashMap<H256, u32>,
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

        Ok(Indexer {
            store,
            provider,
            id_registry,
            key_registry,
            storage_registry,
            chain_id,
            block_timestamp_cache: HashMap::new(),
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

        let timestamp = get_block_timestamp(self.provider.clone(), block_hash).await?;
        self.block_timestamp_cache.insert(block_hash, timestamp);
        Ok(timestamp)
    }

    async fn collect_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<CollectedLogs, Box<dyn Error>> {
        let register_future = self.id_registry.get_register_logs(start_block, end_block);
        let transfer_future = self.id_registry.get_transfer_logs(start_block, end_block);
        let recovery_future = self.id_registry.get_recovery_logs(start_block, end_block);
        let change_recovery_address_future = self
            .id_registry
            .get_change_recovery_address_logs(start_block, end_block);
        let add_future = self.key_registry.get_add_logs(start_block, end_block);
        let remove_future = self.key_registry.get_remove_logs(start_block, end_block);
        let admin_reset_future = self
            .key_registry
            .get_admin_reset_logs(start_block, end_block);
        let migrated_future = self.key_registry.get_migrated_logs(start_block, end_block);
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
            add_logs,
            remove_logs,
            admin_reset_logs,
            migrated_logs,
            // rent_logs,
            // set_max_units_logs,
            // deprecation_timestamp_logs,
        ) = tokio::try_join!(
            register_future,
            transfer_future,
            recovery_future,
            change_recovery_address_future,
            add_future,
            remove_future,
            admin_reset_future,
            migrated_future,
            // rent_future,
            // set_max_units_future,
            // deprecation_timestamp_future
        )?;

        Ok(CollectedLogs {
            register: register_logs,
            transfer: transfer_logs,
            recovery: recovery_logs,
            change_recovery_address: change_recovery_address_logs,
            add: add_logs,
            remove: remove_logs,
            admin_reset: admin_reset_logs,
            migrated: migrated_logs,
        })
    }

    async fn fetch_event_timestamps(&mut self, events: Vec<Log>) -> (Vec<Log>, Vec<u32>) {
        let futures: Vec<_> = events
            .iter()
            .filter_map(|log| log.block_hash)
            .map(|block_hash| {
                let provider = self.provider.clone(); // Clone provider to move into the async block
                let block_timestamp_cache = &self.block_timestamp_cache; // Borrow block_timestamp_cache mutably
                async move {
                    if let Some(timestamp) = block_timestamp_cache.get(&block_hash) {
                        return Ok::<u32, Box<dyn Error>>(*timestamp);
                    }
                    let timestamp = get_block_timestamp(provider, block_hash).await;
                    timestamp
                }
            })
            .collect();

        let timestamps: Vec<u32> = futures::future::join_all(futures)
            .await
            .into_iter()
            .filter_map(Result::ok)
            .collect();

        for (log, timestamp) in events.iter().zip(timestamps.iter()) {
            if let Some(block_hash) = log.block_hash {
                self.block_timestamp_cache.insert(block_hash, *timestamp);
            }
        }

        (events, timestamps)
    }

    async fn sync_register_logs(&mut self, register_logs: Vec<Log>) -> Result<(), Box<dyn Error>> {
        let fetch_start = std::time::Instant::now();
        let (register_logs, timestamps) = self.fetch_event_timestamps(register_logs).await;
        let fetch_duration = fetch_start.elapsed();
        println!("Fetching event timestamps took {:?}", fetch_duration);

        let persist_start = std::time::Instant::now();
        self.id_registry
            .persist_many_register_logs(&self.store, register_logs, self.chain_id, &timestamps)
            .await
            .unwrap();
        let persist_duration = persist_start.elapsed();
        println!("Persisting register logs took {:?}", persist_duration);

        Ok(())
    }

    async fn sync_transfer_logs(&mut self, transfer_logs: Vec<Log>) -> Result<(), Box<dyn Error>> {
        let (transfer_logs, timestamps) = self.fetch_event_timestamps(transfer_logs).await;

        self.id_registry
            .persist_many_transfer_logs(&self.store, transfer_logs, self.chain_id, &timestamps)
            .await
            .unwrap();

        Ok(())
    }

    async fn sync_recovery_logs(&mut self, recovery_logs: Vec<Log>) -> Result<(), Box<dyn Error>> {
        let (recovery_logs, timestamps) = self.fetch_event_timestamps(recovery_logs).await;

        self.id_registry
            .persist_many_recovery_logs(&self.store, recovery_logs, self.chain_id, &timestamps)
            .await
            .unwrap();

        Ok(())
    }

    async fn sync_change_recovery_address_logs(
        &mut self,
        change_recovery_address_logs: Vec<Log>,
    ) -> Result<(), Box<dyn Error>> {
        let (change_recovery_address_logs, timestamps) = self
            .fetch_event_timestamps(change_recovery_address_logs)
            .await;

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

    async fn sync_add_logs(&mut self, add_logs: Vec<Log>) -> Result<(), Box<dyn Error>> {
        let (add_logs, timestamps) = self.fetch_event_timestamps(add_logs).await;

        self.key_registry
            .persist_many_add_logs(&self.store, add_logs, self.chain_id, &timestamps)
            .await
            .unwrap();

        Ok(())
    }

    async fn sync_remove_logs(&mut self, remove_logs: Vec<Log>) -> Result<(), Box<dyn Error>> {
        let (remove_logs, timestamps) = self.fetch_event_timestamps(remove_logs).await;

        self.key_registry
            .persist_many_remove_logs(&self.store, remove_logs, self.chain_id, &timestamps)
            .await
            .unwrap();

        Ok(())
    }

    async fn sync_admin_reset_logs(
        &mut self,
        admin_reset_logs: Vec<Log>,
    ) -> Result<(), Box<dyn Error>> {
        let (admin_reset_logs, timestamps) = self.fetch_event_timestamps(admin_reset_logs).await;

        self.key_registry
            .persist_many_admin_reset_logs(
                &self.store,
                admin_reset_logs,
                self.chain_id,
                &timestamps,
            )
            .await
            .unwrap();

        Ok(())
    }

    async fn sync_migrated_logs(&mut self, migrated_logs: Vec<Log>) -> Result<(), Box<dyn Error>> {
        let (migrated_logs, timestamps) = self.fetch_event_timestamps(migrated_logs).await;

        self.key_registry
            .persist_many_migrated_logs(&self.store, migrated_logs, self.chain_id, &timestamps)
            .await
            .unwrap();

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
        let start_time = std::time::Instant::now();

        log::debug!(
            "syncing events from block {} to {}",
            current_block,
            end_block
        );

        // fetch logs in range [current_block_num, current_block_num + 2000]
        while current_block <= end_block {
            let percent_complete = (current_block - FARCASTER_START_BLOCK) as f64
                / (end_block - FARCASTER_START_BLOCK) as f64;

            let bar_width = 20;
            let progress = (percent_complete * bar_width as f64).round() as usize;
            let bar: String = "=".repeat(progress) + ">" + &" ".repeat(bar_width - progress - 1);

            let elapsed_time = start_time.elapsed().as_secs();
            let rate_of_progress = (current_block - start_block) as f64 / elapsed_time as f64; // blocks per second
            let total_blocks = (end_block - start_block) as f64;
            let estimated_total_time = total_blocks / rate_of_progress; // total estimated time in seconds
            let time_remaining = estimated_total_time - elapsed_time as f64; // remaining time in seconds

            log::info!(
                "Syncing [{}] {:.2}% | ~{:.2} seconds remaining",
                bar,
                percent_complete * 100.0,
                time_remaining
            );

            let start = current_block;
            let end = current_block + BLOCK_INTERVAL;

            let collect_logs_start = std::time::Instant::now();
            let collected_logs = self.collect_logs(start, end).await?;
            log::info!(
                "collect_logs took {} seconds",
                collect_logs_start.elapsed().as_secs()
            );

            // id registry logs
            let sync_start = std::time::Instant::now();
            self.sync_register_logs(collected_logs.register).await?;
            log::info!(
                "sync_register_logs took {} seconds",
                sync_start.elapsed().as_secs()
            );
            self.sync_transfer_logs(collected_logs.transfer).await?;
            self.sync_recovery_logs(collected_logs.recovery).await?;
            self.sync_change_recovery_address_logs(collected_logs.change_recovery_address)
                .await?;

            // key registry logs
            self.sync_add_logs(collected_logs.add).await?;
            self.sync_remove_logs(collected_logs.remove).await?;
            self.sync_admin_reset_logs(collected_logs.admin_reset)
                .await?;
            self.sync_migrated_logs(collected_logs.migrated).await?;

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
