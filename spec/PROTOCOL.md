# AfterState Protocol — Initial Specification

Version: 0.1.0 | Status: Draft

## 1. Problem
Cross-chain workflows often collapse a source event, a relayer message and a destination transaction into one claim. They are distinct. AfterState binds a source fact and a destination outcome to one agreement identifier while preserving their separate verification requirements.

## 2. Agreement
An agreement defines a unique ID, source chain ID, source emitter, event schema, destination chain ID and settlement target, asset/amount bounds, beneficiary, expiry and verifier policy. The initial reference design treats agreements as immutable after creation. Future amendments must be versioned; attestations bind to one exact version.

## 3. Source fact
A source fact is a typed statement that a verifier observed a specific event under a declared finality policy. The current EVM schema binds an agreement version and a nonzero finality-policy identifier into the signed payload. The identifier is metadata selected by the attester; the EVM registry does not independently validate source-chain finality. The signed payload binds: protocol domain/version; source chain; source block number/hash; transaction hash and log index; emitter; event schema/topic; agreement ID/version; asset, amount and recipient; finality-policy identifier; attester key version; nonce and expiry. A transaction hash alone is insufficient. The exact log, emitter, topic and decoded fields must be checked.

## 4. Destination outcome
Destination success is not merely a successful external call. Postconditions must be deterministic and measurable atomically in the destination transaction. Examples: escrow balance increased by at least an agreed amount; beneficiary received a specified asset; order state transitioned from Pending to Fulfilled; a claim was created for the agreed beneficiary. The protocol does not claim to verify arbitrary state changes.

## 5. State machine
- Created: agreement registered.
- SourceFactAttested: valid source fact accepted.
- ReadyToSettle: agreement constraints pass.
- Settled: destination postconditions pass and settlement completes.
- Rejected: policy or verification failure.
- Expired: agreement or attestation deadline elapsed.
- RecoveryAvailable: settlement failed and a defined recovery policy permits action.

An adapter may collapse intermediate states when operations are atomic, but transitions must remain auditable.

## 6. Verification pipeline
1. Load exact agreement and version.
2. Verify proof/signature under the active verifier policy.
3. Verify domain, protocol version and destination chain.
4. Verify source chain, emitter and event schema.
5. Verify agreement, asset, amount, recipient and expiry.
6. Reject replayed nonces and duplicate source-log identities.
7. Execute destination action.
8. Check declared postconditions in the same transaction.
9. Mark settled only after all checks pass.

A failed destination postcondition must revert destination state. It cannot reverse an already-finalized source-chain action.

## 7. Cross-chain failure semantics
Cross-chain execution is not atomic. If source payment is irreversible, destination failure creates a compensation problem, not rollback. Refund guarantees require source-side escrow or a separately secured compensation path. Timeout alone does not prove destination execution did not happen; recovery must handle delayed messages and races.

## 8. NEAR integration
NEAR is the intended orchestration/attestation environment for the prototype. Before claiming an operational Read RPC / MPC adapter, validate the exact interface, signer policy, finality behavior and key lifecycle. Disclose RPC endpoints/provider independence, quorum/threshold assumptions, finality rule, key generation/rotation/revocation, verifier checks and residual trust.

## 9. MVP acceptance criteria
- Attestations cannot replay across agreements, chains, destinations or protocol versions.
- Wrong emitter, schema, chain, amount, recipient, expiry or signer is rejected.
- A source fact cannot be consumed twice.
- Completion is never recorded before postconditions pass.
- Failed postconditions revert destination state.
- Tests cover signer rotation/revocation and malformed signatures.
- Documentation distinguishes observed facts from consensus proofs.