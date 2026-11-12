# AfterState

**Verified conditions. Enforced outcomes.**

AfterState is an execution-assurance protocol for cross-chain workflows. It separates two assertions: (1) a precisely scoped event occurred on a specified source chain, and (2) the destination action completed and its declared postconditions hold. A source event does not prove destination execution; an EVM-successful transaction does not prove the intended application outcome.

> Status: experimental prototype. The EVM reference contracts compile and pass the current Foundry suite. The NEAR Read RPC / MPC adapter remains under implementation. The system is unaudited and not production-ready.

## Design goals
- Typed, domain-separated source attestations.
- Explicit chain, emitter, event, agreement, amount, recipient and asset constraints.
- Replay protection, expiry and registry-level one-time settlement claims.
- Destination ERC-20 settlement guarded by measurable balance-delta postconditions.
- Explicit trust assumptions and recoverable failure states.
- No false claims that RPC-backed observations are trustless consensus proofs.

## Trust model
The current EVM prototype authenticates an authorized EIP-712 attester. That proves who signed a statement, not that the source observation is true. The intended NEAR Read RPC integration will add MPC-backed foreign-transaction observation, but the EVM destination will still trust the configured NEAR-derived signer and the NEAR MPC/contract trust boundary. Read RPC is not an EVM-verifiable light-client proof.

## State model
Created → SourceFactAttested → ReadyToSettle → Settled. Failure/recovery states include Rejected, Expired and RecoveryAvailable. Cross-chain execution is not atomic. An irreversible source transfer cannot be rolled back by a destination contract. Refund guarantees require source escrow or a separately specified compensation mechanism.

## Layout
- contracts/ — EVM reference contracts
- test/ — Foundry tests
- spec/ — protocol and threat-model specifications
- near/ — NEAR adapter research and integration specifications
- docs/ — implementation notes and decisions

## Development
Solidity 0.8.24, Foundry. The current CI workflow runs forge test -vvv. The last verified run passed 15 tests across two suites on commit 2a567b12ab4eb5b1c3fa37a65a25431b3729944a. See the successful run:
https://github.com/Godwin-web3/AfterState/actions/runs/43207215276

The EVM reference includes an EIP-712 verifier with versioned signer keys, signer revocation, destination-registry binding and low-s signature checks; an agreement registry; global one-time settlement claims; and a pre-funded ERC-20 settlement adapter that checks exact sender/recipient balance deltas.

## Non-goals for v0.1
No arbitrary source-chain consensus proofs, arbitrary off-chain fact proofs, cross-chain atomicity, or universal EVM state-diff verification.

## License
MIT. See LICENSE.

## NEAR integration status
Base Sepolia (chain ID 84532) is present in the current NEAR MPC example testnet node configuration. This does not prove the live testnet signer contract currently advertises Base as available. The deployed ForeignTx domain, Sign domain, chain availability, provider quorum and root key must be checked against live testnet state before an end-to-end claim. See near/ARCHITECTURE.md and near/REQUEST_SCHEMA.md. No NEAR adapter is claimed as implemented yet.

## Security review
See docs/SECURITY_REVIEW.md for implemented controls, trust boundaries, fixes and outstanding tests.
