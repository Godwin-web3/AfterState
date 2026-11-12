#!/usr/bin/env python3
"""Build the JSON init arguments for the AfterState NEAR adapter."""

import json
import os
import sys
import time


def required(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise SystemExit(f"missing required environment variable: {name}")
    return value


def integer(name: str, default: str | None = None) -> int:
    raw = os.environ.get(name, default or "").strip()
    if not raw:
        raise SystemExit(f"missing required environment variable: {name}")
    try:
        return int(raw, 0)
    except ValueError:
        return int(raw, 10)


def fixed_bytes(name: str, length: int) -> list[int]:
    value = required(name)
    if value.startswith("0x"):
        value = value[2:]
    try:
        raw = bytes.fromhex(value)
    except ValueError as exc:
        raise SystemExit(f"{name} must be hex: {exc}") from exc
    if len(raw) != length:
        raise SystemExit(f"{name} must contain exactly {length} bytes")
    return list(raw)


def main() -> None:
    foreign_domain = integer("FOREIGN_TX_DOMAIN_ID")
    sign_domain = integer("SIGN_DOMAIN_ID")
    if not (0 <= foreign_domain < 2**64 and 0 <= sign_domain < 2**64):
        raise SystemExit("MPC domain IDs must fit in u64")
    if foreign_domain == sign_domain:
        raise SystemExit("ForeignTx and Sign domain IDs must be different")

    mpc_call_gas = integer("MPC_CALL_GAS_TGAS", "15")
    sign_call_gas = integer("SIGN_CALL_GAS_TGAS", "15")
    if mpc_call_gas < 15 or sign_call_gas < 15:
        raise SystemExit("live MPC testnet requires at least 15 Tgas for both MPC calls")
    foreign_callback_gas = integer("FOREIGN_CALLBACK_GAS_TGAS", "60")
    signature_callback_gas = integer("SIGNATURE_CALLBACK_GAS_TGAS", "30")
    if signature_callback_gas <= 0:
        raise SystemExit("signature callback gas must be non-zero")
    if foreign_callback_gas < sign_call_gas + signature_callback_gas + 10:
        raise SystemExit("ForeignTx callback gas must cover follow-on promises and reserve 10 Tgas for callback execution")

    min_amount = integer("MIN_AMOUNT")
    max_amount = integer("MAX_AMOUNT")
    if min_amount <= 0 or max_amount < min_amount or max_amount >= 2**128:
        raise SystemExit("invalid amount bounds; require 0 < MIN_AMOUNT <= MAX_AMOUNT < 2^128")

    deadline = integer("AGREEMENT_DEADLINE")
    if deadline <= int(time.time()) or deadline >= 2**64:
        raise SystemExit("AGREEMENT_DEADLINE must be a future Unix timestamp below 2^64")

    destination_chain = integer("DESTINATION_CHAIN_ID", "11155111")
    if destination_chain != 11155111:
        raise SystemExit("this deployment script targets Ethereum Sepolia (11155111)")

    agreement_version = integer("AGREEMENT_VERSION", "1")
    signer_version = integer("SIGNER_VERSION", "1")
    if not (0 < agreement_version < 2**32 and 0 < signer_version < 2**32):
        raise SystemExit("agreement and signer versions must fit non-zero u32 values")

    source_emitter = fixed_bytes("BASE_SEPOLIA_SOURCE_EMITTER", 20)
    agreement_id = fixed_bytes("AGREEMENT_ID", 32)
    asset = fixed_bytes("EXPECTED_ASSET_ADDRESS", 20)
    recipient = fixed_bytes("DESTINATION_RECIPIENT", 20)
    finality_policy_id = fixed_bytes("FINALITY_POLICY_ID", 32)
    verifier_contract = fixed_bytes("AFTERSTATE_VERIFIER_ADDRESS", 20)
    destination_registry = fixed_bytes("AFTERSTATE_REGISTRY_ADDRESS", 20)
    for name, value in [
        ("BASE_SEPOLIA_SOURCE_EMITTER", source_emitter),
        ("AGREEMENT_ID", agreement_id),
        ("EXPECTED_ASSET_ADDRESS", asset),
        ("DESTINATION_RECIPIENT", recipient),
        ("FINALITY_POLICY_ID", finality_policy_id),
        ("AFTERSTATE_VERIFIER_ADDRESS", verifier_contract),
        ("AFTERSTATE_REGISTRY_ADDRESS", destination_registry),
    ]:
        if not any(value):
            raise SystemExit(f"{name} must not be zero")

    args = {
        "mpc_contract_id": os.environ.get("MPC_CONTRACT_ID", "v1.signer-prod.testnet"),
        "authorized_submitter": required("AUTHORIZED_SUBMITTER"),
        "foreign_tx_domain_id": foreign_domain,
        "sign_domain_id": sign_domain,
        "mpc_call_gas_tgas": mpc_call_gas,
        "foreign_callback_gas_tgas": foreign_callback_gas,
        "sign_call_gas_tgas": sign_call_gas,
        "signature_callback_gas_tgas": signature_callback_gas,
        "source_emitter": source_emitter,
        "agreement_id": agreement_id,
        "agreement_version": agreement_version,
        "asset": asset,
        "recipient": recipient,
        "min_amount": min_amount,
        "max_amount": max_amount,
        "agreement_deadline": deadline,
        "finality_policy_id": finality_policy_id,
        "destination_chain_id": destination_chain,
        "verifier_contract": verifier_contract,
        "destination_registry": destination_registry,
        "signer_version": signer_version,
    }

    if not args["mpc_contract_id"].endswith(".testnet"):
        raise SystemExit("MPC_CONTRACT_ID must be a NEAR testnet account")
    print(json.dumps(args, separators=(",", ":")))


if __name__ == "__main__":
    try:
        main()
    except BrokenPipeError:
        sys.exit(1)
