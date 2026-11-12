# AfterState

**Verified conditions. Enforced outcomes.**

AfterState is an execution-assurance protocol for cross-chain workflows. It binds a specific source-chain event to a destination EIP-712 authorization, then leaves destination execution to an agreement-bound registry and settlement adapter. The intended testnet path is Base Sepolia (source) → NEAR MPC (verification and signing) → Ethereum Sepolia (destination).

> Status: experimental prototype. EVM and NEAR adapter tests/builds pass. A read-only query of live MPC testnet state succeeded, but the AfterState adapter is not deployed and no live ForeignTx verification or destination settlement has been executed. The system is unaudited and not production-ready.

## Current vertical slice

1. AfterStateSourceEmitter transfers an exact ERC-20 amount to a recipient on Base Sepolia and emits a typed SourcePayment log.
2. The NEAR adapter decodes that event and validates it against one immutable configured agreement.
3. It builds the MPC 3.18.0 ForeignTx request with Finalized finality, BlockHash + exact Log extractors, and mandatory expected_payload_hash.
4. After MPC verifies the request, the NEAR adapter computes the exact AfterState EIP-712 digest and calls the separate Sign-purpose domain using the fixed path afterstate-v1.
5. The NEAR adapter converts the MPC secp256k1 response into canonical EVM r||s||v bytes. The EVM verifier authenticates the configured signer version; the registry enforces agreement constraints and one-time source-fact consumption.

The cross-language EIP-712 digest is tested against the Solidity verifier. The ForeignTx request and Borsh payload are pinned to the NEAR MPC 3.18.0 interface commit e691bdb98d68dcf74653da58d043453a6318765f. The live testnet signer contract reported version 3.18.0 in the 2026-11-12 state snapshot.

## CI status

- Foundry: **22 tests passed, 0 failed** on commit `fca083d68e8605b01db898d487d66a5f47cb1e75`.
  https://github.com/Godwin-web3/AfterState/actions/runs/43244784448
- NEAR adapter: **19 Rust tests passed** against the exact live MPC 3.18.0 interface pin. Deployment argument validation (including negative gas-budget cases), WASM checks and release artifact upload all passed:
  https://github.com/Godwin-web3/AfterState/actions/runs/43243798186
  Artifact: [afterstate-near-adapter-wasm](https://github.com/Godwin-web3/AfterState/actions/runs/43243798186)
  Artifact archive SHA-256: `2a5738a08d6d6810c386cc782520a12055c16b8fbe2c454047dc1dac563b420e`.
- Live MPC state: read-only testnet query passed on commit `f3b439bb5bb17444d783a67415404a76dceae6d8`, confirming Base availability, provider quorum 2, and live ForeignTx / Secp256k1 Sign domains.
  https://github.com/Godwin-web3/AfterState/actions/runs/43243643428

## Important constraints

- Base Sepolia (84532) is confirmed in the live MPC testnet available-chain set. The on-chain Base RPC provider whitelist has three providers and a quorum of two. See the [live MPC testnet snapshot](near/MPC_TESTNET_SNAPSHOT.md); this is a dated observation and must be rechecked before deployment.
- The current testnet snapshot shows ForeignTx domain 3 and a Secp256k1 Sign domain 0; the adapter enforces distinct IDs. The read-only [NEAR MPC Testnet State workflow](https://github.com/Godwin-web3/AfterState/actions/workflows/near-mpc-state.yml) now discovers the live ForeignTx and Secp256k1 Sign domains automatically. With the intended adapter account ID, it can derive the exact EVM signer address for afterstate-v1 before deployment; the Sign domain is discovered automatically. Always recheck before deployment.
- The EVM verifier's signer version is an AfterState version, not the MPC domain ID. The derived EVM address for the NEAR adapter account and fixed path must be derived and configured explicitly.
- v0.1 assumes the source and destination ERC-20 use the same address and amount units. Cross-chain asset mapping and decimal conversion are not implemented.
- SourcePayment is an irreversible source-chain payment, not escrow. Destination settlement is not atomic with it; there is no automatic refund if destination execution fails. The reference flow makes a separate, pre-funded destination-chain payout of the attested asset/amount; it does not lock or burn the source asset and does not conserve supply across chains. It is an execution-assurance demo, not a bridge.
- The current NEAR adapter uses one immutable agreement per deployment and an authorized submitter. It does not yet provide a multi-agreement registry or public relayer marketplace.

## Layout

- contracts/ — EVM verifier, registry, settlement adapter, source payment emitter and testnet demo token
- test/ — Foundry tests
- script/ — Base Sepolia source and Ethereum Sepolia destination deployment scripts
- spec/ — protocol specification
- near/foreign-tx-fixtures/ — pinned MPC request builder, event decoder, EIP-712 digest and signature encoding fixtures
- near/adapter/ — NEAR ForeignTx + Chain Signatures adapter
- docs/ — security review and implementation decisions

See near/ARCHITECTURE.md, near/REQUEST_SCHEMA.md, near/MPC_TESTNET_SNAPSHOT.md, near/adapter/README.md and near/DEPLOYMENT_CHECKLIST.md for deployment gates, live-state checks, acceptance tests and trust assumptions. Copy `.env.example` to `.env` for testnet deployment; `.env` is ignored by Git.
