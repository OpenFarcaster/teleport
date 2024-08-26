use crate::utils::{get_logs, get_signature_topic, read_abi};
use ethers::{
    contract::{parse_log, Contract as EthContract, ContractInstance, EthEvent},
    providers::{JsonRpcClient, Provider},
    types::{Address, Filter, Log, U256},
};
use sqlx::Acquire;
use std::error::Error;
use std::sync::Arc;
use teleport_common::protobufs::generated::{
    on_chain_event, OnChainEvent, OnChainEventType, StorageRentEventBody,
};
use teleport_storage::db::{self};
use teleport_storage::Store;

pub const RENT_SIGNATURE: &str = "Rent(address,uint256,uint256)";
pub const SET_MAX_UNITS_SIGNATURE: &str = "SetMaxUnits(uint256,uint256)";
pub const SET_DEPRECATION_TIMESTAMP_SIGNATURE: &str = "SetDeprecationTimestamp(uint256,uint256)";

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
pub struct Contract<T> {
    provider: Arc<Provider<T>>,
    inner: ContractInstance<Arc<Provider<T>>, Provider<T>>,
}

impl<T: JsonRpcClient + Clone> Contract<T> {
    pub fn new(
        provider: Arc<Provider<T>>,
        contract_addr: String,
        abi_path: String,
    ) -> Result<Self, Box<dyn Error>> {
        let contract_abi = read_abi(abi_path)?;
        let addr: Address = contract_addr.parse().unwrap();
        let contract = EthContract::new(addr, contract_abi, provider.clone());

        Ok(Contract {
            provider,
            inner: contract,
        })
    }

    pub async fn get_rent_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(get_signature_topic(RENT_SIGNATURE));
        let logs = get_logs(self.provider.clone(), &filter).await?;

        Ok(logs)
    }

    pub async fn process_rent_log(
        &self,
        log: &Log,
        chain_id: u32,
        timestamp: u32,
    ) -> Result<(db::StorageAllocationRow, db::ChainEventRow), Box<dyn Error>> {
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
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid,
            body: Some(on_chain_event::Body::StorageRentEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        let storage_allocation =
            db::StorageAllocationRow::new(0, expiry, event_row.id.clone(), fid, units, payer);

        Ok((storage_allocation, event_row))
    }

    pub async fn persist_rent_log(
        &self,
        store: &Store,
        log: &Log,
        chain_id: u32,
        timestamp: u32,
    ) -> Result<(), Box<dyn Error>> {
        let (storage_allocation, event_row) =
            self.process_rent_log(log, chain_id, timestamp).await?;

        event_row.insert(&store).await?;
        storage_allocation.insert(&store).await?;

        Ok(())
    }

    pub async fn persist_many_rent_logs(
        &self,
        store: &Store,
        logs: Vec<Log>,
        chain_id: u32,
        timestamps: &[u32],
    ) -> Result<(), Box<dyn Error>> {
        let mut storage_allocations = Vec::new();
        let mut event_rows = Vec::new();

        for (log, timestamp) in logs.iter().zip(timestamps.iter()) {
            let (storage_allocation, event_row) =
                self.process_rent_log(log, chain_id, *timestamp).await?;
            storage_allocations.push(storage_allocation);
            event_rows.push(event_row);
        }

        let mut connection = store.conn.acquire().await?;
        let mut transaction = connection.begin().await?;

        let event_queries = db::ChainEventRow::generate_bulk_insert_queries(&event_rows)?;
        for query in event_queries {
            let query = sqlx::query(&query);
            query.execute(&mut *transaction).await?;
        }

        let allocation_queries =
            db::StorageAllocationRow::generate_bulk_insert_queries(&storage_allocations)?;
        for query in allocation_queries {
            let query = sqlx::query(&query);
            query.execute(&mut *transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    pub async fn get_set_max_units_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(get_signature_topic(SET_MAX_UNITS_SIGNATURE));
        let logs = get_logs(self.provider.clone(), &filter).await?;

        Ok(logs)
    }

    pub async fn process_set_max_units_log(
        &self,
        log: &Log,
        _chain_id: u32,
        _timestamp: u32,
    ) -> Result<(), Box<dyn Error>> {
        let parsed_log: SetMaxUnits = parse_log(log.clone()).unwrap();
        let _old_max = parsed_log.oldMax.as_u32();
        let _new_max = parsed_log.newMax.as_u32();

        // TODO: Return proper rows to be stored

        Ok(())
    }

    pub async fn persist_set_max_units_log(
        &self,
        _store: &Store,
        log: &Log,
        chain_id: u32,
        timestamp: u32,
    ) -> Result<(), Box<dyn Error>> {
        self.process_set_max_units_log(log, chain_id, timestamp);

        Ok(())
    }

    pub async fn persist_many_set_max_units_logs(
        &self,
        _store: &Store,
        logs: Vec<Log>,
        chain_id: u32,
        timestamps: &[u32],
    ) -> Result<(), Box<dyn Error>> {
        for (log, timestamp) in logs.iter().zip(timestamps.iter()) {
            self.process_set_max_units_log(log, chain_id, *timestamp)
                .await?;
        }

        // TODO: Store resulting rows in the database

        Ok(())
    }

    pub async fn get_deprecation_timestamp_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(get_signature_topic(SET_DEPRECATION_TIMESTAMP_SIGNATURE));
        let logs = get_logs(self.provider.clone(), &filter).await?;

        Ok(logs)
    }

    async fn process_deprecation_timestamp_log(&self, log: &Log) -> Result<(), Box<dyn Error>> {
        let parsed_log: SetDeprecationTimestamp = parse_log(log.clone()).unwrap();
        let _old_timestamp = parsed_log.oldTimestamp.as_u32();
        let _new_timestamp = parsed_log.newTimestamp.as_u32();

        // TODO: Return proper rows to be stored

        Ok(())
    }

    pub async fn persist_deprecation_timestamp_log(
        &self,
        _store: &Store,
        log: &Log,
        _chain_id: u32,
        _timestamp: i64,
    ) -> Result<(), Box<dyn Error>> {
        self.process_deprecation_timestamp_log(log).await?;

        // TODO: store deprecation timestamp in db

        Ok(())
    }

    pub async fn persist_many_deprecation_timestamp_logs(
        &self,
        _store: &Store,
        logs: Vec<Log>,
        _chain_id: u32,
        _timestamps: &[u32],
    ) -> Result<(), Box<dyn Error>> {
        for log in logs {
            self.process_deprecation_timestamp_log(&log).await?;
        }

        // TODO: store deprecation timestamps in db

        Ok(())
    }
}
