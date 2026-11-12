use afterstate_near_foreign_tx_fixtures::{
    decode_source_payment, source_payment_topic0, BaseEvmLogCandidate,
    SourcePaymentDecodeError,
};

fn address_word(address: [u8; 20]) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[12..].copy_from_slice(&address);
    word
}

fn u32_word(value: u32) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[28..].copy_from_slice(&value.to_be_bytes());
    word
}

fn u128_word(value: u128) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[16..].copy_from_slice(&value.to_be_bytes());
    word
}

fn u64_word(value: u64) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[24..].copy_from_slice(&value.to_be_bytes());
    word
}

fn candidate() -> BaseEvmLogCandidate {
    let agreement_id = [0x11; 32];
    let asset = [0x22; 20];
    let payer = [0x33; 20];
    let recipient = [0x44; 20];
    let nonce = [0x55; 32];

    let mut data = Vec::new();
    data.extend_from_slice(&address_word(payer));
    data.extend_from_slice(&address_word(recipient));
    data.extend_from_slice(&u128_word(987_654));
    data.extend_from_slice(&u64_word(1_800_000_000));
    data.extend_from_slice(&nonce);

    BaseEvmLogCandidate {
        transaction_hash: [0x66; 32],
        block_hash: [0x77; 32],
        block_number: 12_345,
        transaction_index: 2,
        log_index: 4,
        emitter: [0x88; 20],
        removed: false,
        data: format!("0x{}", hex::encode(data)),
        topics: vec![
            source_payment_topic0(),
            agreement_id,
            u32_word(3),
            address_word(asset),
        ],
    }
}

#[test]
fn decodes_source_payment_event_fields_exactly() {
    let decoded = decode_source_payment(&candidate()).unwrap();
    assert_eq!(decoded.agreement_id, [0x11; 32]);
    assert_eq!(decoded.agreement_version, 3);
    assert_eq!(decoded.asset, [0x22; 20]);
    assert_eq!(decoded.payer, [0x33; 20]);
    assert_eq!(decoded.recipient, [0x44; 20]);
    assert_eq!(decoded.amount, 987_654);
    assert_eq!(decoded.source_timestamp, 1_800_000_000);
    assert_eq!(decoded.nonce, [0x55; 32]);
}

#[test]
fn rejects_wrong_event_topic() {
    let mut candidate = candidate();
    candidate.topics[0] = [0x99; 32];
    assert_eq!(
        decode_source_payment(&candidate).unwrap_err(),
        SourcePaymentDecodeError::WrongEventSchema
    );
}

#[test]
fn rejects_noncanonical_address_padding() {
    let mut candidate = candidate();
    candidate.topics[3][0] = 1;
    assert_eq!(
        decode_source_payment(&candidate).unwrap_err(),
        SourcePaymentDecodeError::InvalidAddressPadding
    );
}

#[test]
fn rejects_removed_logs() {
    let mut candidate = candidate();
    candidate.removed = true;
    assert_eq!(
        decode_source_payment(&candidate).unwrap_err(),
        SourcePaymentDecodeError::RemovedLog
    );
}
