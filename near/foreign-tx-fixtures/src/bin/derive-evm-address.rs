use afterstate_near_foreign_tx_fixtures::evm_address_from_mpc_public_key_json;
use std::{env, fs};

fn main() {
    let path = env::args().nth(1).expect("usage: derive-evm-address <public-key-json>");
    let json = fs::read_to_string(path).expect("read public-key JSON");
    let address = evm_address_from_mpc_public_key_json(&json)
        .expect("public key must be a valid MPC secp256k1 key");
    println!("0x{}", hex::encode(address));
}
