# NEAR ForeignTx adapter

Status: first integration slice implemented; not deployed and not end-to-end.

## What it does

The contract:
1. Restricts requests to the configured authorized submitter.
2. Builds a Base EVM ForeignTx request using the pinned NEAR MPC 3.19.0 interface.
3. Requests both block hash and the exact log index at Finalized finality.
4. Supplies the locally computed expected Borsh+SHA-256 payload hash.
5. Attaches the pinned interface's required 1 yoctoNEAR deposit and configured call gas.
6. Waits for the MPC contract response and re-checks that the returned payload hash equals the expected hash.
7. Returns the MPC ForeignTx response to the submitter.

The request builder and deterministic serialization fixture live in ../foreign-tx-fixtures. The interface dependency is pinned to commit 86e2cc4d124397ec81f436b11f48bf9b6dfd062e.

## Deployment arguments

- mpc_contract_id: deployed MPC signer contract account.
- authorized_submitter: account allowed to submit candidate logs.
- foreign_tx_domain_id: live domain ID whose purpose is ForeignTx.
- mpc_call_gas_tgas: gas attached to the MPC request; must satisfy the deployed contract's live requirement.
- callback_gas_tgas: gas reserved for the adapter callback.

Do not copy domain IDs or gas values from a tutorial. Read the live testnet contract state and config first. Base must appear in the deployed contract's available foreign-chain set; the example node configuration alone is insufficient.

## Input

verify_base_log accepts a BaseEvmLogCandidate containing transaction hash, block hash/number, transaction/log indexes, emitter, removed flag, data and topics. These values are untrusted candidate receipt data. MPC independently extracts the requested values from its configured provider set and rejects a payload-hash mismatch.

The current JSON DTO uses byte arrays for hashes and addresses. A production-facing method should expose validated hex strings or a typed EVM receipt DTO and enforce the application event schema before calling MPC.

## Explicit non-claims

This contract does not yet:
- validate emitter/topic/data against an AfterState agreement;
- decode agreement ID, agreement version, asset, amount or recipient from a SourcePayment event;
- enforce source-log replay state;
- compute the AfterState EIP-712 digest;
- call the separate Sign-purpose Chain Signatures domain;
- return a destination EIP-712 signature;
- prove cross-chain atomicity or source-side refunds.

The MPC contract validates the threshold response before resolving the cross-contract promise. This adapter additionally checks the expected payload hash in its callback. The EVM destination still needs to trust the configured derived signer in the next integration stage.

## Build check

The NEAR workflow runs the fixture tests and:

    cargo check --manifest-path near/adapter/Cargo.toml --target wasm32-unknown-unknown
