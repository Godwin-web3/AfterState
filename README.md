# AfterState

**Verified conditions. Enforced outcomes.**

AfterState is an execution-assurance protocol for cross-chain workflows. It binds a specific source-chain event to a destination EIP-712 authorization, then leaves destination execution to an agreement-bound registry and settlement adapter.

> Status: experimental prototype. EVM tests and NEAR adapter compile checks pass. The NEAR adapter is not deployed or tested end-to-end against live MPC testnet state. The system is unaudited and not production-ready.

## Current vertical slice

1. AfterStateSourceEmitter transfers an exact ERC-20 amount to a recipient on Base Sepolia and emits a typed SourcePayment log.
2. The NEAR adapter decodes that event and validates it against one immutable configured agreement.
3. It builds the MPC 3.19.0 ForeignTx request with Finalized finality, BlockHash + exact Log extractors, and mandatory expected_payload_hash.
4. After MPC verifies the request, the NEAR adapter computes the exact AfterState EIP-712 digest and calls the separate Sign-purpose domain using the fixed path afterstate-v1.
5. The NEAR adapter converts the MPC secp256k1 response into canonical EVM r||s||v bytes. The EVM verifier authenticates the configured signer version; the registry enforces agreement constraints and one-time source-fact consumption.

The cross-language EIP-712 digest is tested against the Solidity verifier. The ForeignTx request and Borsh payload are pinned to NEAR MPC 3.19.0 commit 86e2cc4d124397ec81f436b11f48bf9b6dfd062e.

## CI status

- Foundry: **20 tests passed, 0 failed** on commit `1a9c137a015ad214c67eb5951b4cc7e4b3da80fd`.
  https://github.com/Godwin-web3/AfterState/actions/runs/43239292839
- NEAR adapter: fixture tests, adapter unit tests and WASM checks passed on commit `15958e60b6f037bfc317a5d30b12e19d2a482229`.
  https://github.com/Godwin-web3/AfterState/actions/runs/43211514663
- The expanded NEAR workflow now also validates the live-state query script and builds the release WASM. That run is still in progress on commit `094cba9f2c4525a18b54bf395473adceac0a1d71`; do not treat the expanded workflow as green until it completes.

## Important constraints

- Base Sepolia (84532) appears in the MPC 3.19.0 example testnet node configuration; live signer contract availability and provider quorum still require a live state query.
- The ForeignTx domain ID and Sign domain ID are separate and must be read from the deployed contract. Neither is hardcoded. The manual [NEAR MPC Testnet State workflow](https://github.com/Godwin-web3/AfterState/actions/workflows/near-mpc-state.yml) queries contract state, domains, available chains, provider configuration and gas settings; with the deployed adapter account and Sign domain ID, it also derives the EVM signer address for afterstate-v1.
- The EVM verifier's signer version is an AfterState version, not the MPC domain ID. The derived EVM address for the NEAR adapter account and fixed path must be derived and configured explicitly.
- v0.1 assumes the source and destination ERC-20 use the same address and amount units. Cross-chain asset mapping and decimal conversion are not implemented.
- SourcePayment is an irreversible source-chain payment, not escrow. Destination settlement is not atomic with it; there is no automatic refund if destination execution fails.
- The current NEAR adapter uses one immutable agreement per deployment and an authorized submitter. It does not yet provide a multi-agreement registry or public relayer marketplace.

## Layout

- contracts/ — EVM verifier, registry, settlement adapter and source payment emitter
- test/ — Foundry tests
- spec/ — protocol specification
- near/foreign-tx-fixtures/ — pinned MPC request builder, event decoder, EIP-712 digest and signature encoding fixtures
- near/adapter/ — NEAR ForeignTx + Chain Signatures adapter
- docs/ — security review and implementation decisions

See near/ARCHITECTURE.md, near/REQUEST_SCHEMA.md, near/adapter/README.md and near/DEPLOYMENT_CHECKLIST.md for deployment gates, live-state checks, acceptance tests and trust assumptions.
