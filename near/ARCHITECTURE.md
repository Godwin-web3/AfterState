# NEAR Integration Architecture Decision

Status: researched design; not yet implemented or deployed.

## Why NEAR is relevant

NEAR's MPC infrastructure has a foreign-chain transaction verification path ("Read RPC" / foreign transaction verification). The MPC repository describes nodes independently querying foreign chains and threshold-signing observed values. NEAR's October 23, 2026 architecture update describes a whitelisted quorum of RPC providers (at least 3 of 5) for foreign-chain deposit verification, with MPC operations inside TEEs. This is materially stronger than one application-operated RPC endpoint plus one private signing key, but it is not a source-chain light client or a proof of consensus embedded in the EVM destination.

Primary references:
- https://github.com/near/mpc
- https://www.near.org/blog/chain-signatures-tee
- https://github.com/NearDeFi/shade-agent-framework

## Proposed flow

1. An EVM source contract emits a typed payment/fulfilment event containing an AfterState agreement ID and exact terms.
2. A NEAR-side adapter requests foreign-transaction verification using the supported NEAR MPC Read RPC path, if the target source chain and event payload are supported by the current deployed interface.
3. The adapter validates the returned typed observation against the agreement: chain, transaction, block, emitter, event/log identity, asset, amount, recipient, agreement version and finality policy.
4. A NEAR contract records the accepted observation and issues a domain-separated attestation for the EVM destination registry, or the adapter returns the MPC-authenticated observation through a proof format supported by the destination verifier.
5. The EVM registry consumes the source fact exactly once.
6. The single configured destination settlement adapter executes the payout and checks exact ERC-20 balance deltas atomically.

## Hard integration gate

Do not assume the Read RPC feature can sign arbitrary application-defined event payloads or that it exposes a public testnet contract/API with the exact required shape. Before implementing the adapter, inspect the current MPC contract interface, foreign-transaction verification design and supported-chain/event schema. If it only verifies a narrower transaction/deposit format, AfterState must adapt the source event to that format or use a clearly disclosed attester fallback.

## Trust and security assumptions to publish

- RPC provider allowlist and quorum threshold (the October 2026 NEAR announcement describes at least 3 of 5).
- MPC signer/operator threshold and current active set (the October 2026 announcement describes 11 of 17).
- TEE hardware and attestation assumptions.
- Source-chain finality rule and reorg handling.
- Supported transaction/event formats and decoding rules.
- Contract/interface version, key rotation and revocation behavior.
- Liveness assumptions: RPC quorum or MPC unavailability can delay observation.
- This is not an EVM-verifiable light-client proof unless the destination verifies such a proof independently.

## Current implementation boundary

The EVM EIP-712 verifier currently authenticates a configured signer. It does not itself prove that the signer used NEAR Read RPC, that the RPC quorum was independent, or that a source-chain event reached finality. The finalityPolicyId is a signed policy identifier, not an independently enforced finality proof.
