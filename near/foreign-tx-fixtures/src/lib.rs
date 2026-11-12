//! Version-pinned NEAR MPC ForeignTx payload construction helpers.
//!
//! This crate intentionally depends on the exact NEAR MPC 3.19.0 interface
//! commit. Updating that pin is a protocol change: review DTO/Borsh changes
//! and regenerate the regression fixtures deliberately.

use serde::{Deserialize, Serialize};
use near_mpc_contract_interface::types::{
    EvmExtractedValue, EvmExtractor, EvmFinality, EvmLog, EvmRpcRequest, EvmTxId,
    ExtractedValue, ForeignChainRpcRequest, ForeignTxPayloadVersion, ForeignTxSignPayload,
    Hash160, Hash256, VerifyForeignTransactionRequestArgs, DomainId,
};

pub const MPC_RELEASE: &str = "3.19.0";
pub const MPC_COMMIT: &str = "86e2cc4d124397ec81f436b11f48bf9b6dfd062e";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseEvmLogCandidate {
    pub transaction_hash: [u8; 32],
    pub block_hash: [u8; 32],
    pub block_number: u64,
    pub transaction_index: u64,
    pub log_index: u64,
    pub emitter: [u8; 20],
    pub removed: bool,
    pub data: String,
    pub topics: Vec<[u8; 32]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateError {
    RemovedLog,
}

/// Constructs the exact MPC 3.19.0 Base ForeignTx request and expected payload
/// hash for a candidate receipt log. The returned request is still only a
/// candidate: MPC independently extracts the transaction/log from its
/// configured providers and rejects a payload-hash mismatch.
pub fn build_base_foreign_tx_request(
    candidate: BaseEvmLogCandidate,
    domain_id: DomainId,
) -> Result<VerifyForeignTransactionRequestArgs, CandidateError> {
    if candidate.removed {
        return Err(CandidateError::RemovedLog);
    }

    let request = ForeignChainRpcRequest::Base(EvmRpcRequest {
        tx_id: EvmTxId(candidate.transaction_hash),
        extractors: [
            EvmExtractor::BlockHash,
            EvmExtractor::Log {
                log_index: candidate.log_index,
            },
        ]
        .into(),
        finality: EvmFinality::Finalized,
    });

    let log = EvmLog {
        removed: candidate.removed,
        log_index: candidate.log_index,
        transaction_index: candidate.transaction_index,
        transaction_hash: Hash256(candidate.transaction_hash),
        block_hash: Hash256(candidate.block_hash),
        block_number: candidate.block_number,
        address: Hash160(candidate.emitter),
        data: candidate.data,
        topics: candidate.topics.into_iter().map(Hash256).collect(),
    };

    let payload = ForeignTxSignPayload::new(
        ForeignTxPayloadVersion::V1,
        request.clone(),
        vec![
            ExtractedValue::EvmExtractedValue(EvmExtractedValue::BlockHash(
                Hash256(candidate.block_hash),
            )),
            ExtractedValue::EvmExtractedValue(EvmExtractedValue::Log(log)),
        ],
    );
    let expected_payload_hash = payload
        .compute_msg_hash()
        .expect("MPC ForeignTx V1 payload serialization must be infallible");

    Ok(VerifyForeignTransactionRequestArgs {
        request,
        domain_id,
        payload_version: ForeignTxPayloadVersion::V1,
        expected_payload_hash: Some(expected_payload_hash),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePaymentFields {
    pub agreement_id: [u8; 32],
    pub agreement_version: u32,
    pub asset: [u8; 20],
    pub payer: [u8; 20],
    pub recipient: [u8; 20],
    pub amount: u128,
    pub source_timestamp: u64,
    pub nonce: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourcePaymentDecodeError {
    RemovedLog,
    InvalidEmitter,
    InvalidTopicCount,
    WrongEventSchema,
    InvalidAddressPadding,
    InvalidIntegerRange,
    InvalidDataLength,
    InvalidFields,
}

pub fn source_payment_topic0() -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    Keccak256::digest(
        b"SourcePayment(bytes32,uint32,address,address,address,uint256,uint64,bytes32)",
    )
    .into()
}

fn decode_address_word(word: &[u8]) -> Result<[u8; 20], SourcePaymentDecodeError> {
    if word.len() != 32 || word[..12].iter().any(|byte| *byte != 0) {
        return Err(SourcePaymentDecodeError::InvalidAddressPadding);
    }
    let mut address = [0u8; 20];
    address.copy_from_slice(&word[12..]);
    Ok(address)
}

fn decode_u128_word(word: &[u8]) -> Result<u128, SourcePaymentDecodeError> {
    if word.len() != 32 || word[..16].iter().any(|byte| *byte != 0) {
        return Err(SourcePaymentDecodeError::InvalidIntegerRange);
    }
    let mut value = [0u8; 16];
    value.copy_from_slice(&word[16..]);
    Ok(u128::from_be_bytes(value))
}

fn decode_u64_word(word: &[u8]) -> Result<u64, SourcePaymentDecodeError> {
    if word.len() != 32 || word[..24].iter().any(|byte| *byte != 0) {
        return Err(SourcePaymentDecodeError::InvalidIntegerRange);
    }
    let mut value = [0u8; 8];
    value.copy_from_slice(&word[24..]);
    Ok(u64::from_be_bytes(value))
}

/// Decode the exact SourcePayment event emitted by AfterStateSourceEmitter.
/// The MPC request must still verify this candidate log before any fact is accepted.
pub fn decode_source_payment(
    candidate: &BaseEvmLogCandidate,
) -> Result<SourcePaymentFields, SourcePaymentDecodeError> {
    if candidate.removed {
        return Err(SourcePaymentDecodeError::RemovedLog);
    }
    if candidate.emitter == [0u8; 20] {
        return Err(SourcePaymentDecodeError::InvalidEmitter);
    }
    if candidate.topics.len() != 4 {
        return Err(SourcePaymentDecodeError::InvalidTopicCount);
    }
    if candidate.topics[0] != source_payment_topic0() {
        return Err(SourcePaymentDecodeError::WrongEventSchema);
    }

    let agreement_id = candidate.topics[1];
    let agreement_version = decode_u32_word(&candidate.topics[2])?;
    let asset = decode_address_word(&candidate.topics[3])?;

    let data_hex = candidate
        .data
        .strip_prefix("0x")
        .unwrap_or(candidate.data.as_str());
    let data = hex::decode(data_hex).map_err(|_| SourcePaymentDecodeError::InvalidDataLength)?;
    if data.len() != 160 {
        return Err(SourcePaymentDecodeError::InvalidDataLength);
    }

    let payer = decode_address_word(&data[0..32])?;
    let recipient = decode_address_word(&data[32..64])?;
    let amount = decode_u128_word(&data[64..96])?;
    let source_timestamp = decode_u64_word(&data[96..128])?;
    let mut nonce = [0u8; 32];
    nonce.copy_from_slice(&data[128..160]);

    if agreement_id == [0u8; 32]
        || agreement_version == 0
        || asset == [0u8; 20]
        || payer == [0u8; 20]
        || recipient == [0u8; 20]
        || amount == 0
        || source_timestamp == 0
        || nonce == [0u8; 32]
    {
        return Err(SourcePaymentDecodeError::InvalidFields);
    }

    Ok(SourcePaymentFields {
        agreement_id,
        agreement_version,
        asset,
        payer,
        recipient,
        amount,
        source_timestamp,
        nonce,
    })
}

fn decode_u32_word(word: &[u8]) -> Result<u32, SourcePaymentDecodeError> {
    if word.len() != 32 || word[..28].iter().any(|byte| *byte != 0) {
        return Err(SourcePaymentDecodeError::InvalidIntegerRange);
    }
    let mut value = [0u8; 4];
    value.copy_from_slice(&word[28..]);
    Ok(u32::from_be_bytes(value))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Eip712SourceFact {
    pub source_chain_id: u64,
    pub emitter: [u8; 20],
    pub event_schema: [u8; 32],
    pub agreement_id: [u8; 32],
    pub source_block_hash: [u8; 32],
    pub source_tx_hash: [u8; 32],
    pub source_block_number: u64,
    pub log_index: u64,
    pub asset: [u8; 20],
    pub amount: u128,
    pub recipient: [u8; 20],
    pub observed_at: u64,
    pub expires_at: u64,
    pub key_version: u32,
    pub agreement_version: u32,
    pub finality_policy_id: [u8; 32],
    pub nonce: [u8; 32],
}

fn keccak256(bytes: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    Keccak256::digest(bytes).into()
}

fn uint_word(value: u128) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[16..].copy_from_slice(&value.to_be_bytes());
    word
}

fn address_word(value: [u8; 20]) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[12..].copy_from_slice(&value);
    word
}

/// Reproduces AfterStateEIP712Verifier._digest exactly. Amount is constrained
/// to u128 here because the current registry agreement bounds are uint128.
pub fn afterstate_eip712_digest(
    fact: &Eip712SourceFact,
    destination_chain_id: u64,
    verifier_contract: [u8; 20],
    destination_registry: [u8; 20],
) -> [u8; 32] {
    let source_fact_typehash = keccak256(b"SourceFact(uint256 sourceChainId,address emitter,bytes32 eventSchema,bytes32 agreementId,bytes32 sourceBlockHash,bytes32 sourceTxHash,uint256 sourceBlockNumber,uint256 logIndex,address asset,uint256 amount,address recipient,uint64 observedAt,uint64 expiresAt,uint32 keyVersion,uint32 agreementVersion,bytes32 finalityPolicyId,bytes32 nonce,address destinationRegistry)");
    let domain_typehash =
        keccak256(b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");
    let name_hash = keccak256(b"AfterState");
    let version_hash = keccak256(b"1");

    let mut encoded_fact = Vec::with_capacity(19 * 32);
    encoded_fact.extend_from_slice(&source_fact_typehash);
    encoded_fact.extend_from_slice(&uint_word(fact.source_chain_id as u128));
    encoded_fact.extend_from_slice(&address_word(fact.emitter));
    encoded_fact.extend_from_slice(&fact.event_schema);
    encoded_fact.extend_from_slice(&fact.agreement_id);
    encoded_fact.extend_from_slice(&fact.source_block_hash);
    encoded_fact.extend_from_slice(&fact.source_tx_hash);
    encoded_fact.extend_from_slice(&uint_word(fact.source_block_number as u128));
    encoded_fact.extend_from_slice(&uint_word(fact.log_index as u128));
    encoded_fact.extend_from_slice(&address_word(fact.asset));
    encoded_fact.extend_from_slice(&uint_word(fact.amount));
    encoded_fact.extend_from_slice(&address_word(fact.recipient));
    encoded_fact.extend_from_slice(&uint_word(fact.observed_at as u128));
    encoded_fact.extend_from_slice(&uint_word(fact.expires_at as u128));
    encoded_fact.extend_from_slice(&uint_word(fact.key_version as u128));
    encoded_fact.extend_from_slice(&uint_word(fact.agreement_version as u128));
    encoded_fact.extend_from_slice(&fact.finality_policy_id);
    encoded_fact.extend_from_slice(&fact.nonce);
    encoded_fact.extend_from_slice(&address_word(destination_registry));
    let struct_hash = keccak256(&encoded_fact);

    let mut encoded_domain = Vec::with_capacity(5 * 32);
    encoded_domain.extend_from_slice(&domain_typehash);
    encoded_domain.extend_from_slice(&name_hash);
    encoded_domain.extend_from_slice(&version_hash);
    encoded_domain.extend_from_slice(&uint_word(destination_chain_id as u128));
    encoded_domain.extend_from_slice(&address_word(verifier_contract));
    let domain_separator = keccak256(&encoded_domain);

    let mut digest_input = Vec::with_capacity(66);
    digest_input.extend_from_slice(&[0x19, 0x01]);
    digest_input.extend_from_slice(&domain_separator);
    digest_input.extend_from_slice(&struct_hash);
    keccak256(&digest_input)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvmSignatureError {
    WrongSignatureScheme,
    InvalidRecoveryId,
    InvalidRPoint,
    InvalidRScalar,
    InvalidSScalar,
}

const SECP256K1_ORDER: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe,
    0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b,
    0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36, 0x41, 0x41,
];

const SECP256K1_HALF_ORDER: [u8; 32] = [
    0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0x5d, 0x57, 0x6e, 0x73, 0x57, 0xa4, 0x50, 0x1d,
    0xdf, 0xe9, 0x2f, 0x46, 0x68, 0x1b, 0x20, 0xa0,
];

fn is_zero(bytes: &[u8; 32]) -> bool {
    bytes.iter().all(|byte| *byte == 0)
}

/// Convert NEAR MPC's recoverable secp256k1 response to Solidity's r||s||v.
/// The EVM verifier requires v=27/28 and canonical low-s signatures.
pub fn mpc_signature_to_evm_bytes(
    response: near_mpc_contract_interface::types::SignatureResponse,
) -> Result<[u8; 65], EvmSignatureError> {
    use near_mpc_contract_interface::types::SignatureResponse;

    let signature = match response {
        SignatureResponse::Secp256k1(signature) => signature,
        SignatureResponse::Ed25519 { .. } => {
            return Err(EvmSignatureError::WrongSignatureScheme);
        }
    };

    if signature.recovery_id > 1 {
        return Err(EvmSignatureError::InvalidRecoveryId);
    }
    let point = signature.big_r.affine_point;
    if point[0] != 0x02 && point[0] != 0x03 {
        return Err(EvmSignatureError::InvalidRPoint);
    }
    let mut r = [0u8; 32];
    r.copy_from_slice(&point[1..]);
    if is_zero(&r) || r >= SECP256K1_ORDER {
        return Err(EvmSignatureError::InvalidRScalar);
    }
    let s = signature.s.scalar;
    if is_zero(&s) || s > SECP256K1_HALF_ORDER {
        return Err(EvmSignatureError::InvalidSScalar);
    }

    let mut output = [0u8; 65];
    output[..32].copy_from_slice(&r);
    output[32..64].copy_from_slice(&s);
    output[64] = 27 + signature.recovery_id;
    Ok(output)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvmAddressDerivationError {
    InvalidPublicKeyJson,
    UnsupportedPublicKeyScheme,
}

pub fn evm_address_from_mpc_public_key(
    public_key: &near_mpc_contract_interface::types::PublicKey,
) -> Result<[u8; 20], EvmAddressDerivationError> {
    use near_mpc_contract_interface::types::PublicKey;

    let uncompressed_xy = match public_key {
        PublicKey::Secp256k1(key) => key.as_bytes(),
        _ => return Err(EvmAddressDerivationError::UnsupportedPublicKeyScheme),
    };
    let digest = keccak256(uncompressed_xy);
    let mut address = [0u8; 20];
    address.copy_from_slice(&digest[12..]);
    Ok(address)
}

pub fn evm_address_from_mpc_public_key_json(
    json: &str,
) -> Result<[u8; 20], EvmAddressDerivationError> {
    let public_key: near_mpc_contract_interface::types::PublicKey =
        serde_json::from_str(json).map_err(|_| EvmAddressDerivationError::InvalidPublicKeyJson)?;
    evm_address_from_mpc_public_key(&public_key)
}
