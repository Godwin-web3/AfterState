use afterstate_near_foreign_tx_fixtures::{
    mpc_signature_to_evm_bytes, EvmSignatureError,
};
use near_mpc_contract_interface::types::{
    K256AffinePoint, K256Scalar, K256Signature, SignatureResponse,
};

fn signature(prefix: u8, r: u8, s: u8, recovery_id: u8) -> SignatureResponse {
    let mut point = [0u8; 33];
    point[0] = prefix;
    point[32] = r;
    let mut scalar = [0u8; 32];
    scalar[31] = s;
    SignatureResponse::Secp256k1(K256Signature {
        big_r: K256AffinePoint { affine_point: point },
        s: K256Scalar { scalar },
        recovery_id,
    })
}

#[test]
fn converts_mpc_signature_to_evm_r_s_v() {
    let encoded = mpc_signature_to_evm_bytes(signature(2, 1, 1, 1)).unwrap();
    assert_eq!(encoded[31], 1);
    assert_eq!(encoded[63], 1);
    assert_eq!(encoded[64], 28);
}

#[test]
fn rejects_recovery_id_outside_evm_range() {
    assert_eq!(
        mpc_signature_to_evm_bytes(signature(2, 1, 1, 2)).unwrap_err(),
        EvmSignatureError::InvalidRecoveryId
    );
}

#[test]
fn rejects_noncanonical_high_s() {
    assert_eq!(
        mpc_signature_to_evm_bytes(signature(2, 1, 0xff, 0)).unwrap_err(),
        EvmSignatureError::InvalidSScalar
    );
}

#[test]
fn rejects_invalid_recovery_point_encoding() {
    assert_eq!(
        mpc_signature_to_evm_bytes(signature(4, 1, 1, 0)).unwrap_err(),
        EvmSignatureError::InvalidRPoint
    );
}
