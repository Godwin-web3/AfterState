# NEAR ForeignTx request contract

Status: interface contract pinned to NEAR MPC 3.19.0, release commit 86e2cc4d124397ec81f436b11f48bf9b6dfd062e. This is an integration specification, not a deployed or end-to-end adapter.

## Source of truth

The inspected 3.19.0 source exposes:
- verify_foreign_transaction(VerifyForeignTransactionRequestArgs) as a payable method.
- VerifyForeignTransactionRequestArgs with request, domain_id, payload_version and optional expected_payload_hash.
- ForeignTxPayloadVersion::V1 serialized as integer 1.
- ForeignChainRpcRequest::Base(EvmRpcRequest).
- EVM extractors BlockHash and Log { log_index }.
- EvmFinality values Latest, Safe and Finalized.
- EvmLog fields removed, log_index, transaction_index, transaction_hash, block_hash, block_number, address, data and topics.
- VerifyForeignTransactionResponse with payload_hash and signature.

Pinned primary sources:
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/contract/src/api/foreign_chain/requests.rs
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/near-mpc-contract-interface/src/types/foreign_chain/request.rs
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/near-mpc-contract-interface/src/types/foreign_chain/rpc_request.rs
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/near-mpc-contract-interface/src/types/foreign_chain.rs

In 3.19.0, the foreign-chain DTOs were consolidated into foreign_chain.rs. Do not copy paths from older releases.

## Base Sepolia request shape

The JSON wire shape is:

    {
      "request": {
        "Base": {
          "tx_id": "<32-byte transaction hash as hex>",
          "extractors": [
            "BlockHash",
            { "Log": { "log_index": 0 } }
          ],
          "finality": "Finalized"
        }
      },
      "domain_id": "<live ForeignTx-purpose domain id>",
      "payload_version": 1,
      "expected_payload_hash": "<32-byte expected SHA-256 payload hash>"
    }

This is a schema illustration, not a ready-to-submit request. The domain ID must be read from deployed contract state and checked for ForeignTx purpose. Do not assume a domain number from a tutorial or another deployment. Transaction hash, log index and expected log fields must come from a candidate receipt, then be independently checked by MPC.

The 3.19.0 example testnet node configuration includes Base Sepolia with expected network fingerprint 84532. This proves Base Sepolia is in the example testnet node configuration; it does not prove that the live signer contract currently advertises Base as available. The contract checks its live available-chain set before accepting the request. A live contract query and real end-to-end request remain mandatory.

## Why expected_payload_hash is mandatory

The MPC source constructs ForeignTxSignPayload::V1 containing the exact request and extracted values, Borsh-serializes it, then SHA-256 hashes the bytes. The nodes sign that 32-byte hash. When expected_payload_hash is supplied, the MPC contract compares the MPC response payload hash with the expected hash and rejects a mismatch. The field is optional in the interface, but AfterState must not omit it.

The candidate EVM log is not trusted merely because an ordinary RPC returned it. The NEAR adapter must construct the exact expected EVM request and expected extracted values, calculate the exact Borsh payload hash, and pass that hash to MPC. MPC nodes independently query their configured foreign-chain RPC providers. A mismatch must fail closed.

Do not hand-roll a partial encoding. AfterState now includes a Rust fixture crate that imports the exact NEAR MPC 3.19.0 interface commit and checks a deterministic Base Sepolia V1 payload hash. Borsh enum discriminants, field order, bounded extractor encoding, hash byte order and the ForeignChainRpcRequest::Base variant index are consensus-critical inputs to the hash. Updating the pinned MPC commit requires reviewing the DTO changes and deliberately updating the fixture.

## Required two-stage NEAR flow

1. Validate the candidate source receipt/log against the AfterState agreement: source chain, configured emitter, exact log index, event topic/schema, agreement ID/version, asset, amount and beneficiary. Reject removed logs and malformed topics/data.
2. Construct the exact ForeignTx request using Base, Finalized, and both BlockHash and Log { log_index } extractors.
3. Construct the exact expected EvmLog, calculate the V1 Borsh+SHA-256 payload hash, and call verify_foreign_transaction with expected_payload_hash set.
4. In the callback, require the returned payload hash to equal the locally expected hash. Handle PromiseError and timeout as failed/pending observation; never accept the fact on callback failure.
5. Only after ForeignTx verification succeeds, construct the destination EIP-712 digest from the same validated fact and call the separate sign method on a Sign-purpose domain. Use a fixed application derivation path; never accept a caller-selected path or digest.
6. In the second callback, validate the derived EVM signer and return the EIP-712 signature. Configure the EVM verifier with the exact derived address for this NEAR contract and fixed path.
7. EVM registry consumes the attestation once; the configured settlement adapter claims and settles it once.

The ForeignTx-purpose key and Sign-purpose key are distinct. Do not use the ForeignTx signature as if it were the AfterState EIP-712 signature.

## 3.19.0 release-specific considerations

3.19.0 changes the MPC implementation and contract beyond a simple patch: it removes ECDSA presignatures, changes ForeignTx whitelist/enforcement behavior, and takes attestation expiry from collateral. It also includes a provider-matching fix. These changes make it especially important to validate the deployed contract state and node/contract version compatibility rather than relying on older testnet tutorials.

Release notes: https://github.com/near/mpc/releases/tag/3.19.0

## Deployment and liveness gates

- Read live signer contract state on NEAR testnet to confirm contract account, active ForeignTx domain ID, active Sign domain ID, protocol/scheme, participants, threshold, root keys and available chains.
- Confirm the deployed contract exposes the inspected method signatures and active MPC nodes support Base Sepolia.
- Read live provider quorum/allowlist and confirm provider independence; example node config is not proof of deployed quorum.
- Read the current signer root key and derive the EVM address for the NEAR contract account plus fixed path. Do not copy a public key from a stale tutorial.
- Use the method's live gas requirement and required attached deposit; do not hardcode tutorial values.
- Test rejection for wrong payload hash, wrong log index, emitter/topic/data mismatch, missing transaction, non-finalized receipt, provider-quorum failure and callback timeout.

## Trust boundary

Read RPC is MPC-backed foreign-chain observation using a configured RPC provider set and MPC operator threshold. It is not a light-client proof verified by the EVM destination. AfterState's NEAR contract and MPC network are trusted components. The EVM EIP-712 verifier still only verifies the derived authorized signer; it does not independently parse or verify the NEAR ForeignTx signature.
