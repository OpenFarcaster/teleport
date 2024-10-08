use crate::utils::{get_logs, get_signature_topic, read_abi};
use ethers::{
    contract::{parse_log, Contract as EthContract, ContractInstance, EthEvent},
    providers::{JsonRpcClient, Provider},
    types::{Address, Filter, Log, U256},
};
use sqlx::Acquire;
use std::error::Error;
use std::sync::Arc;
use teleport_config::Config;
use teleport_protobuf::protobufs::generated::{
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
pub struct Contract<T> {
    provider: Arc<Provider<T>>,
    inner: ContractInstance<Arc<Provider<T>>, Provider<T>>,
}

pub const TRANSFER_SIGNATURE: &str = "Transfer(address,address,uint256)";
pub const REGISTER_SIGNATURE: &str = "Register(address,uint256,address)";
pub const RECOVERY_SIGNATURE: &str = "Recover(address,address,uint256)";
pub const CHANGE_RECOVERY_ADDRESS_SIGNATURE: &str = "ChangeRecoveryAddress(uint256,address)";

impl<T: JsonRpcClient + Clone> Contract<T> {
    pub fn new(
        provider: Arc<Provider<T>>,
        contract_addr: String,
        abi_path: String,
    ) -> Result<Self, Box<dyn Error>> {
        let contract_abi = read_abi(abi_path)?;
        let addr: Address = contract_addr.parse()?;
        let contract = EthContract::new(addr, contract_abi, provider.clone());

        Ok(Contract {
            provider,
            inner: contract,
        })
    }

    pub async fn get_register_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(get_signature_topic(REGISTER_SIGNATURE));
        let logs = get_logs(self.provider.clone(), &filter).await?;

        Ok(logs)
    }

    async fn process_register_log(
        &self,
        log: &Log,
        chain_id: u32,
        timestamp: u32,
    ) -> Result<(db::FidRow, db::ChainEventRow), Box<dyn Error>> {
        let parsed_log: Register = parse_log(log.clone())?;

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
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.as_u64(),
            body: Some(on_chain_event::Body::IdRegisterEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());
        let fid_row = db::FidRow {
            fid: parsed_log.id.as_u64() as i64,
            registered_at: timestamp.into(),
            chain_event_id: event_row.id.clone(),
            custody_address: parsed_log.to.to_fixed_bytes(),
            recovery_address: parsed_log.recovery.to_fixed_bytes(),
        };

        Ok((fid_row, event_row))
    }

    pub async fn persist_register_log(
        &self,
        store: &Store,
        log: &Log,
        chain_id: u32,
        timestamp: u32,
    ) -> Result<(), Box<dyn Error>> {
        let (fid_row, event_row) = self.process_register_log(log, chain_id, timestamp).await?;

        // These must happen 1 at a time to satisfy foreign key constraints
        event_row.insert(store).await?;
        fid_row.insert(&store).await?;

        Ok(())
    }

    pub async fn persist_many_register_logs(
        &self,
        store: &Store,
        logs: Vec<Log>,
        chain_id: u32,
        timestamps: &[u32],
    ) -> Result<(), Box<dyn Error>> {
        let mut fid_rows = Vec::new();
        let mut event_rows = Vec::new();

        for (log, timestamp) in logs.iter().zip(timestamps.iter()) {
            let (fid_row, event_row) = self.process_register_log(log, chain_id, *timestamp).await?;
            fid_rows.push(fid_row);
            event_rows.push(event_row);
        }

        let mut connection = store.conn.acquire().await?;
        let mut transaction = connection.begin().await?;

        let queries = db::ChainEventRow::generate_bulk_insert_queries(&event_rows)?;
        for query in queries {
            let query = sqlx::query(&query);
            query.execute(&mut *transaction).await?;
        }

        let queries = db::FidRow::generate_bulk_insert_queries(&fid_rows)?;
        for query in queries {
            let query = sqlx::query(&query);
            query.execute(&mut *transaction).await?;
        }
        transaction.commit().await?;

        Ok(())
    }

    pub async fn get_transfer_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(get_signature_topic(TRANSFER_SIGNATURE));
        let logs = get_logs(self.provider.clone(), &filter).await?;
        Ok(logs)
    }

    pub async fn process_transfer_log(
        &self,
        log: &Log,
        chain_id: u32,
        timestamp: u32,
    ) -> Result<(db::FidTransfer, db::ChainEventRow), Box<dyn Error>> {
        let parsed_log: Transfer = parse_log(log.clone())?;

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
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.as_u64(),
            body: Some(on_chain_event::Body::IdRegisterEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());

        let fid_transfer = db::FidTransfer {
            fid: parsed_log.id.as_u32() as u32,
            custody_address: parsed_log.to.to_fixed_bytes(),
        };

        Ok((fid_transfer, event_row))
    }

    pub async fn persist_transfer_log(
        &self,
        store: &Store,
        log: &Log,
        chain_id: u32,
        timestamp: u32,
    ) -> Result<(), Box<dyn Error>> {
        let (fid_transfer, event_row) = self.process_transfer_log(log, chain_id, timestamp).await?;

        event_row.insert(&store).await?;
        db::FidRow::transfer(&store, &fid_transfer).await?;

        Ok(())
    }

    pub async fn persist_many_transfer_logs(
        &self,
        store: &Store,
        logs: Vec<Log>,
        chain_id: u32,
        timestamps: &[u32],
    ) -> Result<(), Box<dyn Error>> {
        let mut fid_transfers = Vec::new();
        let mut event_rows = Vec::new();

        for (log, timestamp) in logs.iter().zip(timestamps.iter()) {
            let (fid_transfer, event_row) =
                self.process_transfer_log(log, chain_id, *timestamp).await?;
            fid_transfers.push(fid_transfer);
            event_rows.push(event_row);
        }

        let mut conn = store.conn.acquire().await?;
        let mut transaction = conn.begin().await?;

        let insert_queries = db::ChainEventRow::generate_bulk_insert_queries(&event_rows)?;
        for query in insert_queries {
            sqlx::query(&query).execute(&mut *transaction).await?;
        }

        let transfer_queries = db::FidRow::generate_bulk_transfer_queries(&fid_transfers)?;
        for query in transfer_queries {
            sqlx::query(&query).execute(&mut *transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    pub async fn get_recovery_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(get_signature_topic(RECOVERY_SIGNATURE));
        let logs = get_logs(self.provider.clone(), &filter).await?;
        Ok(logs)
    }

    pub async fn process_recovery_log(
        &self,
        log: &Log,
        chain_id: u32,
        timestamp: u32,
    ) -> Result<(db::FidTransfer, db::ChainEventRow), Box<dyn Error>> {
        let parsed_log: Recover = parse_log(log.clone())?;

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
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.as_u64(),
            body: Some(on_chain_event::Body::IdRegisterEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());

        // A recovery's delta is identical to a transfer
        Ok((
            db::FidTransfer {
                fid: parsed_log.id.as_u32(),
                custody_address: parsed_log.to.to_fixed_bytes(),
            },
            event_row,
        ))
    }

    pub async fn persist_recovery_log(
        &self,
        store: &Store,
        log: &Log,
        chain_id: u32,
        timestamp: u32,
    ) -> Result<(), Box<dyn Error>> {
        let (recovery_update, event_row) =
            self.process_recovery_log(log, chain_id, timestamp).await?;

        event_row.insert(&store).await?;
        db::FidRow::transfer(&store, &recovery_update).await?;

        Ok(())
    }

    pub async fn persist_many_recovery_logs(
        &self,
        store: &Store,
        logs: Vec<Log>,
        chain_id: u32,
        timestamps: &[u32],
    ) -> Result<(), Box<dyn Error>> {
        let mut recoveries = Vec::new();
        let mut event_rows = Vec::new();

        for (log, timestamp) in logs.iter().zip(timestamps.iter()) {
            let (recovery_update, event_row) =
                self.process_recovery_log(log, chain_id, *timestamp).await?;
            recoveries.push(recovery_update);
            event_rows.push(event_row);
        }

        let mut connection = store.conn.acquire().await?;
        let mut transaction = connection.begin().await?;

        let insert_queries = db::ChainEventRow::generate_bulk_insert_queries(&event_rows)?;
        for query in insert_queries {
            let query = sqlx::query(&query);
            query.execute(&mut *transaction).await?;
        }

        let transfer_queries = db::FidRow::generate_bulk_transfer_queries(&recoveries)?;
        for query in transfer_queries {
            let query = sqlx::query(&query);
            query.execute(&mut *transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    pub async fn get_change_recovery_address_logs(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<Log>, Box<dyn Error>> {
        let filter = Filter::new()
            .address(self.inner.address())
            .from_block(start_block)
            .to_block(end_block)
            .topic0(get_signature_topic(CHANGE_RECOVERY_ADDRESS_SIGNATURE));
        let logs = get_logs(self.provider.clone(), &filter).await?;

        Ok(logs)
    }

    pub async fn process_change_recovery_address_log(
        &self,
        log: &Log,
        chain_id: u32,
        timestamp: u32,
    ) -> Result<(db::FidRecoveryUpdate, db::ChainEventRow), Box<dyn Error>> {
        let parsed_log: ChangeRecoveryAddress = parse_log(log.clone())?;

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
            block_timestamp: timestamp as u64,
            transaction_hash: log.transaction_hash.unwrap().as_bytes().to_vec(),
            log_index: log.log_index.unwrap().as_u32(),
            fid: parsed_log.id.as_u64(),
            body: Some(on_chain_event::Body::IdRegisterEventBody(body)),
            tx_index: log.transaction_index.unwrap().as_u32(),
            version: 2,
        };

        let event_row = db::ChainEventRow::new(onchain_event, log.data.to_vec());

        Ok((
            db::FidRecoveryUpdate {
                fid: parsed_log.id.as_u32(),
                recovery_address: parsed_log.recovery.to_fixed_bytes(),
            },
            event_row,
        ))
    }

    pub async fn persist_change_recovery_address_log(
        &self,
        store: &Store,
        log: &Log,
        chain_id: u32,
        timestamp: u32,
    ) -> Result<(), Box<dyn Error>> {
        let (fid_recovery_update, event_row) = self
            .process_change_recovery_address_log(log, chain_id, timestamp)
            .await?;

        db::FidRow::update_recovery_address(&store, &fid_recovery_update).await?;
        event_row.insert(&store).await?;

        Ok(())
    }

    pub async fn persist_many_change_recovery_address_logs(
        &self,
        store: &Store,
        logs: Vec<Log>,
        chain_id: u32,
        timestamps: &[u32],
    ) -> Result<(), Box<dyn Error>> {
        let mut recovery_address_updates = Vec::new();
        let mut event_rows = Vec::new();

        for (log, timestamp) in logs.iter().zip(timestamps.iter()) {
            let (update, event_row) = self
                .process_change_recovery_address_log(log, chain_id, *timestamp)
                .await?;
            recovery_address_updates.push(update);
            event_rows.push(event_row);
        }

        let mut conn = store.conn.acquire().await?;
        let mut transaction = conn.begin().await?;

        let insert_queries = db::ChainEventRow::generate_bulk_insert_queries(&event_rows)?;
        for query in insert_queries {
            sqlx::query(&query).execute(&mut *transaction).await?;
        }

        let update_queries =
            db::FidRow::generate_bulk_update_recovery_address_queries(&recovery_address_updates)?;
        for query in update_queries {
            sqlx::query(&query).execute(&mut *transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::core::types::{Bytes, Log, H160, U64};
    use ethers::types::H256;
    use hex::FromHex;
    use sqlx::Row;
    use std::str::FromStr;

    async fn setup_db() -> Store {
        let config = Config {
            db_path: "sqlite::memory:".to_string(),
            db_migrations_path: "../storage/migrations".to_string(),
            ..Default::default()
        };

        let store = Store::new(config).await;
        store
    }

    fn mock_log() -> Log {
        Log {
            address: H160::from_str("0x00000000fc6c5f01fc30151999387bb99a9f489b").unwrap(),
            topics: vec![
                H256::from_str(
                    "0xf2e19a901b0748d8b08e428d0468896a039ac751ec4fec49b44b7b9c28097e45",
                )
                .unwrap(),
                H256::from_str(
                    "0x00000000000000000000000074551863ebff52d6e3d6657dd1d2337bdb60521b",
                )
                .unwrap(),
                H256::from_str(
                    "0x0000000000000000000000000000000000000000000000000000000000000d55",
                )
                .unwrap(),
            ],
            data: Bytes::from_hex(
                "0x00000000000000000000000000000000fcb080a4d6c39a9354da9eb9bc104cd7",
            )
            .unwrap(),
            block_hash: Some(
                H256::from_str(
                    "0x81340703f2d3064dc4ce507b1491e25efdd32e048827f68819e12727c9924d5d",
                )
                .unwrap(),
            ),
            block_number: Some(U64::from(111894017)),
            transaction_hash: Some(
                H256::from_str(
                    "0xd6b5e15c489e27cdeecbbd8801d62b6f7a0ff05609bc89dd3ab1083c9e3a2d1a",
                )
                .unwrap(),
            ),
            transaction_index: Some(U64::from(7)),
            log_index: Some(U256::from(208)),
            transaction_log_index: None,
            log_type: None,
            removed: Some(false),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_register_logs() {
        //TODO: fix broken test of missing chain_events table
        let store = setup_db().await;
        let (provider, mock) = Provider::mocked();
        mock.push::<Vec<Log>, Vec<Log>>(vec![mock_log()])
            .expect("pushing mock log");

        let id_registry = Contract::new(
            provider.into(),
            "0x00000000fc6c5f01fc30151999387bb99a9f489b".to_string(),
            "./abis/IdRegistry.json".to_string(),
        )
        .unwrap();

        let logs = id_registry.get_register_logs(0, 100000000).await.unwrap();
        id_registry
            .persist_register_log(&store, &logs[0], 10u32, 0u32)
            .await
            .unwrap();

        let mut conn = store.conn.acquire().await.unwrap();
        let chain_event_rows = sqlx::query("select * from chain_events")
            .fetch_all(&mut *conn)
            .await
            .unwrap();
        assert_eq!(chain_event_rows.len(), 1);
        assert_eq!(chain_event_rows[0].get::<i64, _>("fid"), 3413);
        assert_eq!(chain_event_rows[0].get::<i32, _>("type"), 3);
        assert_eq!(chain_event_rows[0].get::<i32, _>("chain_id"), 10);
        assert_eq!(chain_event_rows[0].get::<i32, _>("transaction_index"), 7);
        assert_eq!(chain_event_rows[0].get::<i32, _>("block_number"), 111894017);
        assert_eq!(
            hex::encode(chain_event_rows[0].get::<Vec<u8>, _>("block_hash")),
            "81340703f2d3064dc4ce507b1491e25efdd32e048827f68819e12727c9924d5d"
        );
        assert_eq!(
            hex::encode(chain_event_rows[0].get::<Vec<u8>, _>("transaction_hash")),
            "d6b5e15c489e27cdeecbbd8801d62b6f7a0ff05609bc89dd3ab1083c9e3a2d1a"
        );
        assert_eq!(
            hex::encode(chain_event_rows[0].get::<Vec<u8>, _>("body")),
            "0a1474551863ebff52d6e3d6657dd1d2337bdb60521b1001221400000000fcb080a4d6c39a9354da9eb9bc104cd7"
        );
        assert_eq!(
            hex::encode(chain_event_rows[0].get::<Vec<u8>, _>("raw")),
            "00000000000000000000000000000000fcb080a4d6c39a9354da9eb9bc104cd7"
        );

        let fid_rows = sqlx::query("select * from fids")
            .fetch_all(&mut *conn)
            .await
            .unwrap();

        assert_eq!(fid_rows.len(), 1);
        assert_eq!(fid_rows[0].get::<i64, _>("fid"), 3413);
        assert_eq!(
            fid_rows[0].get::<String, _>("chain_event_id"),
            chain_event_rows[0].get::<String, _>("id")
        );
        assert_eq!(
            hex::encode(fid_rows[0].get::<Vec<u8>, _>("custody_address")),
            "74551863ebff52d6e3d6657dd1d2337bdb60521b"
        );
        assert_eq!(
            hex::encode(fid_rows[0].get::<Vec<u8>, _>("recovery_address")),
            "00000000fcb080a4d6c39a9354da9eb9bc104cd7"
        );
    }
}
