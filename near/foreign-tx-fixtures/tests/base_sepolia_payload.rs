use afterstate_near_foreign_tx_fixtures::{
    build_base_foreign_tx_request, BaseEvmLogCandidate, CandidateError,
};
use near_mpc_contract_interface::types::{
    DomainId, EvmExtractor, EvmFinality, ForeignChainRpcRequest, Hash256,
};

fn candidate() -> BaseEvmLogCandidate {
    BaseEvmLogCandidate {
        transaction_hash: [0x11; 32],
        block_hash: [0x22; 32],
        block_number: 100,
        transaction_index: 3,
        log_index: 7,
        emitter: [0x33; 20],
        removed: false,
        data: "0xdeadbeef".to_string(),
        topics: vec![[0x44; 32], [0x55; 32]],
    }
}

#[test]
fn base_sepolia_v1_payload_hash_matches_pinned_mpc_types() {
    let args = build_base_foreign_tx_request(candidate(), DomainId(7)).unwrap();

    let actual = args.expected_payload_hash.expect("expected hash is mandatory");
    assert_eq!(
        hex::encode(actual.0),
        "9bc1f744d04ebee4e32a814c3742b5b7ded2cf0e1da23fa1689675b695d1616a"
    );

    match args.request {
        ForeignChainRpcRequest::Base(request) => {
            assert_eq!(request.tx_id.0, [0x11; 32]);
            assert_eq!(request.finality, EvmFinality::Finalized);
            assert_eq!(
                request.extractors.as_slice(),
                &[
                    EvmExtractor::BlockHash,
                    EvmExtractor::Log { log_index: 7 }
                ]
            );
        }
        other => panic!("wrong source chain request: {other:?}"),
    }
}

#[test]
fn removed_candidate_log_is_rejected_before_mpc_request() {
    let mut candidate = candidate();
    candidate.removed = true;

    assert_eq!(
        build_base_foreign_tx_request(candidate, DomainId(7)).unwrap_err(),
        CandidateError::RemovedLog
    );
}
