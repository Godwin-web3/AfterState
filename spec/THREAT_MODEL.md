# Threat Model — v0.1

## Assets
Agreement integrity/version binding; source-event association; attestation authenticity and freshness; replay resistance; destination funds/state; correct terminal-state and recovery accounting.

## Adversaries
Malicious users submitting fabricated/replayed attestations; compromised relayers; compromised attester keys; faulty or dishonest RPC providers; source-chain reorgs or finality mismatch; malicious destination integrations; races between delayed settlement and recovery.

## Trust boundaries
**Attester:** trusted to observe/decode source events unless a stronger proof mechanism is integrated. Signature verification proves signer identity, not truth.

**RPC/finality:** RPC-backed observations inherit provider integrity and availability assumptions. Multiple endpoints are not necessarily independent. Finality must be chain-specific and explicit.

**Destination adapter:** predicates are only as strong as their implementation. They must be deterministic, atomic with settlement and bound to the agreement. External calls/callbacks require reentrancy and adversarial-call analysis.

## Principal attack classes
1. Cross-domain replay: bind signatures to protocol domain/version, destination chain, contract, agreement and nonce.
2. Log substitution: bind source block hash, tx hash, log index, emitter, schema and decoded fields.
3. Stale observation: enforce chain-specific finality policy and observation metadata.
4. Signer compromise: support key versioning, revocation, bounded expiry and monitoring.
5. Duplicate settlement: consume nonce and source-log identity exactly once.
6. False success: check application-level postconditions atomically.
7. Recovery race: do not refund merely because a timeout elapsed; reconcile delayed settlement and escrow-specific recovery.

## Out of scope for v0.1
Arbitrary source consensus proofs, arbitrary off-chain facts, compromised underlying chains, universal cross-chain rollback, and formal verification of arbitrary callbacks.

## Security posture
Experimental and unaudited. Do not use to custody real assets.