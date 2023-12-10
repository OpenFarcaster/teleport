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

use teleport_common::protobufs::generated::{
    on_chain_event, IdRegisterEventBody, IdRegisterEventType, OnChainEvent, OnChainEventType,
};
use teleport_storage::db::{self};
use teleport_storage::Store;

#[derive(Debug, Clone, EthEvent)]
struct Register {
    #[ethevent(indexed)]
    pub to: Address,
    #[ethevent(indexed)]
    pub id: U256,
    pub recovery: Address,
}
#[derive(Debug, Clone, EthEvent)]
struct Transfer {
    #[ethevent(indexed)]
    pub from: Address,
    #[ethevent(indexed)]
    pub to: Address,
    #[ethevent(indexed)]
    pub id: U256,
}

#[derive(Debug, Clone, EthEvent)]
struct Recover {
    #[ethevent(indexed)]
    pub from: Address,
    #[ethevent(indexed)]
    pub to: Address,
    #[ethevent(indexed)]
    pub id: U256,
}

#[derive(Debug, Clone, EthEvent)]
struct ChangeRecoveryAddress {
    #[ethevent(indexed)]
    pub id: U256,
    #[ethevent(indexed)]
    pub recovery: Address,
}

#[derive(Debug, Clone)]
pub struct IdRegistryListener {
    pub store: Store,
    pub provider: Provider<Http>,
    pub contract: ContractInstance<Arc<Provider<Http>>, Provider<Http>>,
}

// Is this right? IdRegistry seems to be deployed at 108869029u64
const FARCASTER_START_BLOCK: u64 = 108864739u64;

impl IdRegistryListener {
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

        Ok(IdRegistryListener {
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

    async fn get_register_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "Register(address,uint256,address)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.contract.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    async fn persist_register_log(&self, log: Log, chain_id: u32) -> Result<(), Box<dyn Error>> {
        let parsed_log: Register = parse_log(log.clone()).unwrap();

        let body = IdRegisterEventBody {
            to: parsed_log.to.to_fixed_bytes().to_vec(),
            event_type: IdRegisterEventType::Register as i32,
            from: vec![],
            recovery_address: parsed_log.recovery.as_bytes().to_vec(),
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeIdRegister as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.as_u64(),
            body: Some(on_chain_event::Body::IdRegisterEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        event_row.insert(&self.store).await.unwrap();

        let fid_row = db::FidRow {
            fid: parsed_log.id.as_u64() as i64,
            // TODO: there is no efficient way to get the timestamp from the block
            // without fetching the block itself in another RPC call
            registered_at: 0,
            chain_event_id: event_row.id,
            custody_address: parsed_log.to.to_fixed_bytes(),
            recovery_address: parsed_log.recovery.to_fixed_bytes(),
        };
        fid_row.insert(&self.store).await.unwrap();
        Ok(())
    }

    pub async fn get_transfer_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "Transfer(address,address,uint256)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.contract.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;
        Ok(logs)
    }

    pub async fn persist_transfer_log(
        &self,
        log: Log,
        chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let parsed_log: Transfer = parse_log(log.clone()).unwrap();

        let body = IdRegisterEventBody {
            to: parsed_log.to.to_fixed_bytes().to_vec(),
            from: parsed_log.from.to_fixed_bytes().to_vec(),
            event_type: IdRegisterEventType::Transfer as i32,
            recovery_address: vec![],
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeIdRegister as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.as_u64(),
            body: Some(on_chain_event::Body::IdRegisterEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        event_row.insert(&self.store).await.unwrap();

        db::FidRow::update_custody_address(
            &self.store,
            parsed_log.id.as_u64(),
            parsed_log.to.to_fixed_bytes(),
        )
        .await
        .unwrap();

        Ok(())
    }

    pub async fn get_recovery_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "Recover(address,address,uint256)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.contract.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;
        Ok(logs)
    }

    pub async fn persist_recovery_log(
        &self,
        log: Log,
        chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let parsed_log: Recover = parse_log(log.clone()).unwrap();

        let body = IdRegisterEventBody {
            to: parsed_log.to.to_fixed_bytes().to_vec(),
            from: parsed_log.from.to_fixed_bytes().to_vec(),
            event_type: IdRegisterEventType::Transfer as i32,
            recovery_address: vec![],
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeIdRegister as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.as_u64(),
            body: Some(on_chain_event::Body::IdRegisterEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        event_row.insert(&self.store).await.unwrap();

        db::FidRow::update_recovery_address(
            &self.store,
            parsed_log.id.as_u64(),
            parsed_log.to.to_fixed_bytes(),
        )
        .await
        .unwrap();

        Ok(())
    }

    pub async fn get_change_recovery_address_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "ChangeRecoveryAddress(uint256,address)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.contract.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    pub async fn persist_change_recovery_address_log(
        &self,
        log: Log,
        chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let parsed_log: ChangeRecoveryAddress = parse_log(log.clone()).unwrap();

        let body = IdRegisterEventBody {
            to: vec![],
            from: vec![],
            event_type: IdRegisterEventType::Transfer as i32,
            recovery_address: parsed_log.recovery.as_bytes().to_vec(),
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeIdRegister as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.as_u64(),
            body: Some(on_chain_event::Body::IdRegisterEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        event_row.insert(&self.store).await.unwrap();

        db::FidRow::update_recovery_address(
            &self.store,
            parsed_log.id.as_u64(),
            parsed_log.recovery.to_fixed_bytes(),
        )
        .await
        .unwrap();

        Ok(())
    }

    pub async fn sync(&self) -> Result<(), Box<dyn Error>> {
        let max_block_num = db::ChainEventRow::max_block_number(&self.store)
            .await
            .unwrap_or(FARCASTER_START_BLOCK as i64);

        let start_block_num = if max_block_num == 0 {
            FARCASTER_START_BLOCK
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
                "ID Registry Events: sync progress = {:.2}%",
                percent_complete * 100.0
            );
            let start = current_block_num;
            let end = current_block_num + 2000;

            let register_logs = self.get_register_logs(start, end).await.unwrap();
            for log in register_logs {
                // persist the log
                self.persist_register_log(log.clone(), chain_id.as_u64() as u32)
                    .await
                    .unwrap();
            }

            let transfer_logs = self.get_transfer_logs(start, end).await.unwrap();
            for log in transfer_logs {
                self.persist_transfer_log(log.clone(), chain_id.as_u64() as u32)
                    .await
                    .unwrap();
            }

            let recovery_logs = self.get_recovery_logs(start, end).await.unwrap();
            for log in recovery_logs {
                self.persist_recovery_log(log.clone(), chain_id.as_u64() as u32)
                    .await
                    .unwrap();
            }

            current_block_num = end + 1;
        }
        Ok(())
    }
}
