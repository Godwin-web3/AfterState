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
    if foreign_domain == sign_domain:
        raise SystemExit("ForeignTx and Sign domain IDs must be different")

    mpc_call_gas = integer("MPC_CALL_GAS_TGAS", "15")
    sign_call_gas = integer("SIGN_CALL_GAS_TGAS", "15")
    if mpc_call_gas < 15 or sign_call_gas < 15:
        raise SystemExit("live MPC testnet requires at least 15 Tgas for both MPC calls")

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

    args = {
        "mpc_contract_id": os.environ.get("MPC_CONTRACT_ID", "v1.signer-prod.testnet"),
        "authorized_submitter": required("AUTHORIZED_SUBMITTER"),
        "foreign_tx_domain_id": foreign_domain,
        "sign_domain_id": sign_domain,
        "mpc_call_gas_tgas": mpc_call_gas,
        "foreign_callback_gas_tgas": integer("FOREIGN_CALLBACK_GAS_TGAS", "30"),
        "sign_call_gas_tgas": sign_call_gas,
        "signature_callback_gas_tgas": integer("SIGNATURE_CALLBACK_GAS_TGAS", "30"),
        "source_emitter": fixed_bytes("BASE_SEPOLIA_SOURCE_EMITTER", 20),
        "agreement_id": fixed_bytes("AGREEMENT_ID", 32),
        "agreement_version": integer("AGREEMENT_VERSION", "1"),
        "asset": fixed_bytes("EXPECTED_ASSET_ADDRESS", 20),
        "recipient": fixed_bytes("DESTINATION_RECIPIENT", 20),
        "min_amount": str(min_amount),
        "max_amount": str(max_amount),
        "agreement_deadline": deadline,
        "finality_policy_id": fixed_bytes("FINALITY_POLICY_ID", 32),
        "destination_chain_id": destination_chain,
        "verifier_contract": fixed_bytes("AFTERSTATE_VERIFIER_ADDRESS", 20),
        "destination_registry": fixed_bytes("AFTERSTATE_REGISTRY_ADDRESS", 20),
        "signer_version": integer("SIGNER_VERSION", "1"),
    }

    if not args["mpc_contract_id"].endswith(".testnet"):
        raise SystemExit("MPC_CONTRACT_ID must be a NEAR testnet account")
    if args["agreement_version"] <= 0 or args["signer_version"] <= 0:
        raise SystemExit("agreement and signer versions must be non-zero")
    print(json.dumps(args, separators=(",", ":")))


if __name__ == "__main__":
    try:
        main()
    except BrokenPipeError:
        sys.exit(1)
