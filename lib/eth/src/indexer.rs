use crate::id_registry;
use crate::key_registry;
use crate::storage_registry;
use crate::utils::get_block_timestamp;

use ethers::{
    providers::{JsonRpcClient, Middleware, Provider},
    types::{BlockNumber, Log, H256},
};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error};
use std::{
    cmp::min,
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};
use teleport_common::state::PersistentState;
use teleport_storage::Store;
use tokio;

const FARCASTER_START_BLOCK: u64 = 108864739u64;

struct CollectedLogs {
    id_registry: Vec<Log>,
    key_registry: Vec<Log>,
    storage_registry: Vec<Log>,
}

pub struct Indexer<T> {
    store: Store,
    state: Arc<Mutex<PersistentState>>,
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
        state: Arc<Mutex<PersistentState>>,
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
            state,
            provider,
            id_registry,
            key_registry,
            storage_registry,
            chain_id,
            block_timestamp_cache: HashMap::new(),
        })
    }

    pub async fn get_start_block(&self) -> u64 {
        let last_synced_block = self.state.lock().unwrap().last_synced_block;

        if last_synced_block == 0 {
            FARCASTER_START_BLOCK
        } else {
            last_synced_block + 1
        }
    }

    pub async fn get_latest_block(&self) -> Result<u64, Box<dyn Error>> {
        let latest_block = self.provider.get_block(BlockNumber::Latest).await?.unwrap();
        Ok(latest_block.number.unwrap().as_u64())
    }

    async fn get_all_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<CollectedLogs, Box<dyn Error>> {
        let id_registry_logs = self
            .id_registry
            .get_id_registry_logs(start_block, end_block);
        let key_registry_logs = self
            .key_registry
            .get_key_registry_logs(start_block, end_block);
        let storage_registry_logs = self
            .storage_registry
            .get_storage_registry_logs(start_block, end_block);

        let (id_registry_logs, key_registry_logs, storage_registry_logs) =
            tokio::try_join!(id_registry_logs, key_registry_logs, storage_registry_logs)?;

        Ok(CollectedLogs {
            id_registry: id_registry_logs,
            key_registry: key_registry_logs,
            storage_registry: storage_registry_logs,
        })
    }

    async fn fetch_event_timestamps(&mut self, events: Vec<Log>) -> (Vec<Log>, Vec<u32>) {
        let mut timestamps = Vec::new();

        for event in events.iter() {
            if let Some(block_hash) = event.block_hash {
                if let Some(timestamp) = self.block_timestamp_cache.get(&block_hash) {
                    timestamps.push(*timestamp);
                } else {
                    // Fetch timestamp from provider if not in cache
                    let timestamp = get_block_timestamp(self.provider.clone(), block_hash)
                        .await
                        .unwrap();
                    self.block_timestamp_cache.insert(block_hash, timestamp);
                    timestamps.push(timestamp);
                }
            }
        }

        (events, timestamps)
    }

    pub async fn subscribe(
        &mut self,
        start_block: u64,
        interval_in_secs: u64,
        sync_block_range_size: u64,
    ) -> Result<(), Box<dyn Error>> {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(interval_in_secs));
        let mut current_block = start_block;
        loop {
            interval.tick().await;
            let latest_block = self.get_latest_block().await?;
            self.sync(current_block, latest_block, sync_block_range_size)
                .await
                .unwrap();
            current_block = latest_block + 1;
        }
    }

    pub async fn sync(
        &mut self,
        start_block: u64,
        end_block: u64,
        sync_block_range_size: u64,
    ) -> Result<(), Box<dyn Error>> {
        let mut current_block = start_block;
        let pb = ProgressBar::new(end_block - FARCASTER_START_BLOCK);
        pb.set_style(
            ProgressStyle::with_template(
                "Syncing Blocks: [{elapsed_precise}] [{percent_precise}%] [{wide_bar:.cyan/blue}] {msg} (ETA: {eta_precise})",
            )
            .unwrap()
            .progress_chars("#>-"),
        );

        while current_block <= end_block {
            pb.set_position(current_block - FARCASTER_START_BLOCK);
            pb.set_message(format!("{}/{}", current_block, end_block));

            let start = current_block;
            let end = current_block + min(sync_block_range_size, end_block - current_block);

            let collected_logs = self.get_all_logs(start, end).await?;

            if collected_logs.storage_registry.len() > 0 {
                debug!(
                    "Found {} logs from the storage registry for current block range",
                    collected_logs.storage_registry.len()
                );

                let (storage_registry_logs, timestamps) = self
                    .fetch_event_timestamps(collected_logs.storage_registry)
                    .await;

                let result = self
                    .storage_registry
                    .process_storage_registry_logs(
                        &self.store,
                        storage_registry_logs,
                        timestamps,
                        self.chain_id,
                    )
                    .await;

                match result {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error processing storage registry logs: {}", e);
                    }
                }
            }

            if collected_logs.id_registry.len() > 0 {
                debug!(
                    "Found {} logs from the id registry for current block range",
                    collected_logs.id_registry.len()
                );

                let (id_registry_logs, timestamps) = self
                    .fetch_event_timestamps(collected_logs.id_registry)
                    .await;

                let result = self
                    .id_registry
                    .process_id_registry_logs(
                        &self.store,
                        id_registry_logs,
                        timestamps,
                        self.chain_id,
                    )
                    .await;

                match result {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error processing id registry logs: {}", e);
                    }
                }
            }

            if collected_logs.key_registry.len() > 0 {
                debug!(
                    "Found {} logs from the key registry for current block range",
                    collected_logs.key_registry.len()
                );

                let (key_registry_logs, timestamps) = self
                    .fetch_event_timestamps(collected_logs.key_registry)
                    .await;

                let result = self
                    .key_registry
                    .process_key_registry_logs(
                        &self.store,
                        key_registry_logs,
                        timestamps,
                        self.chain_id,
                    )
                    .await;

                match result {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error processing key registry logs: {}", e);
                    }
                }
            }

            self.block_timestamp_cache.clear();

            self.state.lock().unwrap().last_synced_block = end;
            self.state.lock().unwrap().store();

            current_block = end + 1;
        }

        pb.finish_with_message("Synced!");
        Ok(())
    }
}
