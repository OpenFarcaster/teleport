use ethers::{
    contract::{parse_log, Contract, ContractInstance, EthEvent},
    core::utils::keccak256,
    providers::{JsonRpcClient, Middleware, Provider},
    types::{Address, Filter, Log, H256, U256},
};

use std::error::Error;
use std::sync::Arc;
use teleport_common::protobufs::generated::{
    on_chain_event, IdRegisterEventBody, IdRegisterEventType, OnChainEvent, OnChainEventType,
};
use teleport_storage::db::{self};
use teleport_storage::Store;

use crate::utils::read_abi;

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
pub struct IdRegistry<T> {
    store: Store,
    provider: Provider<T>,
    contract: ContractInstance<Arc<Provider<T>>, Provider<T>>,
}

impl<T: JsonRpcClient + Clone> IdRegistry<T> {
    pub fn new(
        provider: Provider<T>,
        store: Store,
        contract_addr: String,
        abi_path: String,
    ) -> Result<Self, Box<dyn Error>> {
        let contract_abi = read_abi(abi_path)?;
        let addr: Address = contract_addr.parse().unwrap();
        let contract = Contract::new(addr, contract_abi, Arc::new(provider.clone()));

        Ok(IdRegistry {
            store,
            provider,
            contract,
        })
    }

    pub async fn get_register_logs(
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

    pub async fn persist_register_log(
        &self,
        log: Log,
        chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::core::types::Bytes;
    use ethers::core::types::Log;
    use ethers::core::types::H160;
    use hex::FromHex;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_get_register_logs() {
        let store = Store::new("sqlite::memory:".to_string()).await;
        let (provider, mock) = Provider::mocked();
        mock.push(Log {
            address: H160::from_str("0x0").unwrap(),
            topics: vec![],
            data: Bytes::from_hex("0x0").unwrap(),
            block_hash: None,
            block_number: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        })
        .expect("pushing mock log");

        let id_registry = IdRegistry::new(
            provider.clone(),
            store.clone(),
            "0x0".to_string(),
            "abi.json".to_string(),
        );
        let logs = id_registry
            .unwrap()
            .get_register_logs(0, 100000000)
            .await
            .unwrap();
        println!("{:?}", logs);
    }
}
