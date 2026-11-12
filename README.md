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
  https://github.com/Godwin-web3/AfterState/actions/runs...[truncated]