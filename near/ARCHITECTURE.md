# NEAR Integration Architecture Decision

Status: source-interface researched; end-to-end adapter not yet implemented.

## Confirmed MPC interface

The current `near/mpc` main-branch code exposes `verify_foreign_transaction`. This is not a generic arbitrary-message signing oracle. A caller submits a chain-specific transaction request containing:
- a source transaction ID;
- a bounded list of extractors (maximum 32);
- a chain-specific finality selector;
- a ForeignTx-purpose MPC domain;
- a payload version and optional expected payload hash.

For EVM chains, the current interface exposes:
- `EvmFinality::Finalized`, `Safe`, or `Latest`;
- `EvmExtractor::BlockHash`;
- `EvmExtractor::Log { log_index }`;
- extracted log fields: removed flag, log index, transaction index/hash, block hash/number, emitter address, data and topics.

The MPC response contains a `payload_hash` and recoverable secp256k1 signature. The signed message is the SHA-256 hash of the Borsh-serialized `ForeignTxSignPayload::V1`, which contains the exact request and extracted values. This means the source log can be authenticated without trusting a single application-operated RPC endpoint, but the destination must verify the correct MPC signature and the exact payload encoding. Do not treat the returned signature as an EIP-712 signature over AfterState's current SourceFact struct.

The MPC contract requires the ForeignTx domain to be active and the requested chain to be available. The request path requires an attached deposit of at least 1 yoctoNEAR. The public contract interface documents a maximum of 32 extractors. See primary source files:
- https://github.com/near/mpc/blob/main/crates/contract/src/api/foreign_chain/requests.rs
- https://github.com/near/mpc/blob/main/crates/near-mpc-contract-interface/src/types/foreign_chain/chains/evm.rs
- https://github.com/near/mpc/blob/main/crates/near-mpc-contract-interface/src/types/foreign_chain/sign_payload.rs
- https://github.com/near/mpc/blob/main/crates/near-mpc-sdk/src/foreign_chain/evm.rs

## Proposed flow

1. A source EVM contract emits a typed AfterState event containing agreement ID/version, beneficiary, asset and amount.
2. The adapter submits `verify_foreign_transaction` for the source transaction hash, requesting both `BlockHash` and the exact `Log { log_index }`, with `Finalized` finality.
3. The application may obtain a *candidate* receipt/log from an ordinary RPC endpoint. It constructs the expected `EvmLog` and uses the official SDK builder's `.with_expected_log(log_index, expected_log)` plus `.with_finality(EvmFinality::Finalized)`. The builder computes the expected Borsh payload hash. The NEAR contract submits that `expected_payload_hash`; MPC nodes independently inspect the source transaction and only return a response if their extracted payload hashes to the exact expected value. A lying candidate RPC therefore causes a mismatch rather than a false attestation, assuming the MPC verification path and provider quorum behave as specified.
4. The NEAR contract validates the candidate log's emitter, topic0/event schema, agreement ID/version, asset, amount and beneficiary against the agreement, and checks that the MPC response payload hash equals the contract-computed expected hash. The MPC signer contract verifies the threshold signature before returning the response. The NEAR contract can then request a separate Chain Signatures `sign` operation over the AfterState EIP-712 digest, using a key derived for the NEAR contract and a fixed application path. Configure the EVM verifier with that derived EVM address. This avoids a backend-held private key, while retaining explicit trust in NEAR's MPC network and the NEAR contract code.
5. The EVM registry consumes the source fact exactly once.
6. The single configured destination settlement adapter executes the payout and checks exact ERC-20 balance deltas atomically.

The existing EVM `AfterStateEIP712Verifier` can accept this derived MPC key once the owner configures its address. However, the repository does not yet contain the NEAR contract, candidate-log builder, MPC callback flow, or Chain Signatures signing call.
   - **Direct EVM verification:** implement and audit the exact Borsh + SHA-256 payload reconstruction and verify the NEAR ForeignTx MPC root public key on EVM. This avoids introducing a second trusted signer, but requires exact serialization, key/domain selection and signer-set/key-rotation handling.
   - **NEAR contract attestation:** a NEAR contract verifies/records the MPC result and emits a compact AfterState fact. The EVM side must then verify the NEAR state transition/proof or explicitly trust a separate attester. Merely storing the result on NEAR and having a backend re-sign it does not remove the backend trust assumption.
## Hard integration gates

- Confirm the exact deployed signer contract and ForeignTx domain on NEAR testnet; do not assume mainnet interface details are available identically on testnet.
- Confirm Sepolia (or chosen EVM testnet) appears in the live available foreign-chain set.
- Confirm the configured EVM finality selector and provider quorum behavior.
- Validate that the requested log index extraction returns the full canonical log and that the MPC's extracted payload binds transaction ID, finality, block hash and exact log.
- Use the official SDK for response signature verification and payload construction; pin a released version or commit.
- Determine whether the ForeignTx root key is stable and how domain rotation/resharing affects EVM verification.
- Test malformed, missing, removed, wrong-index and reorged logs.

## Trust and security assumptions to publish

The October 23, 2026 NEAR architecture announcement states that the MPC network uses 11-of-17 operators and that Read MPC requires a whitelisted quorum of at least 3-of-5 RPC providers. Treat those as dated mainnet architecture statements, not immutable guarantees or assumptions about a particular testnet deployment. Also disclose:
- RPC provider allowlist, quorum and provider independence;
- MPC signer/operator threshold and active set for the exact network;
- TEE hardware and attestation assumptions;
- source-chain finality rule and reorg handling;
- supported transaction/event formats and decoding rules;
- contract/interface version, domain/key rotation and revocation behavior;
- liveness assumptions: RPC quorum or MPC unavailability can delay observation.

This is not an EVM-verifiable light-client proof unless the destination verifies a proof with equivalent independent security. The RPC quorum and MPC operator set remain part of the trust model.

## Current implementation boundary

The EVM EIP-712 verifier currently authenticates a configured signer. It does not verify the NEAR ForeignTx MPC signature or reconstruct the Borsh payload. The EIP-712 `finalityPolicyId` is a signed policy identifier, not an independently enforced finality proof. No NEAR adapter is implemented yet.
