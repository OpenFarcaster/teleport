use crate::utils::read_abi;
use alloy_dyn_abi::DynSolType;
use ethers::{
    contract::{parse_log, Contract as EthContract, ContractInstance, EthEvent},
    core::utils::keccak256,
    providers::{JsonRpcClient, Middleware, Provider},
    types::{Address, Bytes, Filter, Log, H256, U256},
};
use log;
use serde::{Deserialize, Serialize};
use serde_json::{self};
use sqlx::Row;
use std::error::Error;
use std::sync::Arc;
use teleport_common::protobufs::generated::{
    on_chain_event, OnChainEvent, OnChainEventType, SignerEventBody, SignerEventType,
    SignerMigratedEventBody,
};
use teleport_storage::db::{self};
use teleport_storage::Store;

#[derive(Debug, Clone, EthEvent)]
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
struct SignerRequestMetadata {
    pub request_fid: u64,
    pub request_signer: Vec<u8>,
    pub signature: Vec<u8>,
    pub deadline: u64,
}

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

    pub async fn get_add_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "Add(uint256,uint32,bytes,bytes,uint8,bytes)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
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

    pub async fn persist_add_log(
        &self,
        store: &Store,
        log: &Log,
        chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let fid = U256::from_big_endian(log.topics[1].as_bytes()).as_u64();
        let key_type = U256::from_big_endian(log.topics[2].as_bytes()).as_u32();
        let key_hash = H256::from_slice(&log.topics[3].as_bytes());

        log::info!(
            "got Add log for key hash: {:?} in tx: {:?}",
            key_hash,
            log.transaction_hash
        );

        let key = H256::from_slice(&log.data[128..160]); // 160
        let key_bytes = key.as_bytes();

        // validate that keyBytes is an EdDSA pub key and keyType == 1
        assert_eq!(key_bytes.len(), 32, "key is not 32 bytes long");

        let metadata_type = log.data[190]; // 190
        let signer_request = Contract::<T>::decode_metadata(&log);
        let metadata_json = serde_json::to_string(&signer_request).unwrap();
        let metadata = metadata_json.to_string().as_bytes().to_vec();
        let body = SignerEventBody {
            key: key_bytes.to_vec(),
            key_type,
            event_type: SignerEventType::Add as i32,
            metadata,
            metadata_type: metadata_type as u32,
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeSigner as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid,
            body: Some(on_chain_event::Body::SignerEventBody(body.clone())),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        let event_id = event_row.insert(&store).await?;

        // store keyBytes in db
        let signer = db::SignerRow::new(
            fid,
            signer_request.request_fid,
            event_id,
            None,
            key_type as i16,
            metadata_type as i16,
            key_bytes.to_vec(),
            metadata_json.to_string(),
        );
        let result = signer.insert(&store).await;

        match &result {
            Err(sqlx::error::Error::Database(e)) if e.is_unique_violation() => {
                println!("signer already exists, skipping");
            }
            _ => {
                result?;
            }
        }

        Ok(())
    }

    pub async fn get_remove_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "Remove(uint256,bytes,bytes)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    /// Hubs listen for this, validate that keyType == 1 and keyBytes exists in db.
    /// keyBytes is marked as removed, messages signed by keyBytes with `fid` are invalid (todo).
    pub async fn persist_remove_log(
        &self,
        store: &Store,
        log: &Log,
        chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let fid = U256::from_big_endian(log.topics[1].as_bytes());
        let key_hash = Address::from(log.topics[2]);
        log::info!(
            "got Remove log for key hash: {:? } in tx: {:?}",
            key_hash,
            log.transaction_hash
        );

        // last 32 bytes of data is the keyBytes
        let key_bytes = log.data.chunks(32).last().unwrap();

        // get signer from db
        let signer = db::SignerRow::get_by_key(&store, key_bytes.to_vec()).await?;
        let key_type: u32 = signer.get("key_type");
        let metadata: Vec<u8> = signer.get("metadata");
        let body = SignerEventBody {
            key: key_bytes.to_vec(),
            key_type,
            event_type: SignerEventType::Remove.into(),
            metadata,
            metadata_type: 1u32,
        };

        // validate that keyType == 1
        assert_eq!(key_type, 1, "key type is not 1");

        // store event in db
        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeSigner as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: fid.as_u64(),
            body: Some(on_chain_event::Body::SignerEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        let event_id = event_row.insert(&store).await.unwrap();

        db::SignerRow::update_remove_chain_event(&store, key_bytes.to_vec(), event_id).await?;

        Ok(())
    }

    pub async fn get_admin_reset_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "AdminReset(uint256,bytes,bytes)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    // validate that keyType == 1 and that keyBytes exists in db.
    // these keyBytes is no longer tracked, messages signed by keyBytes with `fid` are invalid,
    // dropped immediately and not accepted (todo)
    pub async fn persist_admin_reset_log(
        &self,
        store: &Store,
        log: &Log,
        chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let fid = U256::from_big_endian(log.topics[1].as_bytes());
        let key_hash = Address::from(log.topics[2]);
        log::info!(
            "got Admin Reset log for key hash: {:? } in tx: {:?}",
            key_hash,
            log.transaction_hash
        );

        // last 32 bytes of data is the keyBytes
        let key_bytes = log.data.chunks(32).last().unwrap();

        // get signer from db
        let signer = db::SignerRow::get_by_key(&store, key_bytes.to_vec()).await?;
        let key_type: u32 = signer.get("key_type");
        assert_eq!(key_type, 1, "key type is not 1");

        let metadata: Vec<u8> = signer.get("metadata");
        let body = SignerEventBody {
            key: key_bytes.to_vec(),
            key_type,
            event_type: SignerEventType::AdminReset.into(),
            metadata,
            metadata_type: 1u32,
        };

        let onchain_event = OnChainEvent {
            r#type: OnChainEventType::EventTypeSigner as i32,
            chain_id,
            block_number: log.block_number.unwrap().as_u32(),
            block_hash: log.block_hash.unwrap().to_fixed_bytes().to_vec(),
            block_timestamp: 0,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: fid.as_u64(),
            body: Some(on_chain_event::Body::SignerEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        event_row.insert(&store).await?;

        // TODO: invalidate keyBytes and messages signed by these keyBytes

        Ok(())
    }

    pub async fn get_migrated_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let event_signature = "Migrated(uint256)";
        let topic = H256::from_slice(&keccak256(event_signature));
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(topic);
        let logs = self.provider.get_logs(&filter).await?;

        Ok(logs)
    }

    pub async fn persist_migrated_log(
        &self,
        store: &Store,
        log: &Log,
        chain_id: u32,
    ) -> Result<(), Box<dyn Error>> {
        let parsed_log: Migrated = parse_log(log.clone()).unwrap();
        let body = SignerMigratedEventBody {
            migrated_at: parsed_log.keysMigratedAt.as_u64() as u32,
        };

        log::info!("got Migrated log in tx: {:?}", log.transaction_hash);

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
        event_row.insert(&store).await?;

        /*
        TODO
        1. Stop accepting Farcaster Signer messages with a timestamp >= keysMigratedAt.
        2. After the grace period (24 hours), stop accepting all Farcaster Signer messages.
        3. Drop any messages created by off-chain Farcaster Signers whose pub key was not emitted as an Add event.
        */

        Ok(())
    }
}
