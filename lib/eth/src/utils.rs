use ethers::abi::Abi;
use std::error::Error;
use std::fs;

pub fn read_abi(path: String) -> Result<Abi, Box<dyn Error>> {
    let abi_str = fs::read_to_string(path)?;
    let contract_abi: Abi = serde_json::from_str(&abi_str)?;
    Ok(contract_abi)
}
