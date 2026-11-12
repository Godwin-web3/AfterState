# AfterState

**Verified conditions. Enforced outcomes.**

AfterState is an execution-assurance protocol for cross-chain workflows. It separates two assertions: (1) a precisely scoped event occurred on a specified source chain, and (2) the destination action completed and its declared postconditions hold. A source event does not prove destination execution; an EVM-successful transaction does not prove the intended application outcome.

> Status: early experimental implementation. The repository contains an initial specification and a minimal EVM source-fact registry. It is unaudited and not production-ready.

## Design goals
- Typed, domain-separated source attestations.
- Explicit chain, emitter, event, agreement, amount, recipient and asset constraints.
- Replay protection and expiry.
- Destination settlement guarded by measurable postconditions (next milestone).
- Explicit trust assumptions and recoverable failure states.
- No false claims that RPC-backed observations are trustless consensus proofs.

## Trust model
The initial design has an authorized attester/verifier boundary. A future NEAR Read RPC / MPC adapter must disclose RPC-provider independence, quorum, threshold-key security, signer lifecycle, finality policy and exactly what is verified. A signature proves who signed a statement, not that the statement is true.

## State model
Created → SourceFactAttested → ReadyToSettle → Settled. Failure/recovery states include Rejected, Expired and RecoveryAvailable. Cross-chain execution is not atomic. An irreversible source transfer cannot be rolled back by a destination contract. Refund guarantees require source escrow or a separately specified compensation mechanism.

## Layout
- contracts/ — EVM reference contracts
- test/ — Foundry tests
- spec/ — protocol and threat-model specifications
- near/ — NEAR adapter research and integration
- docs/ — implementation notes and decisions

## Development
Solidity 0.8.24, Foundry. First milestone: agreement-bound source-fact verification and one-time consumption. Second milestone: a narrow pre-funded ERC-20 settlement adapter now releases the attested asset/amount to the attested recipient and checks exact sender/recipient balance deltas in the same transaction. Settlement and verifier tests are authored but have not yet been executed.

## Non-goals for v0.1
No arbitrary source-chain consensus proofs, arbitrary off-chain fact proofs, cross-chain atomicity, or universal EVM state-diff verification.

## License
MIT. See LICENSE.

## NEAR integration status
NEAR Read RPC / foreign-chain transaction verification is the intended source-observation path. Its exact deployed interface and supported event formats still require validation; see [near/ARCHITECTURE.md](near/ARCHITECTURE.md). No NEAR adapter is claimed as implemented yet.

## Current implementation
The EVM reference now includes an EIP-712 verifier with versioned signer keys, signer revocation, destination-registry binding and low-s signature checks, plus a Foundry test suite covering valid consumption, replay, registry-domain substitution, revoked signers, expiry and agreement constraints. These tests are authored but have not yet been executed in this environment.
