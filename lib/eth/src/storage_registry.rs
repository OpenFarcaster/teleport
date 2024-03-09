use crate::utils::{get_logs, get_signature_topic, read_abi};
use ethers::{
    contract::{parse_log, Contract as EthContract, ContractInstance, EthEvent},
    providers::{JsonRpcClient, Provider},
    types::{Address, Filter, Log, U256},
};
use log::{error, info, warn};
use sqlx::Acquire;
use std::sync::Arc;
use std::{error::Error, iter::zip};
use teleport_common::{
    protobufs::generated::{on_chain_event, OnChainEvent, OnChainEventType, StorageRentEventBody},
    time::block_timestamp_to_farcaster_time,
};
use teleport_storage::db::{self, ChainEventRow, StorageAllocationRow};
use teleport_storage::Store;

pub const RENT_SIGNATURE: &str = "Rent(address,uint256,uint256)";
const RENT_EXPIRY_IN_SECONDS: u32 = 365 * 24 * 60 * 60; // One year

#[derive(Debug, Clone, EthEvent)]
struct Rent {
    #[ethevent(indexed)]
    pub payer: Address,
    #[ethevent(indexed)]
    pub fid: U256,
    pub units: U256,
}

#[derive(Debug, Clone)]
pub struct Contract<T> {
    provider: Provider<T>,
    inner: ContractInstance<Arc<Provider<T>>, Provider<T>>,
}

impl<T: JsonRpcClient + Clone> Contract<T> {
    pub fn new(
        provider: Provider<T>,
        contract_addr: String,
        abi_path: String,
    ) -> Result<Self, Box<dyn Error>> {
        let contract_abi = read_abi(abi_path)?;
        let addr: Address = contract_addr.parse().unwrap();
        let contract = EthContract::new(addr, contract_abi, Arc::new(provider.clone()));

        Ok(Contract {
            provider,
            inner: contract,
        })
    }

    pub async fn get_storage_registry_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(get_signature_topic(RENT_SIGNATURE));

        let all_logs = get_logs(self.provider.clone(), &filter).await?;

        Ok(all_logs)
    }

    pub async fn process_storage_registry_logs(
        &self,
        store: &Store,
        logs: Vec<Log>,
        timestamps: Vec<u32>,
        chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let mut chain_events: Vec<db::ChainEventRow> = vec![];
        let mut storage_allocations: Vec<db::StorageAllocationRow> = vec![];

        for (i, log) in logs.iter().enumerate() {
            if log.block_hash.is_none()
                || log.block_number.is_none()
                || log.transaction_hash.is_none()
                || log.transaction_index.is_none()
                || log.log_index.is_none()
                || log.removed.is_some_and(|removed| removed)
            {
                continue;
            }

            let timestamp = timestamps[i];

            if log.topics[0] == get_signature_topic(RENT_SIGNATURE) {
                let (chain_events_row, storage_allocations_row) =
                    match self.process_rent_log(log, timestamp, chain_id) {
                        Ok((chain_events_row, storage_allocations_row)) => {
                            (chain_events_row, storage_allocations_row)
                        }
                        Err(e) => {
                            warn!("Failed to process Rent log: {:?}", e);
                            continue;
                        }
                    };

                chain_events.push(chain_events_row);
                storage_allocations.push(storage_allocations_row);
            }
        }

        let mut connection = store.conn.acquire().await?;
        let mut transaction = connection.begin().await?;

        let event_queries = db::ChainEventRow::generate_bulk_insert_queries(&chain_events)?;
        let allocation_queries =
            db::StorageAllocationRow::generate_bulk_insert_queries(&storage_allocations)?;

        for (event_query_str, allocation_query_str) in zip(event_queries, allocation_queries) {
            let event_query = sqlx::query(&event_query_str);
            let event_query_result = event_query.execute(&mut *transaction).await;
            match event_query_result {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        "Failed to insert chain event row: {} {}",
                        e, &event_query_str
                    );
                    transaction.rollback().await?;
                    return Err(Box::new(e));
                }
            }

            let allocation_query = sqlx::query(&allocation_query_str);
            let allocation_query_result = allocation_query.execute(&mut *transaction).await;
            match allocation_query_result {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        "Failed to insert storage allocation row: {}\n {}\n {}",
                        e, &event_query_str, &allocation_query_str
                    );
                    transaction.rollback().await?;
                    return Err(Box::new(e));
                }
            }
        }

        transaction.commit().await?;

        Ok(())
    }

    fn process_rent_log(
        &self,
        log: &Log,
        timestamp: u32,
        chain_id: u32,
    ) -> Result<(ChainEventRow, StorageAllocationRow), Box<dyn Error>> {
        let parsed_log = match parse_log::<Rent>(log.clone()) {
            Ok(parsed_log) => parsed_log,
            Err(e) => return Err(format!("Failed to parse Rent event args: {:?}", e).into()),
        };

        let timestamp_as_fc_time = match block_timestamp_to_farcaster_time(timestamp) {
            Ok(timestamp_as_fc_time) => timestamp_as_fc_time,
            Err(e) => {
                return Err(
                    format!("Failed to parse block timestamp: {:?} {:?}", timestamp, e).into(),
                )
            }
        };

        let expiry = timestamp_as_fc_time + RENT_EXPIRY_IN_SECONDS;
        let storage_rent_event_body = StorageRentEventBody {
            payer: parsed_log.payer.as_bytes().to_vec(),
            units: parsed_log.units.as_u32(),
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
            fid: parsed_log.fid.try_into()?,
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
            body: Some(on_chain_event::Body::StorageRentEventBody(
                storage_rent_event_body,
            )),
        };

        let chain_events_row = db::ChainEventRow::new(&onchain_event, log.data.to_vec());
        let storage_allocations_row = db::StorageAllocationRow::new(
            timestamp_as_fc_time.into(),
            expiry,
            log.transaction_hash.unwrap().as_bytes().to_vec(),
            log.log_index.unwrap().as_u32(),
            parsed_log.fid.try_into()?,
            parsed_log.units.as_u32(),
            parsed_log.payer.as_bytes().to_vec(),
        );

        Ok((chain_events_row, storage_allocations_row))
    }
}
