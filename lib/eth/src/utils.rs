use ethers::abi::Abi;
use ethers::providers::JsonRpcClient;
use ethers::{
    providers::{Middleware, Provider},
    types::H256,
};
use std::error::Error;
use std::fs;

pub fn read_abi(path: String) -> Result<Abi, Box<dyn Error>> {
    let abi_str = fs::read_to_string(path)?;
    let contract_abi: Abi = serde_json::from_str(&abi_str)?;
    Ok(contract_abi)
}

pub async fn get_block_timestamp<T: JsonRpcClient + Clone>(
    provider: Provider<T>,
    block_hash: H256,
) -> Result<i64, Box<dyn Error>> {
    let block = loop {
        match provider.get_block(block_hash).await {
            Ok(Some(block)) => break block,
            Ok(None) => return Err("Block not found".into()),
            Err(e) => {
                if e.to_string().contains("429") {
                    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                    continue;
                } else {
                    return Err(e.into());
                }
            }
        }
    };
    let timestamp = block.timestamp.as_u32().into();

    Ok::<_, Box<dyn Error>>(timestamp)
}
