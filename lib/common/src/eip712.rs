use ethers::providers::JsonRpcClient;
use ethers::types::{Address, Bytes, U256};
use serde::{Deserialize, Deserializer};

use crate::errors::{BadRequestType, HubError};

fn deserialize_u256_from_i64<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    i64::deserialize(deserializer).map(U256::from)
}

#[derive(Clone, Debug, Deserialize)]
pub enum EIP712Claim {
    VerificationClaim(EIP712VerificationClaim),
    MessageData(EIP712MessageData),
    UserNameProof(EIP712UserNameProof),
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct VerifyTypedDataParams {
    pub address: Address,
    pub domain: Domain,
    pub types: &'static str,
    pub primaryType: &'static str,
    pub claim: EIP712Claim,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug)]
pub enum Domain {
    EIP712FarcasterDomain(EIP712FarcasterDomain),
    EIP712FarcasterDomainExtended(EIP712FarcasterDomainExtended),
    EIP712FarcasterUserNameDomain(EIP712FarcasterUserNameDomain),
}

#[derive(Clone, Debug)]
pub struct EIP712FarcasterDomain {
    pub name: &'static str,
    pub version: &'static str,
    pub salt: &'static str,
}

impl EIP712FarcasterDomain {
    fn with_chain_id(self, chain_id: u64) -> EIP712FarcasterDomainExtended {
        EIP712FarcasterDomainExtended {
            name: self.name,
            version: self.version,
            salt: self.salt,
            chain_id,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EIP712FarcasterDomainExtended {
    pub name: &'static str,
    pub version: &'static str,
    pub salt: &'static str,
    pub chain_id: u64,
}

const EIP_712_FARCASTER_DOMAIN: EIP712FarcasterDomain = EIP712FarcasterDomain {
    name: "Farcaster Verify Ethereum Address",
    version: "2.0.0",
    salt: "0xf2d857f4a3edcb9b78b4d503bfe733db1e3f6cdc2b7971ee739626c97e86a558",
};

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct EIP712FarcasterUserNameDomain {
    pub name: &'static str,
    pub version: &'static str,
    pub chain_id: u64,
    pub verifyingContract: &'static str,
}

const EIP_712_FARCASTER_USER_NAME_DOMAIN: EIP712FarcasterUserNameDomain =
    EIP712FarcasterUserNameDomain {
        name: "Farcaster name verification",
        version: "1",
        chain_id: 1,
        verifyingContract: "0xe3be01d99baa8db9905b33a3ca391238234b79d1",
    };

const EIP_712_VERIFICATION_CLAIM_TYPES: &str = r#"{
	VerificationClaim: [
		{"name": "fid", "type": "uint256"},
		{"name": "address", "type": "address"},
		{"name": "blockHash", "type": "bytes32"},
		{"name": "network", "type": "uint8"}
	]
}"#;

const EIP_712_USERNAME_PROOF_CLAIM_TYPES: &str = r#"{
	UserNameProof: [
		{"name": "fid", "type": "uint256"},
		{"name": "address", "type": "address"},
		{"name": "blockHash", "type": "bytes32"},
		{"name": "network", "type": "uint8"}
	]
}"#;

const EIP_712_FARCASTER_MESSAGE_DATA_TYPES: &str = r#"{
	"MessageData": [
		{"name": "hash", "type": "bytes"}
	]
}"#;

