# NEAR ForeignTx request contract

Status: implemented request builder and adapter call path pinned to NEAR MPC 3.19.0, release commit 86e2cc4d124397ec81f436b11f48bf9b6dfd062e. Not yet deployed against live testnet state.

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

Pinned sources:
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/contract/src/api/foreign_chain/requests.rs
- https://github.com/near/mpc/blob/86e2cc4d124397ec81f436b11f48bf9b6dfd062e/crates/near-mpc-contract-interface/src/types/foreign_chain.rs

## Request shape

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

The domain ID is read from deployed contract state and must have ForeignTx purpose. Do not assume a domain number from a tutorial. The adapter uses the exact Base Sepolia request and always supplies expected_payload_hash.

## Why expected_payload_hash is mandatory

MPC constructs ForeignTxSignPayload::V1 from the exact request and extracted values, Borsh-serializes it, then SHA-256 hashes the bytes. Nodes sign that hash. When expected_payload_hash is supplied, the MPC contract compares the response payload hash with the expected hash and rejects a mismatch. AfterState must not omit this field.

The repo's Rust fixture crate imports the exact MPC 3.19.0 interface commit. It asserts the exact Borsh bytes and deterministic Base payload hash:
9bc1f744d04ebee4e32a814c3742b5b7ded2cf0e1da23fa1689675b695d1616a

## Application event

AfterStateSourceEmitter emits:
SourcePayment(bytes32,uint32,address,address,address,uint256,uint64,bytes32)

Indexed topics: agreementId, agreementVersion, asset. Data words: payer, recipient, amount, sourceTimestamp, nonce. The NEAR decoder checks topic0, topic count, data length, canonical address padding, integer bounds and nonzero fields. The MPC request then authenticates the exact block hash and log.

## Separate Chain Signatures step

After the ForeignTx callback succeeds, the adapter computes the Solidity-compatible AfterState EIP-712 digest and calls sign with:
- Sign-purpose domain ID (distinct from ForeignTx domain);
- Payload::Ecdsa digest;
- fixed derivation path afterstate-v1;
- configured signer_version in the EIP-712 fact.

The final callback converts only canonical secp256k1 signatures into EVM r||s||v bytes. The EIP-712 digest is cross-checked against the Solidity verifier by a shared deterministic test vector.

## Live deployment gates

- Query deployed contract state for ForeignTx and Sign domains, purposes, schemes, root keys, participants, threshold and available chains.
- Confirm Base Sepolia appears in the live available-chain set; example node config is not enough.
- Query provider quorum/allowlist and confirm provider independence.
- Derive the EVM signer address for the deployed adapter account and fixed path afterstate-v1; configure EVM signerAtVersion with that address.
- Use live gas requirements and attached deposits. The pinned interface requires 1 yoctoNEAR for each call, but gas must be set to the deployed contract's current requirement.
- Test wrong payload hash, wrong log index, malformed/removed log, wrong emitter/topic/data, non-finalized receipt, provider-quorum failure and callback failures.

## Trust boundary

Read RPC is MPC-backed foreign-chain observation, not a light-client proof verified by the EVM destination. The EVM destination trusts the configured derived signer. The EVM verifier does not independently parse or verify the ForeignTx response.
