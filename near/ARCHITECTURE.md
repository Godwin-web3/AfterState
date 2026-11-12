# NEAR Integration Architecture Decision

Status: the two-stage NEAR contract path is implemented and wasm-target checked. It is not deployed or exercised against live testnet MPC state.

## Version pin

The integration baseline is NEAR MPC 3.18.0, release commit e691bdb98d68dcf74653da58d043453a6318765f. Do not code against floating main or older tutorial DTOs. Review the release notes before changing this pin:
https://github.com/near/mpc/releases/tag/3.18.0

The pinned interface sources include:
- https://github.com/near/mpc/blob/e691bdb98d68dcf74653da58d043453a6318765f/crates/contract/src/api/foreign_chain/requests.rs
- https://github.com/near/mpc/blob/e691bdb98d68dcf74653da58d043453a6318765f/crates/near-mpc-contract-interface/src/types/foreign_chain.rs
- https://github.com/near/mpc/blob/e691bdb98d68dcf74653da58d043453a6318765f/crates/near-mpc-crypto-types/src/sign.rs
- https://github.com/near/mpc/blob/e691bdb98d68dcf74653da58d043453a6318765f/crates/near-mpc-contract-interface/src/deposits.rs

## Implemented flow

1. AfterStateSourceEmitter transfers the exact ERC-20 amount to the recipient and emits SourcePayment. It is a payment emitter, not escrow.
2. NEAR adapter decodes the exact SourcePayment event and validates source emitter, agreement ID/version, asset, recipient, amount bounds, deadline and nonzero source log identity.
3. It builds a Base ForeignTx request with EvmFinality::Finalized, BlockHash and Log { log_index } extractors, payload version V1 and mandatory expected_payload_hash.
4. The MPC contract independently queries its configured providers and validates the expected payload hash. The NEAR callback checks the returned hash again.
5. The adapter constructs Eip712SourceFact from the same validated candidate and decoded event. observedAt is the source event timestamp; expiresAt is the configured agreement deadline.
6. The adapter computes the exact Solidity-compatible EIP-712 digest: domain name AfterState, version 1, destination chain ID, verifier contract, full SourceFact type hash and destination registry.
7. It calls the separate Sign-purpose MPC domain using SignRequestArgs, Payload::Ecdsa and the hard-coded derivation path afterstate-v1.
8. The final callback accepts only secp256k1 responses, requires a valid compressed R point, recovery ID 0/1, r in range and low-s, then returns 65-byte EVM r||s||v.

The ForeignTx domain and Sign domain are distinct. The EVM verifier's signer_version is an application-level version and is not the MPC domain ID.

## Exact EIP-712 mapping

The Rust digest helper is tested against AfterStateEIP712Verifier.digestFor using a deterministic fixture. It binds:
- sourceChainId = Base Sepolia 84532;
- emitter, eventSchema, agreementId;
- sourceBlockHash, sourceTxHash, sourceBlockNumber, logIndex;
- asset, amount, recipient;
- observedAt = source event timestamp, expiresAt = configured agreement deadline;
- keyVersion = configured AfterState signer_version;
- agreementVersion, finalityPolicyId, nonce;
- destinationRegistry;
- EIP-712 domain chain ID = configured destination_chain_id;
- EIP-712 verifyingContract = configured verifier_contract.

The configured EVM verifier must map signer_version to the EVM address derived from the NEAR adapter account and fixed path afterstate-v1 under the configured Sign domain.

## Deployment configuration

The NEAR adapter constructor takes:
- MPC contract account, authorized submitter;
- ForeignTx domain ID and Sign domain ID;
- separate gas allocations for MPC call, ForeignTx callback, sign call and signature callback;
- source emitter, agreement ID/version, asset, recipient, min/max amount and agreement deadline;
- finality policy ID;
- destination chain ID, verifier contract address, destination registry address and signer_version.

Both MPC calls attach 1 yoctoNEAR. Gas values and domain IDs must come from live deployed contract configuration; do not copy tutorial values. The source emitter, NEAR agreement and EVM registry agreement must share the same source emitter, asset, recipient, amount bounds, version and deadline.

## Base Sepolia configuration and live gate

The MPC 3.18.0 example testnet node configuration includes Base Sepolia with expected network fingerprint 84532:
https://github.com/near/mpc/blob/e691bdb98d68dcf74653da58d043453a6318765f/deployment/cvm-deployment/user-config.toml

That proves Base Sepolia is in the example node configuration, not that the live signer contract has Base in its available-chain set. Before deployment, query live contract state for active domains/purposes, schemes, root keys, participants, threshold, available chains and provider quorum. Derive the signer address for the deployed adapter account and fixed path; do not copy public keys from tutorials.

## v0.1 limitations

- One immutable agreement per adapter deployment; authorized submitter only.
- Source and destination ERC-20 address and units must match. No asset mapping/decimal conversion.
- SourcePayment is an irreversible transfer, not escrow. No automatic source refund.
- No NEAR pending/replay state. EVM registry provides final one-time fact consumption; public relayers need NEAR-side request controls.
- No live testnet end-to-end test yet.

## Trust boundary

Read RPC is MPC-backed foreign-chain observation using an allowlisted RPC provider set and an MPC operator threshold. It is not a light-client proof verified by the EVM destination. The EVM destination trusts the configured derived signer and does not independently verify the ForeignTx response. The trusted components include NEAR adapter code, MPC contract/network, participants/threshold, provider set, source-chain finality and configured agreement values.
