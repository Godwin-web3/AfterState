# NEAR adapter

Status: two-stage ForeignTx + Chain Signatures adapter implemented and WASM-target checked. Not deployed or tested end-to-end against live MPC testnet.

## What it does

1. Restricts requests to the configured authorized submitter.
2. Decodes the exact SourcePayment event emitted by AfterStateSourceEmitter.
3. Validates emitter, agreement ID/version, asset, recipient, amount bounds, deadline and source log identity against one immutable agreement configured at initialization.
4. Builds a Base ForeignTx request using MPC 3.19.0 types, Finalized finality, BlockHash + exact Log extractors, and mandatory expected_payload_hash.
5. Calls verify_foreign_transaction with 1 yoctoNEAR and configured gas.
6. In the callback, checks the MPC response payload hash against the expected hash.
7. Constructs the exact AfterState EIP-712 digest from the validated event and candidate log.
8. Calls the separate Sign-purpose domain with Payload::Ecdsa and fixed derivation path afterstate-v1.
9. Converts the MPC secp256k1 signature to canonical 65-byte EVM r||s||v bytes. Invalid recovery IDs, point encodings, r values and high-s signatures are rejected.

The request builder, event decoder, EIP-712 digest and signature encoder live in ../foreign-tx-fixtures and are pinned to MPC commit 86e2cc4d124397ec81f436b11f48bf9b6dfd062e.

## Deployment arguments

- mpc_contract_id and authorized_submitter.
- foreign_tx_domain_id and sign_domain_id (distinct live domains). The adapter constructor now rejects identical IDs; this is a hard separation between the ForeignTx observation purpose and Sign authorization purpose.
- mpc_call_gas_tgas, foreign_callback_gas_tgas, sign_call_gas_tgas and signature_callback_gas_tgas.
- source_emitter, agreement_id/version, asset, recipient, min/max amount, agreement_deadline and finality_policy_id.
- destination_chain_id, verifier_contract, destination_registry and signer_version.

Both MPC calls attach 1 yoctoNEAR. Domain IDs, gas allocations, available chains, provider quorum and root keys must be read from live testnet state. Never copy domain IDs or gas values from a tutorial. Run the NEAR MPC Testnet State workflow in GitHub Actions; provide the deployed adapter account and Sign domain ID to retrieve the derived public key and EVM address.

## EVM signer configuration

The signer key is derived by MPC for the NEAR adapter account and fixed path afterstate-v1 under the configured Sign domain. Derive the corresponding EVM address from the live Sign-domain root key and configure the EVM AfterStateEIP712Verifier at signer_version with exactly that address. signer_version is an application-level EVM version, not an MPC domain ID.

The destination_chain_id must equal the EVM chain ID. verifier_contract must be the deployed AfterStateEIP712Verifier address; destination_registry must be the deployed AfterStateRegistry address. The EVM registry agreement must mirror the NEAR agreement.

## v0.1 constraints

- One immutable agreement per adapter deployment and one authorized submitter.
- Source and destination ERC-20 address and amount units must match. No token mapping or decimal conversion exists.
- SourcePayment transfers funds irreversibly to the recipient; it is not escrow and cannot be refunded automatically if destination settlement fails.
- observedAt is populated from the source event's block timestamp.
- The adapter does not store pending or consumed source-log state. The EVM registry enforces final replay/consumption; add NEAR-side controls before supporting public relayers.
- Live testnet domains, key derivation, gas and end-to-end execution remain unverified.

## Build and tests

The NEAR workflow runs the Rust fixture tests and:

    cargo check --manifest-path near/adapter/Cargo.toml --target wasm32-unknown-unknown
