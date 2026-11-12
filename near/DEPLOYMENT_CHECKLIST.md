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

The ForeignTx domain must have ForeignTx purpose; the signing domain must have Sign purpose and expose Secp256k1. Their IDs must differ. The adapter enforces distinct IDs, and MPC independently checks each request's purpose. The live testnet currently requires at least 15 Tgas on both ForeignTx and Sign requests; the adapter constructor rejects lower values. The ForeignTx callback must also have enough gas to schedule the follow-on Sign call and its callback: the constructor requires `foreign_callback_gas_tgas >= sign_call_gas_tgas + signature_callback_gas_tgas + 10`. The supplied example uses 60 Tgas for that callback, 15 Tgas for Sign, and 30 Tgas for the final callback.

If Base is absent from the live available-chain view, stop. Do not silently switch chains or weaken finality.

Before deploying the EVM verifier, choose the NEAR account ID that will host the adapter. Create/fund that testnet account if needed and retain its full-access key locally. The derived EVM key depends on the account ID, fixed path `afterstate-v1`, and Sign domain; it can be derived before the adapter contract is deployed.

Run [NEAR MPC Testnet State](https://github.com/Godwin-web3/AfterState/actions/workflows/near-mpc-state.yml) manually with `adapter_account_id` set to that intended account. Leave domain inputs blank so the workflow discovers the live domains. Copy `derived_evm_address` from the report into `AFTERSTATE_MPC_SIGNER_ADDRESS` for the destination deployment.

For the intended account `b6739911d06797c8d55539944a5226d132c2001e34d7bf38993e735954b162d3`, the account-specific query already derived `AFTERSTATE_MPC_SIGNER_ADDRESS=0x5073ec8a42ad4dad872fc593de1009ce728a088f` on 2026-11-12. Evidence: [workflow run 43249688456](https://github.com/Godwin-web3/AfterState/actions/runs/43249688456). This is the MPC signing address, not the EVM deployer wallet address. Re-query before deploying.

## 3. Deploy the EVM source and destination sides

The demo must use two different EVM chains or it is not a cross-chain execution demonstration:

**Source: Base Sepolia (84532)**

- Deploy an allowlisted conventional ERC-20 test token.
- Deploy AfterStateSourceEmitter.
- Record the token and emitter addresses, source chain ID, and deployed bytecode hashes.

**Destination: Ethereum Sepolia (11155111)**

- Deploy the destination ERC-20 at the **same address and with the same decimals** as the source token. For this narrow v0.1 demo, use the same deployer and matching account nonce for token creation on both networks, then verify the resulting token addresses are identical before proceeding. If they differ, stop; do not create an agreement.
- Deploy AfterStateEIP712Verifier, AfterStateRegistry, and AfterStateERC20Settlement.
- Initialize verifier signer version 1 with the EVM address derived from the live Sign domain for the adapter account and fixed path.
- Point the registry at the verifier.
- Point the settlement adapter at the registry and configure it as the registry's sole settlement adapter.
- Create an agreement with source chain 84532, destination chain 11155111 (set by the destination registry), Base Sepolia source emitter, SourcePayment event schema, matching token address, recipient, amount bounds, and deadline.
- Prefund the destination settlement contract with the destination-chain token.

This prototype requires matching source/destination asset addresses and amount units. It does not bridge assets or convert decimals. The same token address on both networks is a demo constraint, not an asset-bridging mechanism.

Deployment scripts are provided in `script/DeployBaseSepolia.s.sol` and `script/DeployEthereumSepolia.s.sol`. Start from `.env.example`, copy it to an ignored local `.env`, and fill in real testnet values. The deployer key must be the same on both chains, and `TOKEN_DEPLOYER_NONCE` must match the deployer's current nonce on both networks.

```sh
set -a
source .env
set +a

# First deploy the source token and emitter. Save the returned token address, emitter address, and token code hash.
forge script script/DeployBaseSepolia.s.sol:DeployBaseSepolia \
  --rpc-url "$BASE_SEPOLIA_RPC_URL" --broadcast

# Fill EXPECTED_ASSET_ADDRESS, EXPECTED_ASSET_CODE_HASH, and BASE_SEPOLIA_SOURCE_EMITTER with the first deployment's address/hash outputs.
# Fill AFTERSTATE_MPC_SIGNER_ADDRESS with the derived address from the live NEAR query.
forge script script/DeployEthereumSepolia.s.sol:DeployEthereumSepolia \
  --rpc-url "$ETHEREUM_SEPOLIA_RPC_URL" --broadcast
```

The destination script checks the expected token address and bytecode hash, signer, chain ID, amount bounds, deadline width, and escrow prefunding before broadcasting. Keep `.env`, broadcast output, private keys, and seed phrases out of Git.

## 4. Deploy the NEAR adapter

Deploy the release WASM to a dedicated NEAR testnet account. Initialize it with the live MPC account, dedicated authorized submitter, distinct live ForeignTx and Sign domain IDs, measured gas values, source emitter and agreement fields, destination chain/verifier/registry addresses, finality-policy identifier, and EVM signer version.

The local deployment helper builds the release WASM, validates and serializes the fixed-array init arguments, then uses near-cli-rs with the local OS keychain:

```sh
# Fill the NEAR adapter values in your ignored local .env first.
bash near/scripts/deploy-adapter.sh
```

The helper uses `ADAPTER_ACCOUNT_ID` as the contract account and `AUTHORIZED_SUBMITTER` as the only account permitted to call `verify_base_log`. Set `DERIVED_FOR_ADAPTER_ACCOUNT_ID` to the account ID used in the live signer-derivation workflow; the helper aborts if it differs from `ADAPTER_ACCOUNT_ID`. The account must already exist on NEAR testnet and have a full-access key available in your local near-cli-rs keychain. The script does not upload credentials to GitHub.

Never commit a seed phrase, private key, or full-access key to the repository or CI.

## 5. End-to-end acceptance test

Use a small amount and fresh nonce:

1. Approve the source emitter and call pay(agreementId, agreementVersion, asset, recipient, amount, nonce).
2. Obtain the exact Base receipt log, preserving transaction hash, block hash/number, transaction index, log index, emitter, topics, and data.
3. Submit the candidate log to verify_base_log from the authorized NEAR submitter with enough prepaid gas for the complete promise chain (start with 100 Tgas, then measure actual use).
4. Confirm MPC ForeignTx verification returns only after the expected payload hash matches.
5. Confirm the follow-on sign request uses the separate Sign domain and fixed afterstate-v1 path.
6. Submit the returned EVM signature and exact fact to consumeSourceFact on the Ethereum Sepolia destination registry.
7. Call settleERC20(agreementId, nonce) on Ethereum Sepolia.
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