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
