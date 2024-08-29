use ethers::contract::{Eip712, EthAbiType};
use ethers::types::{Address, Bytes, U256};
use serde::{Deserialize, Deserializer};

fn deserialize_u256_from_i64<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    i64::deserialize(deserializer).map(U256::from)
}

#[derive(Clone, Debug, Deserialize, Eip712, EthAbiType)]
#[eip712(
    name = "Farcaster Verify Ethereum Address",
    version = "2.0.0",
    salt = "0xf2d857f4a3edcb9b78b4d503bfe733db1e3f6cdc2b7971ee739626c97e86a558"
)]

#[allow(non_snake_case)]
pub struct EIP712VerificationClaim {
    pub fid: U256,
    pub address: Address,
    pub blockHash: Bytes,
    pub network: u8,
}

#[derive(Clone, Debug, Deserialize, Eip712, EthAbiType)]
#[eip712(
    name = "Farcaster Verify Ethereum Address",
    version = "2.0.0",
    salt = "0xf2d857f4a3edcb9b78b4d503bfe733db1e3f6cdc2b7971ee739626c97e86a558"
)]

pub struct EIP712MessageData {
    pub hash: Bytes,
}

#[derive(Clone, Debug, Deserialize, Eip712, EthAbiType)]
#[eip712(
    name = "Farcaster name verification",
    version = "1",
    chain_id = 1,
    verifying_contract = "0xe3be01d99baa8db9905b33a3ca391238234b79d1"
)]

pub struct EIP712UsernameProof {
    pub name: String,
    #[serde(deserialize_with = "deserialize_u256_from_i64")]
    pub timestamp: U256,
    pub owner: Address,
}