pub enum VerificationType {
    EOA = 0,
    Contract = 1,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct EIP712VerificationClaim {
    #[serde(deserialize_with = "deserialize_u256_from_i64")]
    pub fid: U256,
    pub address: Address,
    pub blockHash: Bytes,
    pub network: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EIP712MessageData {
    pub hash: Bytes,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EIP712UserNameProof {
    pub name: String,
    #[serde(deserialize_with = "deserialize_u256_from_i64")]
    pub timestamp: U256,
    pub owner: Address,
}

pub fn verify_eoa_message(_params: VerifyTypedDataParams) -> Result<bool, HubError> {
    Ok(true)
}

pub fn verify_contract_message<C>(
    _client: C,
    _params: VerifyTypedDataParams,
) -> Result<bool, HubError>
where
    C: JsonRpcClient<Error = HubError>,
{
    Ok(true)
}

pub fn verify_verification_claim_eoa_signature(
    claim: EIP712VerificationClaim,
    signature: Vec<u8>,
    address: Address,
    chain_id: u64,
) -> Result<bool, HubError> {
    if chain_id != 0 {
        return Err(HubError::BadRequest(
            BadRequestType::InvalidParam,
            format!("RPC client not provided for chainId {}", chain_id),
        ));
    }

    let params: VerifyTypedDataParams = VerifyTypedDataParams {
        address,
        domain: Domain::EIP712FarcasterDomain(EIP_712_FARCASTER_DOMAIN),
        types: EIP_712_VERIFICATION_CLAIM_TYPES,
        primaryType: "VerificationClaim",
        claim: EIP712Claim::VerificationClaim(claim),
        signature,
    };

    verify_eoa_message(params)
}

pub fn verify_verification_claim_contract_signature<C>(
    claim: EIP712VerificationClaim,
    signature: Vec<u8>,
    address: Address,
    chain_id: u64,
    client: C,
) -> Result<bool, HubError>
where
    C: JsonRpcClient<Error = HubError>,
{
    let params: VerifyTypedDataParams = VerifyTypedDataParams {
        address,
        domain: Domain::EIP712FarcasterDomainExtended(
            EIP_712_FARCASTER_DOMAIN.with_chain_id(chain_id),
        ),
        types: EIP_712_VERIFICATION_CLAIM_TYPES,
        primaryType: "VerificationClaim",
        claim: EIP712Claim::VerificationClaim(claim),
        signature,
    };

    verify_contract_message(client, params)
}

pub fn verify_verification_eth_address_claim_signature<C>(
    claim: EIP712VerificationClaim,
    signature: Vec<u8>,
    address: Address,
    verification_type: VerificationType,
    chain_id: u64,
    client: Option<C>,
) -> Result<bool, HubError>
where
    C: JsonRpcClient<Error = HubError>,
{
    match verification_type {
        VerificationType::EOA => {
            verify_verification_claim_eoa_signature(claim, signature, address, chain_id)
        }
        VerificationType::Contract => match client {
            Some(c) => {
                return verify_verification_claim_contract_signature(
                    claim, signature, address, chain_id, c,
                );
            }
            None => Err(HubError::BadRequest(
                BadRequestType::InvalidParam,
                "RPC client not provided for contract verification".to_string(),
            )),
        },
    }
}

pub fn verify_user_name_proof_claim(
    claim: EIP712UserNameProof,
    signature: Vec<u8>,
    address: Address,
) -> Result<bool, HubError> {
    let params: VerifyTypedDataParams = VerifyTypedDataParams {
        address,
        domain: Domain::EIP712FarcasterUserNameDomain(EIP_712_FARCASTER_USER_NAME_DOMAIN),
        types: EIP_712_USERNAME_PROOF_CLAIM_TYPES,
        primaryType: "UserNameProof",
        claim: EIP712Claim::UserNameProof(claim),
        signature,
    };

    verify_eoa_message(params)
}

pub fn verify_message_hash_signature(
    hash: Vec<u8>,
    signature: Vec<u8>,
    address: Address,
) -> Result<bool, HubError> {
    let message_data = EIP712MessageData {
        hash: Bytes::from(hash),
    };

    let params: VerifyTypedDataParams = VerifyTypedDataParams {
        address,
        domain: Domain::EIP712FarcasterDomain(EIP_712_FARCASTER_DOMAIN),
        types: EIP_712_FARCASTER_MESSAGE_DATA_TYPES,
        primaryType: "MessageData",
        claim: EIP712Claim::MessageData(message_data),
        signature,
    };

    verify_eoa_message(params)
}
