use crate::utils::{get_logs, get_signature_topic, read_abi};
use ethers::{
    contract::{parse_log, Contract as EthContract, ContractInstance, EthEvent},
    providers::{JsonRpcClient, Provider},
    types::{Address, Filter, Log, U256},
};
use log::{error, warn};
use sqlx::Acquire;
use std::error::Error;
use std::sync::Arc;
use teleport_common::protobufs::generated::{
    on_chain_event, IdRegisterEventBody, IdRegisterEventType, OnChainEvent, OnChainEventType,
};
use teleport_storage::db::{self, ChainEventRow, FidRecoveryUpdate, FidRow, FidTransfer};
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
pub struct Contract<T> {
    provider: Provider<T>,
    inner: ContractInstance<Arc<Provider<T>>, Provider<T>>,
}

pub const TRANSFER_SIGNATURE: &str = "Transfer(address,address,uint256)";
pub const REGISTER_SIGNATURE: &str = "Register(address,uint256,address)";
pub const RECOVERY_SIGNATURE: &str = "Recover(address,address,uint256)";
pub const CHANGE_RECOVERY_ADDRESS_SIGNATURE: &str = "ChangeRecoveryAddress(uint256,address)";

impl<T: JsonRpcClient + Clone> Contract<T> {
    pub fn new(
        provider: Provider<T>,
        contract_addr: String,
        abi_path: String,
    ) -> Result<Self, Box<dyn Error>> {
        let contract_abi = read_abi(abi_path)?;
        let addr: Address = contract_addr.parse()?;
        let contract = EthContract::new(addr, contract_abi, Arc::new(provider.clone()));

        Ok(Contract {
            provider,
            inner: contract,
        })
    }

    pub async fn get_id_registry_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(vec![
                get_signature_topic(REGISTER_SIGNATURE),
                get_signature_topic(TRANSFER_SIGNATURE),
                get_signature_topic(RECOVERY_SIGNATURE),
                get_signature_topic(CHANGE_RECOVERY_ADDRESS_SIGNATURE),
            ]);

        let all_logs = get_logs(self.provider.clone(), &filter).await?;

