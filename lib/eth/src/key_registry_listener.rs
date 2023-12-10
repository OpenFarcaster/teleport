use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use ethers::{
    abi::Abi,
    contract::{Contract, EthEvent},
    core::utils::keccak256,
    prelude::*,
    providers::{Http, Provider},
};
use serde_json;
use teleport_common::protobufs::{
    self,
    generated::{
        on_chain_event, OnChainEvent, OnChainEventType, SignerEventBody, SignerEventType,
        SignerMigratedEventBody,
    },
};
use teleport_storage::db::{self};
use teleport_storage::Store;

#[derive(Debug, Clone, EthEvent, serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)]
struct Add {
    #[ethevent(indexed)]
    pub fid: U256,
    #[ethevent(indexed)]
    pub keyType: u32,
    #[ethevent(indexed)]
    pub key: Bytes,
    pub keyBytes: Bytes,
    pub metadataType: u8,
    pub metadata: Bytes,
}

#[derive(Debug, Clone, EthEvent)]
#[allow(non_snake_case)]
struct Remove {
    #[ethevent(indexed)]
    pub fid: U256,
    #[ethevent(indexed)]
    pub key: Bytes,
    pub keyBytes: Bytes,
}

#[derive(Debug, Clone, EthEvent)]
#[allow(non_snake_case)]
struct AdminReset {
    #[ethevent(indexed)]
    pub fid: U256,
    #[ethevent(indexed)]
    pub key: Bytes,
    pub keyBytes: Bytes,
}

#[derive(Debug, Clone, EthEvent)]
#[allow(non_snake_case)]
struct Migrated {
    #[ethevent(indexed)]
    pub keysMigratedAt: U256,
}

#[derive(Debug, Clone)]
pub struct KeyRegistryListener {
    pub store: Store,
    pub provider: Provider<Http>,
    pub contract: ContractInstance<Arc<Provider<Http>>, Provider<Http>>,
}

const KEY_REGISTRY_DEPLOYMENT_BLOCK: u64 = 108869032u64;

impl KeyRegistryListener {
    pub fn new(
        http_rpc_url: String,
        store: Store,
        contract_addr: String,
        abi_path: String,
    ) -> Result<Self, Box<dyn Error>> {
        let http_provider = Provider::<Http>::try_from(http_rpc_url)?
            .interval(std::time::Duration::from_millis(2000));
        let p = http_provider.clone();
        let contract_abi = Self::read_abi(abi_path)?;
        let addr: Address = contract_addr.parse().unwrap();
        let contract = Contract::new(addr, contract_abi, Arc::new(http_provider));

        Ok(KeyRegistryListener {
            store,
            provider: p,
            contract,
        })
    }

