use afterstate_near_foreign_tx_fixtures::{
    evm_address_from_mpc_public_key, evm_address_from_mpc_public_key_json, EvmAddressDerivationError,
};
use near_mpc_contract_interface::types::{PublicKey, Secp256k1PublicKey};

#[test]
fn derives_known_ethereum_address_from_uncompressed_secp256k1_key() {
    let x = hex::decode(
        "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
    )
    .unwrap();
    let y = hex::decode(
        "483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8",
    )
    .unwrap();
    let mut xy = [0u8; 64];
    xy[..32].copy_from_slice(&x);
    xy[32..].copy_from_slice(&y);

    let key = PublicKey::Secp256k1(Secp256k1PublicKey(xy));
    let address = evm_address_from_mpc_public_key(&key).unwrap();
    assert_eq!(
        hex::encode(address),
        "7e5f4552091a69125d5dfcb7b8c2659029395bdf"
    );
}

#[test]
fn rejects_non_secp256k1_mpc_key_scheme() {
    let key = PublicKey::Ed25519(
        near_mpc_contract_interface::types::Ed25519PublicKey([1u8; 32]),
    );
    assert_eq!(
        evm_address_from_mpc_public_key(&key).unwrap_err(),
        EvmAddressDerivationError::UnsupportedPublicKeyScheme
    );
}

#[test]
fn parses_the_live_mpc_public_key_json_format() {
    let x = hex::decode(
        "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
    )
    .unwrap();
    let y = hex::decode(
        "483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8",
    )
    .unwrap();
    let mut xy = [0u8; 64];
    xy[..32].copy_from_slice(&x);
    xy[32..].copy_from_slice(&y);
    let public_key_json = serde_json::to_string(
        &format!("secp256k1:{}", bs58::encode(xy).into_string())
    )
    .unwrap();

    let address = evm_address_from_mpc_public_key_json(&public_key_json).unwrap();
    assert_eq!(
        hex::encode(address),
        "7e5f4552091a69125d5dfcb7b8c2659029395bdf"
    );
}
