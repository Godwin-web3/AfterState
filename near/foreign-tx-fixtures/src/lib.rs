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
