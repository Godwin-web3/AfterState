use afterstate_near_foreign_tx_fixtures::{
    afterstate_eip712_digest, Eip712SourceFact,
};
use sha3::{Digest, Keccak256};

fn hash_text(value: &str) -> [u8; 32] {
    Keccak256::digest(value.as_bytes()).into()
}

fn address_from_u32(value: u32) -> [u8; 20] {
    let mut address = [0u8; 20];
    address[16..].copy_from_slice(&value.to_be_bytes());
    address
}

#[test]
fn digest_vector_matches_afterstate_solidity_typed_data() {
    let fact = Eip712SourceFact {
        source_chain_id: 84_532,
        emitter: address_from_u32(0xBEEF),
        event_schema: hash_text("Payment(address,address,uint256)"),
        agreement_id: hash_text("agreement-1"),
        source_block_hash: hash_text("source-block"),
        source_tx_hash: hash_text("source-transaction"),
        source_block_number: 123_456,
        log_index: 2,
        asset: address_from_u32(0x0A55E7),
        amount: 500,
        recipient: address_from_u32(0xCAFE),
        observed_at: 1_700_000_000,
        expires_at: 1_700_003_600,
        key_version: 1,
        agreement_version: 1,
        finality_policy_id: hash_text("base-sepolia-finalized"),
        nonce: hash_text("nonce-1"),
    };

    let digest = afterstate_eip712_digest(
        &fact,
        84_532,
        address_from_u32(0x1001),
        address_from_u32(0x2002),
    );
    assert_eq!(hex::encode(digest), "TODO");
}