    fn read_abi(path: String) -> Result<Abi, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let contract_abi: Abi = serde_json::from_reader(reader)?;
        Ok(contract_abi)
    }

    async fn get_add_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "Add(uin256,uint32,bytes,bytes,uint8,bytes)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.contract.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    async fn persist_add_log(&self, log: Log, chain_id: u32) -> Result<(), Box<dyn Error>> {
        let parsed_log: Add = parse_log(log.clone()).unwrap();
        let body = SignerEventBody {
            key: parsed_log.key.to_vec(),
            key_type: parsed_log.keyType,
            event_type: SignerEventType::Add as i32,
            metadata: parsed_log.metadata.to_vec(),
            metadata_type: parsed_log.metadataType as u32,
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeSigner as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.fid.as_u64(),
            body: Some(on_chain_event::Body::SignerEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        event_row.insert(&self.store).await.unwrap();

        // TODO: MORE LOGIC

        Ok(())
    }

    async fn get_remove_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "Remove(uint256,bytes,bytes)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.contract.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    async fn persist_remove_log(&self, log: Log, chain_id: u32) -> Result<(), Box<dyn Error>> {
        let parsed_log: Remove = parse_log(log.clone()).unwrap();
        let body = SignerEventBody {
            key: parsed_log.key.to_vec(),
            key_type: 0,
            event_type: SignerEventType::Remove as i32,
            metadata: vec![],
            metadata_type: 0,
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeSigner as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.fid.as_u64(),
            body: Some(on_chain_event::Body::SignerEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        event_row.insert(&self.store).await.unwrap();

        // TODO: MORE LOGIC

        Ok(())
    }

    async fn get_admin_reset_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "AdminReset(uint256,bytes,bytes)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.contract.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    async fn persist_admin_reset_log(&self, log: Log, chain_id: u32) -> Result<(), Box<dyn Error>> {
        let parsed_log: AdminReset = parse_log(log.clone()).unwrap();
        let body = SignerEventBody {
            key: parsed_log.key.to_vec(),
            key_type: 0,
            event_type: SignerEventType::AdminReset as i32,
            metadata: vec![],
            metadata_type: 0,
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeSigner as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.fid.as_u64(),
            body: Some(on_chain_event::Body::SignerEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        event_row.insert(&self.store).await.unwrap();

        // TODO: MORE LOGIC

        Ok(())
    }

    async fn get_migrated_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "Migrated(uint256)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.contract.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    async fn persist_migrated_log(&self, log: Log, chain_id: u32) -> Result<(), Box<dyn Error>> {
        let parsed_log: Migrated = parse_log(log.clone()).unwrap();
        let body = SignerMigratedEventBody {
            migrated_at: parsed_log.keysMigratedAt.as_u64() as u32,
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeSignerMigrated as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: 0,
            body: Some(on_chain_event::Body::SignerMigratedEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        event_row.insert(&self.store).await.unwrap();

        // TODO: MORE LOGIC

        Ok(())
    }

    pub async fn sync(&self) -> Result<(), Box<dyn Error>> {
        let max_block_num = db::ChainEventRow::max_block_number(&self.store)
            .await
            .unwrap_or(KEY_REGISTRY_DEPLOYMENT_BLOCK as i64);

        let start_block_num = if max_block_num == 0 {
            KEY_REGISTRY_DEPLOYMENT_BLOCK
        } else {
            max_block_num as u64 + 1
        };

        let chain_id = self.provider.get_chainid().await.unwrap();
        let mut current_block_num = start_block_num;
        let latest_block = self
            .provider
            .get_block(BlockNumber::Latest)
            .await
            .unwrap()
            .unwrap();
        let latest_block_number = latest_block.number.unwrap().as_u64();
        println!(
            "start block: {:?}, latest block : {:?}",
            current_block_num, latest_block_number
        );

        while current_block_num <= latest_block_number {
            // fetch logs in range [current_block_num, current_block_num + 1000]
            let percent_complete = (current_block_num - start_block_num) as f64
                / (latest_block_number - start_block_num) as f64;
            println!(
                "Key Registry Events: sync progress = {:.2}%",
                percent_complete * 100.0
            );
            let start = current_block_num;
            let end = current_block_num + 2000;

            let add_logs = self.get_add_logs(start, end).await.unwrap();
            for log in add_logs {
                // persist the log
                self.persist_add_log(log.clone(), chain_id.as_u64() as u32)
                    .await
                    .unwrap();
            }

            let remove_logs = self.get_remove_logs(start, end).await.unwrap();
            for log in remove_logs {
                self.persist_remove_log(log.clone(), chain_id.as_u64() as u32)
                    .await
                    .unwrap();
            }

            let admin_reset_logs = self.get_admin_reset_logs(start, end).await.unwrap();
            for log in admin_reset_logs {
                self.persist_admin_reset_log(log.clone(), chain_id.as_u64() as u32)
                    .await
                    .unwrap();
            }

            let migrated_logs = self.get_migrated_logs(start, end).await.unwrap();
            for log in migrated_logs {
                self.persist_migrated_log(log.clone(), chain_id.as_u64() as u32)
                    .await
                    .unwrap();
            }

            current_block_num = end + 1;
        }

        Ok(())
    }
}
