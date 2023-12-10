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
use serde_json::json;

use teleport_common::protobufs::generated::{IdRegisterEventType, OnChainEventType};
use teleport_storage::db::{self};
use teleport_storage::Store;

#[derive(Debug, Clone, EthEvent)]
pub struct Register {
    #[ethevent(indexed)]
    pub to: Address,
    #[ethevent(indexed)]
    pub id: U256,
    pub recovery: Address,
}

#[derive(Debug, Clone, EthEvent)]
pub struct Transfer {
    #[ethevent(indexed)]
    pub from: Address,
    #[ethevent(indexed)]
    pub to: Address,
    #[ethevent(indexed)]
    pub id: U256,
}

#[derive(Debug, Clone, EthEvent)]
pub struct Recover {
    #[ethevent(indexed)]
    pub from: Address,
    #[ethevent(indexed)]
    pub to: Address,
    #[ethevent(indexed)]
    pub id: U256,
}

#[derive(Debug, Clone, EthEvent)]
pub struct ChangeRecoveryAddress {
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

    async fn persist_register_log(&self, log: Log, chain_id: u64) -> Result<(), Box<dyn Error>> {
        let parsed_log: Register = parse_log(log.clone()).unwrap();
        let event = Register {
            to: parsed_log.to,
            id: parsed_log.id,
            recovery: parsed_log.recovery,
        };

        let fid = event.id.as_u64();
        let body = json!({
            "to": event.to.to_string(),
            "id": event.id.as_u64(),
            "recovery": event.recovery.to_string(),
            "eventType": IdRegisterEventType::Register as u64,
        });

        let event_row = db::ChainEventRow::new(
            fid,
            chain_id,
            log.block_number.unwrap().as_u64(),
            log.transaction_index.unwrap().as_u64(),
            log.log_index.unwrap().as_u64(),
            OnChainEventType::EventTypeIdRegister as u64,
            log.block_hash.unwrap().to_fixed_bytes(),
            log.transaction_hash.unwrap().to_string(),
            body.to_string(),
            log.data.to_vec(),
        );

        event_row.insert(&self.store).await.unwrap();

        let fid_row = db::FidRow {
            fid: fid as i64,
            // TODO: there is no efficient way to get the timestamp from the block
            // without fetching the block itself in another RPC call
            registered_at: 0,
            chain_event_id: event_row.id,
            custody_address: event.to.to_fixed_bytes(),
            recovery_address: event.recovery.to_fixed_bytes(),
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
        chain_id: u64,
    ) -> Result<(), Box<dyn Error>> {
        let parsed_log: Transfer = parse_log(log.clone()).unwrap();
        let event = Transfer {
            from: parsed_log.from,
            to: parsed_log.to,
            id: parsed_log.id,
        };

        let fid = event.id.as_u64();
        let body = json!({
            "from": event.from.to_string(),
            "to": event.to.to_string(),
            "id": event.id.as_u64(),
            "eventType": IdRegisterEventType::Transfer as u64,
        });

        let event_row = db::ChainEventRow::new(
            fid,
            chain_id,
            log.block_number.unwrap().as_u64(),
            log.transaction_index.unwrap().as_u64(),
            log.log_index.unwrap().as_u64(),
            OnChainEventType::EventTypeIdRegister as u64,
            log.block_hash.unwrap().to_fixed_bytes(),
            log.transaction_hash.unwrap().to_string(),
            body.to_string(),
            log.data.to_vec(),
        );
        event_row.insert(&self.store).await.unwrap();

        db::FidRow::update_custody_address(
            &self.store,
            fid,
            event.from.to_fixed_bytes(),
            event.to.to_fixed_bytes(),
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
        chain_id: u64,
    ) -> Result<(), Box<dyn Error>> {
        let parsed_log: Recover = parse_log(log.clone()).unwrap();

        let event = Recover {
            from: parsed_log.from,
            to: parsed_log.to,
            id: parsed_log.id,
        };
        let fid = event.id.as_u64();
        let body = json!({
            "from": event.from.to_string(),
            "to": event.to.to_string(),
            "id": event.id.as_u64(),
            "eventType": IdRegisterEventType::ChangeRecovery as u64,
        });

        let event_row = db::ChainEventRow::new(
            fid,
            chain_id,
            log.block_number.unwrap().as_u64(),
            log.transaction_index.unwrap().as_u64(),
            log.log_index.unwrap().as_u64(),
            OnChainEventType::EventTypeIdRegister as u64,
            log.block_hash.unwrap().to_fixed_bytes(),
            log.transaction_hash.unwrap().to_string(),
            body.to_string(),
            log.data.to_vec(),
        );

        event_row.insert(&self.store).await.unwrap();

        db::FidRow::update_recovery_address(&self.store, fid, event.to.to_fixed_bytes())
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
        chain_id: u64,
    ) -> Result<(), Box<dyn Error>> {
        let parsed_log: ChangeRecoveryAddress = parse_log(log.clone()).unwrap();

        let event = ChangeRecoveryAddress {
            id: parsed_log.id,
            recovery: parsed_log.recovery,
        };
        let fid = event.id.as_u64();
        let body = json!({
            "id": event.id.as_u64(),
            "recovery": event.recovery.to_string(),
            "eventType": IdRegisterEventType::ChangeRecovery as u64,
        });

        let event_row = db::ChainEventRow::new(
            fid,
            chain_id,
            log.block_number.unwrap().as_u64(),
            log.transaction_index.unwrap().as_u64(),
            log.log_index.unwrap().as_u64(),
            OnChainEventType::EventTypeIdRegister as u64,
            log.block_hash.unwrap().to_fixed_bytes(),
            log.transaction_hash.unwrap().to_string(),
            body.to_string(),
            log.data.to_vec(),
        );

        event_row.insert(&self.store).await.unwrap();

        db::FidRow::update_recovery_address(&self.store, fid, event.recovery.to_fixed_bytes())
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
            println!("event sync progress: {:.2}%", percent_complete * 100.0);
            let start = current_block_num;
            let end = current_block_num + 2000;

            let register_logs = self.get_register_logs(start, end).await.unwrap();
            for log in register_logs {
                // persist the log
                self.persist_register_log(log.clone(), chain_id.as_u64())
                    .await
                    .unwrap();
            }

            let transfer_logs = self.get_transfer_logs(start, end).await.unwrap();
            for log in transfer_logs {
                self.persist_transfer_log(log.clone(), chain_id.as_u64())
                    .await
                    .unwrap();
            }

            let recovery_logs = self.get_recovery_logs(start, end).await.unwrap();
            for log in recovery_logs {
                self.persist_recovery_log(log.clone(), chain_id.as_u64())
                    .await
                    .unwrap();
            }

            current_block_num = end + 1;
        }
        Ok(())
    }
}