        Ok(all_logs)
    }

    pub async fn process_id_registry_logs(
        &self,
        store: &Store,
        logs: Vec<Log>,
        timestamps: Vec<u32>,
        chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let mut chain_events: Vec<db::ChainEventRow> = vec![];
        let mut fids: Vec<db::FidRow> = vec![];
        let mut fid_transfers: Vec<db::FidTransfer> = vec![];
        let mut fid_recovery_updated: Vec<db::FidRecoveryUpdate> = vec![];

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

            if log.topics[0] == get_signature_topic(REGISTER_SIGNATURE) {
                let (chain_events_row, fid_row) =
                    match self.process_register_log(log, timestamp, chain_id) {
                        Ok((chain_events_row, fid_row)) => (chain_events_row, fid_row),
                        Err(e) => {
                            warn!("Failed to process Register log: {:?}", e);
                            continue;
                        }
                    };

                chain_events.push(chain_events_row);
                fids.push(fid_row);
            } else if log.topics[0] == get_signature_topic(TRANSFER_SIGNATURE) {
                let (chain_events_row, fid_transfer_row) =
                    match self.process_transfer_log(log, timestamp, chain_id) {
                        Ok((chain_events_row, fid_transfer_row)) => {
                            (chain_events_row, fid_transfer_row)
                        }
                        Err(e) => {
                            warn!("Failed to process Transfer log: {:?}", e);
                            continue;
                        }
                    };

                chain_events.push(chain_events_row);
                fid_transfers.push(fid_transfer_row);
            } else if log.topics[0] == get_signature_topic(CHANGE_RECOVERY_ADDRESS_SIGNATURE) {
                let (chain_events_row, fid_recovery_update_row) =
                    match self.process_change_recovery_address_log(log, timestamp, chain_id) {
                        Ok((chain_events_row, fid_recovery_update_row)) => {
                            (chain_events_row, fid_recovery_update_row)
                        }
                        Err(e) => {
                            warn!("Failed to process ChangeRecoveryAddress log: {:?}", e);
                            continue;
                        }
                    };

                chain_events.push(chain_events_row);
                fid_recovery_updated.push(fid_recovery_update_row);
            } else if log.topics[0] == get_signature_topic(RECOVERY_SIGNATURE) {
                let (chain_events_row, fid_transfer_row) =
                    match self.process_recovery_log(log, timestamp, chain_id) {
                        Ok((chain_events_row, fid_transfer_row)) => {
                            (chain_events_row, fid_transfer_row)
                        }
                        Err(e) => {
                            warn!("Failed to process Recover log: {:?}", e);
                            continue;
                        }
                    };

                chain_events.push(chain_events_row);
                fid_transfers.push(fid_transfer_row);
            }
        }

        let mut connection = store.conn.acquire().await?;
        let mut transaction = connection.begin().await?;

        let event_queries = db::ChainEventRow::generate_bulk_insert_queries(&chain_events)?;
        for event_query_str in event_queries {
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
        }

        let fids_queries = db::FidRow::generate_bulk_insert_queries(&fids)?;
        for fid_query_str in fids_queries {
            let fid_query = sqlx::query(&fid_query_str);
            let fid_query_result = fid_query.execute(&mut *transaction).await;
            match fid_query_result {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to insert fid row: {} {}", e, &fid_query_str);
                    transaction.rollback().await?;
                    return Err(Box::new(e));
                }
            }
        }

        let transfer_queries = db::FidRow::generate_bulk_transfer_queries(&fid_transfers)?;
        for transfer_query_str in transfer_queries {
            let transfer_query = sqlx::query(&transfer_query_str);
            let transfer_query_result = transfer_query.execute(&mut *transaction).await;
            match transfer_query_result {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        "Failed to conduct fid transfer: {} {}",
                        e, &transfer_query_str
                    );
                    transaction.rollback().await?;
                    return Err(Box::new(e));
                }
            }
        }

        let recovery_queries =
            db::FidRow::generate_bulk_update_recovery_address_queries(&fid_recovery_updated)?;
        for recovery_query_str in recovery_queries {
            let recovery_query = sqlx::query(&recovery_query_str);
            let recovery_query_result = recovery_query.execute(&mut *transaction).await;
            match recovery_query_result {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        "Failed to update fid recovery address: {} {}",
                        e, &recovery_query_str
                    );
                    transaction.rollback().await?;
                    return Err(Box::new(e));
                }
            }
        }

        transaction.commit().await?;

        Ok(())
    }

    fn process_register_log(
        &self,
        log: &Log,
        timestamp: u32,
        chain_id: u32,
    ) -> Result<(ChainEventRow, FidRow), Box<dyn Error>> {
        let parsed_log = match parse_log::<Register>(log.clone()) {
            Ok(parsed_log) => parsed_log,
            Err(e) => return Err(format!("Failed to parse Register event args: {:?}", e).into()),
        };

        let id_register_event_body = IdRegisterEventBody {
            event_type: IdRegisterEventType::Register as i32,
            to: parsed_log.to.as_bytes().to_vec(),
            from: vec![],
            recovery_address: parsed_log.recovery.as_bytes().to_vec(),
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeIdRegister as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.try_into()?,
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
            body: Some(on_chain_event::Body::IdRegisterEventBody(
                id_register_event_body,
            )),
        };

        let chain_events_row = db::ChainEventRow::new(&onchain_event, log.data.to_vec());
        let fid_row = db::FidRow {
            fid: parsed_log.id.try_into()?,
            registered_at: timestamp as i64 * 1000, // timestamp is in seconds
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            custody_address: parsed_log.to.to_fixed_bytes(),
            recovery_address: parsed_log.recovery.to_fixed_bytes(),
        };

        Ok((chain_events_row, fid_row))
    }

    fn process_transfer_log(
        &self,
        log: &Log,
        timestamp: u32,
        chain_id: u32,
    ) -> Result<(ChainEventRow, FidTransfer), Box<dyn Error>> {
        let parsed_log = match parse_log::<Transfer>(log.clone()) {
            Ok(parsed_log) => parsed_log,
            Err(e) => return Err(format!("Failed to parse Transfer event args: {:?}", e).into()),
        };

        let id_register_event_body = IdRegisterEventBody {
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
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.try_into()?,
            body: Some(on_chain_event::Body::IdRegisterEventBody(
                id_register_event_body,
            )),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let chain_events_row = db::ChainEventRow::new(&onchain_event, log.data.to_vec());
        let fid_transfer_row = db::FidTransfer {
            fid: parsed_log.id.try_into()?,
            custody_address: parsed_log.to.to_fixed_bytes(),
        };

        Ok((chain_events_row, fid_transfer_row))
    }

    fn process_change_recovery_address_log(
        &self,
        log: &Log,
        timestamp: u32,
        chain_id: u32,
    ) -> Result<(ChainEventRow, FidRecoveryUpdate), Box<dyn Error>> {
        let parsed_log = match parse_log::<ChangeRecoveryAddress>(log.clone()) {
            Ok(parsed_log) => parsed_log,
            Err(e) => {
                return Err(
                    format!("Failed to parse ChangeRecoveryAddress event args: {:?}", e).into(),
                )
            }
        };

        let id_register_event_body = IdRegisterEventBody {
            to: vec![],
            from: vec![],
            event_type: IdRegisterEventType::ChangeRecovery as i32,
            recovery_address: parsed_log.recovery.as_bytes().to_vec(),
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeIdRegister as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.try_into()?,
            body: Some(on_chain_event::Body::IdRegisterEventBody(
                id_register_event_body,
            )),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let chain_events_row = db::ChainEventRow::new(&onchain_event, log.data.to_vec());
        let fid_recovery_update_row = db::FidRecoveryUpdate {
            fid: parsed_log.id.try_into()?,
            recovery_address: parsed_log.recovery.to_fixed_bytes(),
        };

        Ok((chain_events_row, fid_recovery_update_row))
    }

    fn process_recovery_log(
        &self,
        log: &Log,
        timestamp: u32,
        chain_id: u32,
    ) -> Result<(ChainEventRow, FidTransfer), Box<dyn Error>> {
        let parsed_log = match parse_log::<Recover>(log.clone()) {
            Ok(parsed_log) => parsed_log,
            Err(e) => return Err(format!("Failed to parse Recover event args: {:?}", e).into()),
        };

        let id_register_event_body = IdRegisterEventBody {
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
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.try_into()?,
            body: Some(on_chain_event::Body::IdRegisterEventBody(
                id_register_event_body,
            )),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let chain_events_row = db::ChainEventRow::new(&onchain_event, log.data.to_vec());
        let fid_transfer_row = db::FidTransfer {
            fid: parsed_log.id.try_into()?,
            custody_address: parsed_log.to.to_fixed_bytes(),
        };

        Ok((chain_events_row, fid_transfer_row))
    }
}
