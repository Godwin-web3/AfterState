use near_mpc_contract_interface::types::{
    EvmExtractedValue, EvmExtractor, EvmFinality, EvmLog, EvmRpcRequest, EvmTxId,
    ExtractedValue, ForeignChainRpcRequest, ForeignTxPayloadVersion, ForeignTxSignPayload,
    Hash160, Hash256,
};

/// This vector is intentionally deterministic and checks our expected hash
/// against the actual DTO/Borsh implementation from NEAR MPC 3.19.0.
#[test]
fn base_sepolia_v1_payload_hash_matches_pinned_mpc_types() {
    let tx_hash = [0x11; 32];
    let block_hash = [0x22; 32];

    let request = ForeignChainRpcRequest::Base(EvmRpcRequest {
        tx_id: EvmTxId(tx_hash),
        extractors: [
            EvmExtractor::BlockHash,
            EvmExtractor::Log { log_index: 7 },
        ]
        .into(),
        finality: EvmFinality::Finalized,
    });

    let log = EvmLog {
        removed: false,
        log_index: 7,
        transaction_index: 3,
        transaction_hash: Hash256(tx_hash),
        block_hash: Hash256(block_hash),
        block_number: 100,
        address: Hash160([0x33; 20]),
        data: "0xdeadbeef".to_string(),
        topics: vec![Hash256([0x44; 32]), Hash256([0x55; 32])],
    };

    let payload = ForeignTxSignPayload::new(
        ForeignTxPayloadVersion::V1,
        request,
        vec![
            ExtractedValue::EvmExtractedValue(EvmExtractedValue::BlockHash(
                Hash256(block_hash),
            )),
            ExtractedValue::EvmExtractedValue(EvmExtractedValue::Log(log)),
        ],
    );

    let bytes = borsh::to_vec(&payload).expect("Borsh bytes");
    assert_eq!(hex::encode(&bytes), "0006000000111111111111111111111111111111111111111111111111111111111111111102000000000107000000000000000202000000010022222222222222222222222222222222222222222222222222222222222222220101000700000000000000030000000000000011111111111111111111111111111111111111111111111111111111111111112222222222222222222222222222222222222222222222222222222222222222640000000000000033333333333333333333333333333333333333330a000000307864656164626565660200000044444444444444444444444444444444444444444444444444444444444444445555555555555555555555555555555555555555555555555555555555555555");
    let actual = payload.compute_msg_hash().expect("Borsh hash");
    assert_eq!(
        hex::encode(actual.0),
        "478574bcc3b8a756291116267886ee49da9899924c5145dface491ab006779a8"
    );
}
