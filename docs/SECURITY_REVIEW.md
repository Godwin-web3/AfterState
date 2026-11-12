# Security Review — Initial EVM Reference

Status: design review notes; no external audit. The contracts are experimental.

## Implemented controls

- EIP-712 domain binds signatures to chain ID and verifier contract.
- Typed fact binds source chain, emitter, schema, agreement, block/transaction/log identity, asset, amount, recipient, timestamps, key version, agreement version, finality-policy ID, nonce and destination registry.
- Registry checks agreement constraints before signature verification.
- Nonces and source transaction/log identities are consumed once.
- Source-log identity excludes block hash so a transaction re-included after a reorg cannot be accepted as a second payment.
- Signer versions are immutable once configured and can be revoked.
- Settlement is restricted to one owner-configured adapter; the registry globally claims each accepted fact once.
- Settlement uses checks-effects-interactions, a reentrancy guard, and exact escrow/recipient ERC-20 balance deltas.

## Important limitations

1. The EIP-712 verifier authenticates an authorized signer only. It does not prove that the source-chain observation is true.
2. `finalityPolicyId` is signed metadata. It is not an on-chain finality proof.
3. The current settlement adapter pays from pre-funded destination inventory. It does not atomically lock or refund the source-chain payment.
4. The adapter supports conventional boolean-returning ERC-20s. Fee-on-transfer, rebasing, malicious, or non-standard tokens may fail or behave unexpectedly. Asset allowlisting and token-specific review are required.
5. The registry owner controls agreements, verifier replacement and settlement-adapter replacement. Production deployment needs governance/timelock or a constrained immutable configuration.
6. The source-chain event schema is represented by a topic/schema hash and must be decoded/validated exactly by the attestation producer.
7. Reorg handling depends on the source-observation system's finality policy. Preventing duplicate log identity does not prove finality.
8. The implementation has not yet been compiled, fuzzed, or run through Foundry CI in this environment.

## Fixed during implementation

- Agreement IDs could initially be reused after deactivation. Added a permanent `agreementEverCreated` guard.
- An accepted fact could initially be paid by multiple independently deployed settlement adapters. Added a single configured adapter and registry-level one-time claim.
- Source-log replay identity initially included block hash. Removed block hash from the replay key to prevent re-inclusion from creating a second consumable identity.
- The registry initially allowed zero-address assets even though the settlement adapter only supports ERC-20. Zero-address assets are now rejected at agreement creation.

## Required next tests

- Fee-on-transfer token causes atomic revert and does not leave the fact claimed.
- Reentrant token callback cannot settle twice.
- Adapter replacement and owner compromise assumptions are documented and tested.
- Signer rotation/revocation edge cases and malformed ECDSA signatures.
- Fuzz source facts across all agreement fields and timestamps.
- Verify the exact NEAR ForeignTx MPC signature and payload-to-EIP712 bridge.
