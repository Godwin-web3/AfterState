# NEAR Integration Architecture Decision

Status: MPC interface researched and pinned; deterministic serialization fixture added; NEAR contract and end-to-end adapter not yet implemented.

## Version pin

The integration baseline is NEAR MPC 3.19.0, release commit 86e2cc4d124397ec81f436b11f48bf9b6dfd062e. Do not code against floating main or older tutorial DTOs. Release 3.19.0 includes changes to ForeignTx whitelist/enforcement behavior, attestation expiry and ECDSA signing internals. Review release notes before changing this pin:
https://github.com/near/mpc/releases/tag/3.19.0

In this release the foreign-chain DTOs are consolidated in:
https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/near-mpc-contract-interface/src/types/foreign_chain.rs

The request and contract entrypoint definitions are:
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/near-mpc-contract-interface/src/types/foreign_chain/request.rs
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/contract/src/api/foreign_chain/requests.rs
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/near-mpc-crypto-types/src/sign.rs
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/near-mpc-contract-interface/src/deposits.rs

## Confirmed ForeignTx interface

The payable verify_foreign_transaction method accepts VerifyForeignTransactionRequestArgs:
- request: a chain-specific ForeignChainRpcRequest;
- domain_id: a DomainId whose live purpose must be ForeignTx;
- payload_version: ForeignTxPayloadVersion::V1 (wire integer 1);
- expected_payload_hash: optional Hash256, which AfterState MUST supply.

For EVM:
- EvmFinality: Latest, Safe or Finalized.
- EvmExtractor: BlockHash or Log { log_index }.
- EvmLog: removed, log_index, transaction_index, transaction_hash, block_hash, block_number, address, data and topics.
- Base is a ForeignChainRpcRequest variant and maps to chain ID 84532 in the MPC testnet node configuration.

The MPC constructs ForeignTxSignPayload::V1 from the exact request and extracted values, Borsh-serializes it and SHA-256 hashes the bytes. The response carries payload_hash and a recoverable secp256k1 signature. The signer contract verifies the response signature against the domain root key and rejects a payload_hash that differs from expected_payload_hash.

This is not an EIP-712 signature over AfterState SourceFact. Do not feed the ForeignTx signature into the EVM EIP-712 verifier.

## Base Sepolia configuration and live gate

The MPC 3.19.0 example testnet node configuration includes Base Sepolia and expected network fingerprint 84532:
https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/deployment/cvm-deployment/user-config.toml

That proves Base Sepolia is in the example node configuration, not that the currently deployed testnet contract has Base in its live available-chain set. The contract checks get_available_foreign_chains() before accepting a request. Query the deployed contract at runtime before committing to the demo path.

## Exact request construction

The JSON request shape is specified in near/REQUEST_SCHEMA.md. It uses:
- ForeignChainRpcRequest::Base;
- EvmFinality::Finalized;
- both BlockHash and Log { log_index } extractors;
- payload_version = 1;
- expected_payload_hash = Some(the locally computed expected V1 hash).

The expected hash is critical. An ordinary RPC receipt is only a candidate. The NEAR adapter must construct the exact expected EvmLog and calculate the exact MPC payload hash before submitting. MPC nodes independently inspect the source transaction through their configured providers. If their extracted values differ, the contract rejects the response.

Do not implement a hand-written, partial Borsh encoder. The repo now has near/foreign-tx-fixtures, which imports the exact 3.19.0 interface commit and tests a deterministic Base EVM payload hash. This fixture is the serialization regression guard. Any MPC upgrade must review the type changes and regenerate fixtures deliberately.

## Proposed two-stage NEAR flow

1. User submits a source transaction hash and log index to the NEAR AfterState contract.
2. Contract validates agreement configuration and obtains a candidate receipt/log from a configured RPC path or receives candidate values as input. Candidate values are untrusted until MPC confirms them.
3. Contract validates candidate emitter, topic0/schema, agreement ID/version, asset, amount and beneficiary against the agreement. Reject removed logs, wrong transaction hash, wrong log index, malformed topics/data and mismatched block metadata.
4. Contract constructs exact EVM request and EvmLog values, computes the official V1 Borsh+SHA-256 payload hash, and calls verify_foreign_transaction with expected_payload_hash set.
5. Callback checks promise success and exact returned payload hash. No accepted fact is stored on timeout or error.
6. After ForeignTx success, contract constructs the AfterState EIP-712 digest from the same validated fact and calls the separate sign method on a Sign-purpose domain. The path is fixed in contract code and is not caller-controlled. The payload is not caller-supplied.
7. Callback validates the returned signature and returns the EIP-712 signature. Configure the EVM verifier with the derived EVM address for this NEAR contract account and fixed path.
8. EVM registry consumes the fact once; the single configured settlement adapter claims and settles it once.

The ForeignTx-purpose domain and Sign-purpose domain are distinct. The contract must query live state and configure their IDs rather than assume a domain number. The normal sign API uses a Payload::Ecdsa payload and SignRequestArgs with path, payload_v2 and domain_id. The signer contract derives the key from the calling NEAR account and path; the path must therefore be stable and hard-coded for this adapter.

## Live deployment and liveness gates

- Query live testnet signer contract state: contract version, active domains and purposes, schemes, participants, threshold, root public keys, and available foreign chains.
- Query the live Base availability and provider configuration/quorum. Example node config is not proof of live availability or provider independence.
- Confirm contract/node compatibility with MPC 3.19.0 before making requests.
- Read current Sign-domain root key and derive the EVM address for this NEAR contract account plus fixed path. Do not copy public keys from tutorials.
- ForeignTx requests require the configured gas attachment and SIGN_DEPOSIT_YOCTONEAR (1 yoctoNEAR in the pinned interface); gas must be set to the deployed contract's current requirement, not a stale hardcoded tutorial value.
- Test wrong expected hash, wrong log index, wrong emitter/topic/data, missing transaction, non-finalized receipt, provider-quorum failure, callback timeout, signer rotation and domain migration.

## Trust and security assumptions

Read RPC is MPC-backed foreign-chain observation using an allowlisted RPC provider set and an MPC operator threshold. It is not a light-client proof verified by the EVM destination. The EVM EIP-712 verifier still only verifies the configured derived signer. The trust boundary includes the NEAR contract code, MPC contract, MPC participants/threshold, TEE assumptions, foreign-chain RPC provider set and source-chain finality configuration.

Do not copy mainnet threshold or provider quorum numbers into testnet claims. Publish the exact live values observed for the deployed testnet contract and active node set.
