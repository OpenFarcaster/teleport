use ethers::abi::Abi;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

pub fn read_abi(path: String) -> Result<Abi, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let contract_abi: Abi = serde_json::from_reader(reader)?;
    Ok(contract_abi)
}
