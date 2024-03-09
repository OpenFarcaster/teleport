use crate::utils::{get_logs, get_signature_topic, read_abi};
use alloy_dyn_abi::DynSolType;
use ethers::{
    contract::{parse_log, Contract as EthContract, ContractInstance, EthEvent},
    providers::{JsonRpcClient, Provider},
    types::{Address, Bytes, Filter, Log, H256, U256},
};
use log::{self, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{self};
use sqlx::Acquire;
use std::error::Error;
use std::sync::Arc;
use teleport_common::protobufs::generated::{
    on_chain_event, OnChainEvent, OnChainEventType, SignerEventBody, SignerEventType,
    SignerMigratedEventBody,
};
use teleport_storage::db::{self, ChainEventRow, SignerRemoved, SignerRow};
use teleport_storage::Store;

// Overriding the `signature` is required here due to a bug in ethers-rs that happens if we mention the `key` parameter as `Bytes`
// Since we have to use `H256` for `key`, the calculated signature doesn't match the actual signature for this event
#[derive(Debug, Clone, EthEvent)]
#[ethevent(signature = "0x7d285df41058466977811345cd453c0c52e8d841ffaabc74fc050f277ad4de02")]
#[allow(non_snake_case)]
struct Add {
    #[ethevent(indexed)]
    pub fid: U256,
    #[ethevent(indexed)]
    pub keyType: u32,
    #[ethevent(indexed)]
    pub key: H256,
    pub keyBytes: Bytes,
    pub metadataType: u8,
    pub metadata: Bytes,
}

// Overriding the `signature` is required here due to a bug in ethers-rs that happens if we mention the `key` parameter as `Bytes`
// Since we have to use `H256` for `key`, the calculated signature doesn't match the actual signature for this event
#[derive(Debug, Clone, EthEvent)]
#[ethevent(signature = "0x09e77066e0155f46785be12f6938a6b2e4be4381e59058129ce15f355cb96958")]
#[allow(non_snake_case)]
struct Remove {
    #[ethevent(indexed)]
    pub fid: U256,
    #[ethevent(indexed)]
    pub key: H256,
    pub keyBytes: Bytes,
}

// Overriding the `signature` is required here due to a bug in ethers-rs that happens if we mention the `key` parameter as `Bytes`
// Since we have to use `H256` for `key`, the calculated signature doesn't match the actual signature for this event
#[derive(Debug, Clone, EthEvent)]
#[ethevent(signature = "0x1ecc1009ebad5d2fb61239462f4f9f6f152662defe1845fc87f07d96bd1c60b4")]
#[allow(non_snake_case)]
struct AdminReset {
    #[ethevent(indexed)]
    pub fid: U256,
    #[ethevent(indexed)]
    pub key: H256,
    pub keyBytes: Bytes,
}

#[derive(Debug, Clone, EthEvent)]
#[allow(non_snake_case)]
struct Migrated {
    #[ethevent(indexed)]
    pub keysMigratedAt: U256,
}

#[derive(Debug, Clone)]
pub struct Contract<T> {
    provider: Provider<T>,
    inner: ContractInstance<Arc<Provider<T>>, Provider<T>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerRequestMetadata {
    pub request_fid: u64,
    pub request_signer: Vec<u8>,
    pub signature: Vec<u8>,
    pub deadline: u64,
}

pub const ADD_SIGNER_SIGNATURE: &str = "Add(uint256,uint32,bytes,bytes,uint8,bytes)";
pub const REMOVE_SIGNER_SIGNATURE: &str = "Remove(uint256,bytes,bytes)";
pub const ADMIN_RESET_SIGNATURE: &str = "AdminReset(uint256,bytes,bytes)";
pub const MIGRATED_SIGNATURE: &str = "Migrated(uint256)";

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

    pub async fn get_key_registry_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(vec![
                get_signature_topic(ADD_SIGNER_SIGNATURE),
                get_signature_topic(REMOVE_SIGNER_SIGNATURE),
                get_signature_topic(ADMIN_RESET_SIGNATURE),
                get_signature_topic(MIGRATED_SIGNATURE),
            ]);

        let all_logs = get_logs(self.provider.clone(), &filter).await?;

        Ok(all_logs)
    }

    pub async fn process_key_registry_logs(
        &self,
        store: &Store,
        logs: Vec<Log>,
        timestamps: Vec<u32>,
        chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let mut chain_events: Vec<db::ChainEventRow> = vec![];
        let mut signers: Vec<db::SignerRow> = vec![];
        let mut signer_removed_updates: Vec<db::SignerRemoved> = vec![];

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

            if log.topics[0] == get_signature_topic(ADD_SIGNER_SIGNATURE) {
                let (chain_events_row, signer_row) =
                    match self.process_add_log(log, timestamp, chain_id) {
                        Ok((chain_events_row, signer_row)) => (chain_events_row, signer_row),
                        Err(e) => {
                            warn!("Failed to process Add log: {}", e);
                            continue;
                        }
                    };

                chain_events.push(chain_events_row);
                signers.push(signer_row);
            } else if log.topics[0] == get_signature_topic(REMOVE_SIGNER_SIGNATURE) {
                let (chain_events_row, signer_removed_update) =
                    match self.process_remove_log(log, timestamp, chain_id) {
                        Ok((chain_events_row, signer_removed_update)) => {
                            (chain_events_row, signer_removed_update)
                        }
                        Err(e) => {
                            warn!("Failed to process Remove log: {}", e);
                            continue;
                        }
                    };

                chain_events.push(chain_events_row);
                signer_removed_updates.push(signer_removed_update);
            } else if log.topics[0] == get_signature_topic(ADMIN_RESET_SIGNATURE) {
                let (chain_events_row, signer_removed_update) =
                    match self.process_admin_reset_log(log, timestamp, chain_id) {
                        Ok((chain_events_row, signer_removed_update)) => {
                            (chain_events_row, signer_removed_update)
                        }
                        Err(e) => {
                            warn!("Failed to process AdminReset log: {}", e);
                            continue;
                        }
                    };

                chain_events.push(chain_events_row);
                signer_removed_updates.push(signer_removed_update);
            } else if log.topics[0] == get_signature_topic(MIGRATED_SIGNATURE) {
                let chain_events_row = match self.process_migrated_log(log, timestamp, chain_id) {
                    Ok(chain_events_row) => chain_events_row,
                    Err(e) => {
                        warn!("Failed to process Migrated log: {}", e);
                        continue;
                    }
                };

                chain_events.push(chain_events_row);
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

        let signer_queries = db::SignerRow::generate_bulk_insert_queries(&signers)?;
        for signer_query_str in signer_queries {
            let signer_query = sqlx::query(&signer_query_str);
            let signer_query_result = signer_query.execute(&mut *transaction).await;
            match signer_query_result {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to insert signer row: {} {}", e, &signer_query_str);
                    transaction.rollback().await?;
                    return Err(Box::new(e));
                }
            }
        }

        let signer_removed_queries =
            db::SignerRow::generate_bulk_remove_signer_queries(&signer_removed_updates)?;
        for signer_removed_query_str in signer_removed_queries {
            let signer_removed_query = sqlx::query(&signer_removed_query_str);
            let signer_removed_query_result = signer_removed_query.execute(&mut *transaction).await;
            match signer_removed_query_result {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        "Failed to mark signer row as removed: {} {}",
                        e, &signer_removed_query_str
                    );
                    transaction.rollback().await?;
                    return Err(Box::new(e));
                }
            }
        }

        transaction.commit().await?;

        Ok(())
    }

    fn process_add_log(
        &self,
        log: &Log,
        timestamp: u32,
        chain_id: u32,
    ) -> Result<(ChainEventRow, SignerRow), Box<dyn Error>> {
        let parsed_log = match parse_log::<Add>(log.clone()) {
            Ok(parsed_log) => parsed_log,
            Err(e) => return Err(format!("Failed to parse Add event args: {:?}", e).into()),
        };

        let signer_event_body = SignerEventBody {
            event_type: SignerEventType::Add as i32,
            key: parsed_log.keyBytes.to_vec(),
            key_type: parsed_log.keyType,
            metadata: parsed_log.metadata.to_vec(),
            metadata_type: parsed_log.metadataType as u32,
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeSigner as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.fid.try_into()?,
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
            body: Some(on_chain_event::Body::SignerEventBody(
                signer_event_body.clone(),
            )),
        };

        let signer_request = Contract::<T>::decode_metadata(log);
        let metadata_json = serde_json::to_string(&signer_request).unwrap();

        let chain_events_row = db::ChainEventRow::new(&onchain_event, log.data.to_vec());
        let signer_row = db::SignerRow::new(
            &signer_event_body,
            &onchain_event,
            signer_request.request_fid,
            metadata_json,
        );

        Ok((chain_events_row, signer_row))
    }

    fn process_remove_log(
        &self,
        log: &Log,
        timestamp: u32,
        chain_id: u32,
    ) -> Result<(ChainEventRow, SignerRemoved), Box<dyn Error>> {
        let parsed_log = match parse_log::<Remove>(log.clone()) {
            Ok(parsed_log) => parsed_log,
            Err(e) => return Err(format!("Failed to parse Remove event args: {:?}", e).into()),
        };

        let signer_event_body = SignerEventBody {
            event_type: SignerEventType::Remove as i32,
            key: parsed_log.keyBytes.to_vec(),
            key_type: 0,
            metadata: vec![],
            metadata_type: 0,
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeSigner as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.fid.try_into()?,
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
            body: Some(on_chain_event::Body::SignerEventBody(signer_event_body)),
        };

        let chain_events_row = db::ChainEventRow::new(&onchain_event, log.data.to_vec());
        let signer_removed = db::SignerRemoved {
            fid: parsed_log.fid.try_into()?,
            key: parsed_log.keyBytes.to_vec(),
            remove_transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            remove_log_index: log.log_index.unwrap().as_u32(),
            removed_at: onchain_event.block_timestamp * 1000,
        };

        Ok((chain_events_row, signer_removed))
    }

    fn process_admin_reset_log(
        &self,
        log: &Log,
        timestamp: u32,
        chain_id: u32,
    ) -> Result<(ChainEventRow, SignerRemoved), Box<dyn Error>> {
        let parsed_log = match parse_log::<AdminReset>(log.clone()) {
            Ok(parsed_log) => parsed_log,
            Err(e) => return Err(format!("Failed to parse AdminReset event args: {:?}", e).into()),
        };

        let signer_event_body = SignerEventBody {
            event_type: SignerEventType::AdminReset as i32,
            key: parsed_log.keyBytes.to_vec(),
            key_type: 0,
            metadata: vec![],
            metadata_type: 0,
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeSigner as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.fid.try_into()?,
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
            body: Some(on_chain_event::Body::SignerEventBody(signer_event_body)),
        };

        let chain_events_row = db::ChainEventRow::new(&onchain_event, log.data.to_vec());
        let signer_removed = db::SignerRemoved {
            fid: parsed_log.fid.try_into()?,
            key: parsed_log.keyBytes.to_vec(),
            remove_transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            remove_log_index: log.log_index.unwrap().as_u32(),
            removed_at: onchain_event.block_timestamp * 1000,
        };

        Ok((chain_events_row, signer_removed))
    }

    fn process_migrated_log(
        &self,
        log: &Log,
        timestamp: u32,
        chain_id: u32,
    ) -> Result<ChainEventRow, Box<dyn Error>> {
        let parsed_log = match parse_log::<Migrated>(log.clone()) {
            Ok(parsed_log) => parsed_log,
            Err(e) => return Err(format!("Failed to parse Migrated event args: {:?}", e).into()),
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeSignerMigrated as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: 0u64,
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
            body: Some(on_chain_event::Body::SignerMigratedEventBody(
                SignerMigratedEventBody {
                    migrated_at: parsed_log.keysMigratedAt.as_u32(),
                },
            )),
        };

        let chain_events_row = db::ChainEventRow::new(&onchain_event, log.data.to_vec());

        Ok(chain_events_row)
    }

    fn decode_metadata(log: &Log) -> SignerRequestMetadata {
        let metadata_abi = DynSolType::CustomStruct {
            name: "metadata".to_string(),
            prop_names: vec![
                "requestFid".to_string(),
                "requestSigner".to_string(),
                "signature".to_string(),
                "deadline".to_string(),
            ],
            tuple: vec![
                DynSolType::Uint(256),
                DynSolType::Address,
                DynSolType::Bytes,
                DynSolType::Uint(256),
            ],
        };
        let decoded = metadata_abi.abi_decode(&log.data[192..]).unwrap();
        let decoded_struct = decoded.as_custom_struct().unwrap();
        let values = decoded_struct.2;

        // extract fields from decoded struct
        let (requester_fid, _) = values[0].as_uint().unwrap();
        let request_signer = values[1].as_address().unwrap();
        let signature = values[2].as_bytes().unwrap();
        let (deadline, _) = values[3].as_uint().unwrap();

        // parse requester_fid as u64
        let requester_fid_int = requester_fid.to_string().parse::<u64>().unwrap();

        // parse deadline as u64
        let deadline_int = deadline.to_string().parse::<u64>().unwrap();

        SignerRequestMetadata {
            request_fid: requester_fid_int,
            request_signer: request_signer.to_vec(),
            signature: signature.to_vec(),
            deadline: deadline_int,
        }
    }
}
