# AfterState testnet deployment and acceptance checklist

Status: procedure only; no live deployment is claimed. Testnet acceptance requires a funded NEAR account, live MPC state, and EVM testnet transactions.

## 1. Pin and build

- Build from a tagged commit and record the Git SHA, Rust toolchain, near-sdk version, and pinned MPC interface commit.
- Build WASM:

```sh
cargo build --release --target wasm32-unknown-unknown --manifest-path near/adapter/Cargo.toml
```

- Record the WASM SHA-256 and run both CI workflows on the exact commit.

## 2. Inspect live MPC testnet state

A read-only query on 2026-11-12 confirmed live contract version 3.18.0, Base availability, three whitelisted Base RPC providers with quorum 2, ForeignTx domain 3, and Secp256k1 Sign domain 0. See [MPC_TESTNET_SNAPSHOT.md](MPC_TESTNET_SNAPSHOT.md) and [workflow run 43239982726](https://github.com/Godwin-web3/AfterState/actions/runs/43239982726). Treat those IDs and values as a dated snapshot, not constants.

Do not assume the contract account, domain IDs, Base availability, provider quorum, or gas values from tutorials or example configuration.

Against the current official NEAR testnet MPC account, make these read-only view calls:

- version() — record deployed contract version.
- state() — inspect the domain registry, domain purposes, and protocols.
- get_available_foreign_chains() — confirm Base is available now.
- allowed_foreign_chain_providers() — record Base providers and quorum.
- public_key({domain_id: SIGN_DOMAIN}) — confirm the Sign domain exposes secp256k1.
- derived_public_key({path: "afterstate-v1", predecessor: ADAPTER_ACCOUNT, domain_id: SIGN_DOMAIN}) — derive the adapter's exact application key.

The ForeignTx domain must have ForeignTx purpose; the signing domain must have Sign purpose and expose Secp256k1. Their IDs must differ. The adapter enforces distinct IDs, and MPC independently checks each request's purpose. The live testnet currently requires at least 15 Tgas on both ForeignTx and Sign requests; the adapter constructor rejects lower values.

If Base is absent from the live available-chain view, stop. Do not silently switch chains or weaken finality.

## 3. Deploy Base Sepolia contracts

Deploy an allowlisted conventional ERC-20 test token, AfterStateSourceEmitter, AfterStateEIP712Verifier, AfterStateRegistry, and AfterStateERC20Settlement.

- Initialize verifier signer version 1 with the EVM address derived from the live Sign domain for the adapter account and fixed path.
- Point the registry at the verifier.
- Point the settlement adapter at the registry and configure it as the registry's sole settlement adapter.
- Create an agreement matching source chain 84532, source emitter, event schema, asset, recipient, amount bounds, and deadline.
- Prefund the destination settlement contract.

This prototype requires matching source/destination asset addresses and amount units. It does not bridge assets or convert decimals.

## 4. Deploy the NEAR adapter

Deploy the release WASM to a dedicated NEAR testnet account. Initialize it with the live MPC account, dedicated authorized submitter, distinct live ForeignTx and Sign domain IDs, measured gas values, source emitter and agreement fields, destination chain/verifier/registry addresses, finality-policy identifier, and EVM signer version.

Never commit a seed phrase, private key, or full-access key to the repository or CI.

## 5. End-to-end acceptance test

Use a small amount and fresh nonce:

1. Approve the source emitter and call pay(agreementId, agreementVersion, asset, recipient, amount, nonce).
2. Obtain the exact Base receipt log, preserving transaction hash, block hash/number, transaction index, log index, emitter, topics, and data.
3. Submit the candidate log to verify_base_log from the authorized NEAR submitter.
4. Confirm MPC ForeignTx verification returns only after the expected payload hash matches.
5. Confirm the follow-on sign request uses the separate Sign domain and fixed afterstate-v1 path.
6. Submit the returned EVM signature and exact fact to consumeSourceFact on the destination registry.
7. Call settleERC20(agreementId, nonce).
8. Verify the recipient's exact balance delta and registry consumed/claimed state.

Save transaction hashes, block identifiers, deployed addresses, domain IDs and purposes, provider quorum, gas used, and Git SHA as demo evidence.

## 6. Mandatory negative tests

- Wrong transaction hash, block hash/number, log index, emitter, topic0, topic count, or event data.
- Removed or non-finalized source log.
- Wrong agreement/version, asset, recipient, amount, deadline, or zero nonce.
- Wrong-purpose MPC domain, identical ForeignTx and Sign domain IDs, or changed expected payload hash.
- MPC timeout, provider quorum failure, signer outage, or failed callback.
- Malformed, non-secp256k1, invalid-recovery-ID, zero/out-of-range r, zero/high-s signature.
- Fact signed for another destination chain, verifier, or registry.
- Same source transaction/log with a different nonce.
- Repeated settlement, unconfigured adapter, insufficient prefunding, and fee-on-transfer/rebasing token behavior.

## 7. Release decision

Compilation and a testnet happy path do not establish production readiness. Production still requires independent security review, live MPC/provider trust analysis, asset mapping, recoverable escrow/refund semantics, monitoring, rate limits, key/domain rotation, and incident response.