use crate::utils::read_abi;
use ethers::{
    contract::{parse_log, Contract, ContractInstance, EthEvent},
    core::utils::keccak256,
    providers::{JsonRpcClient, Middleware, Provider},
    types::{Address, Filter, Log, H256, U256},
};
use std::error::Error;
use std::sync::Arc;
use teleport_common::protobufs::generated::{
    on_chain_event, OnChainEvent, OnChainEventType, StorageRentEventBody,
};
use teleport_storage::db::{self};
use teleport_storage::Store;

#[derive(Debug, Clone, EthEvent)]
struct Rent {
    #[ethevent(indexed)]
    pub payer: Address,
    #[ethevent(indexed)]
    pub fid: U256,
    pub units: U256,
}

#[derive(Debug, Clone, EthEvent)]
#[allow(non_snake_case)]
struct SetMaxUnits {
    pub oldMax: U256,
    pub newMax: U256,
}

#[derive(Debug, Clone, EthEvent)]
#[allow(non_snake_case)]
struct SetDeprecationTimestamp {
    pub oldTimestamp: U256,
    pub newTimestamp: U256,
}

#[derive(Debug, Clone)]
pub struct StorageRegistry<T> {
    store: Store,
    provider: Provider<T>,
    contract: ContractInstance<Arc<Provider<T>>, Provider<T>>,
}

impl<T: JsonRpcClient + Clone> StorageRegistry<T> {
    pub fn new(
        provider: Provider<T>,
        store: Store,
        contract_addr: String,
        abi_path: String,
    ) -> Result<Self, Box<dyn Error>> {
        let contract_abi = read_abi(abi_path)?;
        let addr: Address = contract_addr.parse().unwrap();
        let contract = Contract::new(addr, contract_abi, Arc::new(provider.clone()));

        Ok(StorageRegistry {
            store,
            provider,
            contract,
        })
    }

    pub async fn get_rent_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "Rent(address,uint256,uint256)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.contract.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    pub async fn persist_rent_log(&self, log: Log, chain_id: u32) -> Result<(), Box<dyn Error>> {
        let parsed_log: Rent = parse_log(log.clone()).unwrap();
        let units = parsed_log.units.as_u32();
        let expiry = parsed_log.units.as_u32() + 395 * 24 * 60 * 60;
        let fid = parsed_log.fid.as_u64();
        let payer = parsed_log.payer.as_bytes().to_vec();

        let body = StorageRentEventBody {
            payer: parsed_log.payer.as_bytes().to_vec(),
            units,
            expiry,
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeStorageRent as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: fid,
            body: Some(on_chain_event::Body::StorageRentEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        event_row.insert(&self.store).await.unwrap();

        // Insert in storage allocation table
        let storage_allocation =
            db::StorageAllocationRow::new(0, expiry, event_row.id, fid, units, payer);
        storage_allocation.insert(&self.store).await?;

        Ok(())
    }

    pub async fn get_set_max_units_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "SetMaxUnits(uint256,uint256)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.contract.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    pub async fn persist_set_max_units_log(
        &self,
        log: Log,
        _chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let parsed_log: SetMaxUnits = parse_log(log.clone()).unwrap();
        let _old_max = parsed_log.oldMax.as_u32();
        let _new_max = parsed_log.newMax.as_u32();

        // TODO: store max units in db

        Ok(())
    }

    pub async fn get_deprecation_timestamp_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "SetDeprecationTimestamp(uint256,uint256)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.contract.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    pub async fn persist_deprecation_timestamp_log(
        &self,
        log: Log,
        _chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let parsed_log: SetDeprecationTimestamp = parse_log(log.clone()).unwrap();
        let _old_timestamp = parsed_log.oldTimestamp.as_u32();
        let _new_timestamp = parsed_log.newTimestamp.as_u32();

        // TODO: store deprecation timestamp in db
        Ok(())
    }
}
