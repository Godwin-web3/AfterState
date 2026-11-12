# Security Review — Initial Cross-Chain Prototype

Status: implementation review only; no external audit. EVM and NEAR components are experimental.

## Verified implementation

Latest verified runs: [Foundry CI](https://github.com/Godwin-web3/AfterState/actions/runs/43239292839) and [NEAR adapter CI](https://github.com/Godwin-web3/AfterState/actions/runs/43211514663).

- Foundry CI passed 20 tests on commit 1a9c137a015ad214c67eb5951b4cc7e4b3da80fd.
- NEAR Rust CI passed the pinned fixtures, adapter unit tests and WASM checks on commit 15958e60b6f037bfc317a5d30b12e19d2a482229. The expanded workflow additionally validates the live-state query script and builds release WASM; its run on commit 094cba9f2c4525a18b54bf395473adceac0a1d71 remains in progress.
- EIP-712 digest fixture matches the Solidity verifier byte-for-byte for a deterministic fact, destination chain, verifier and registry.
- MPC ForeignTx fixture uses NEAR MPC 3.19.0 pinned commit 86e2cc4d124397ec81f436b11f48bf9b6dfd062e and asserts both exact Borsh bytes and SHA-256 payload hash.
- SourcePayment decoder checks exact topic0, topic count, ABI data length, canonical address padding, integer bounds and nonzero fields.
- NEAR adapter validates the candidate SourcePayment against its configured source emitter and agreement before calling MPC.
- NEAR adapter rejects a source event timestamp equal to the agreement deadline, because the EVM registry requires expiresAt to be strictly later than observedAt.
- Destination settlement failure rolls back the registry's cross-contract fact claim atomically; a Foundry regression test verifies that an underfunded attempt can be retried after funding.
- NEAR adapter requires expected_payload_hash, waits for the MPC ForeignTx callback, checks the returned hash, then calls the distinct Sign-purpose domain with fixed derivation path afterstate-v1.
- MPC secp256k1 signature conversion checks recovery ID, compressed R-point prefix, r range and low-s canonicality before returning EVM r||s||v.
- EVM EIP-712 verifier binds signatures to chain ID and verifier address, and supports versioned signer keys and revocation.
- EVM registry checks agreement constraints, consumes nonces and source transaction/log identities once, and globally claims each accepted fact once for the configured settlement adapter.
- Source emitter transfers the exact requested ERC-20 amount before emitting SourcePayment; fee-on-transfer and inexact transfers revert.

## Important limitations and trust boundaries

1. The NEAR adapter has not been deployed or exercised against the live NEAR testnet MPC contract. The deployed ForeignTx domain, Sign domain, Base availability, provider quorum, root key and gas requirements remain live deployment gates.
2. Read RPC is MPC-backed observation using configured RPC providers and MPC operators; it is not an EVM-verifiable light-client proof.
3. The EVM destination trusts the configured derived signer. It does not independently verify the NEAR ForeignTx signature.
4. The NEAR adapter currently supports one immutable agreement per deployment and an authorized submitter. It has no multi-agreement storage or permissionless relayer.
5. v0.1 requires the source ERC-20 address to equal the destination ERC-20 address and assumes matching units/decimals. Asset mapping and conversion are not implemented.
6. SourcePayment transfers funds irreversibly on the source chain. Destination settlement is not atomic; the source emitter is not escrow and does not provide refunds.
7. observedAt in the EIP-712 fact is populated from the source event's block timestamp, not the NEAR callback time.
8. The configured finalityPolicyId is signed metadata. The actual MPC request uses EvmFinality::Finalized, but the EVM registry does not independently validate this policy ID.
9. Settlement uses pre-funded destination inventory and conventional boolean-returning ERC-20s with exact balance-delta checks. Fee-on-transfer, rebasing and nonstandard assets are unsupported.
10. The EVM registry owner controls agreement creation, verifier replacement and settlement adapter replacement. Production deployment needs constrained governance or immutable configuration.

## Fixed during implementation

- Agreement IDs cannot be recreated after deactivation.
- One accepted fact cannot be paid by multiple settlement adapters.
- Source-log replay identity excludes block hash so re-inclusion cannot create a second consumable identity.
- Zero-address assets and zero finality policy IDs are rejected.
- Source emitter rejects inexact ERC-20 transfers.
- The MPC ForeignTx Borsh fixture was corrected to use one-byte Borsh enum discriminants and is pinned against official 3.19.0 types.

## Required next validation

- Query live testnet MPC state for active domain IDs/purposes, schemes, root keys, participants, threshold and available chains.
- Derive the EVM signer address for the deployed NEAR adapter account plus fixed path afterstate-v1; configure the EVM verifier at the matching signer_version and verify recovery on-chain.
- Deploy source emitter and identical-address test token on Base Sepolia and the destination EVM testnet.
- Deploy the NEAR adapter with matching agreement/deadline/token/recipient/finality settings and live MPC gas parameters.
- Submit a real Base Sepolia SourcePayment transaction; verify MPC ForeignTx, NEAR Chain Signature, EVM consumeSourceFact and destination settlement end-to-end.
- Test wrong payload hash, wrong log index, removed log, wrong emitter/topic/data, non-finalized receipt, provider quorum failure and both callback failure paths.
- Add stateful NEAR replay/pending request tracking before exposing a public relayer.
- Add source/destination asset mapping before supporting assets with different addresses or decimals.
- Test fee-on-transfer token rollback, reentrant token callbacks, signer rotation/revocation and owner compromise.

## Primary references

- https://github.com/near/mpc/releases/tag/3.19.0
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/contract/src/api/foreign_chain/requests.rs
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/near-mpc-contract-interface/src/types/foreign_chain.rs
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/near-mpc-crypto-types/src/sign.rs
