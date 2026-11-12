# Live NEAR MPC testnet snapshot

Observed: 2026-11-12 15:10 UTC  
Network: NEAR testnet  
MPC account: `v1.signer-prod.testnet`  
Deployed MPC version: `3.18.0`  
Pinned interface source: [NEAR MPC 3.18.0](https://github.com/near/mpc/tree/e691bdb98d68dcf74653da58d043453a6318765f)  
Keyset epoch: `27`  
Threshold: `11 of 17`

Evidence: [read-only state workflow run 43244784087](https://github.com/Godwin-web3/AfterState/actions/runs/43244784087)  
Artifact: `near-mpc-testnet-state`  
Artifact SHA-256: `c8e0616f15d947dbeb0f09aa6e6be8ee4396317b62d0ed5e1714f3f3c9f4c691`

## Foreign-chain support

Base is in the contract's live `get_available_foreign_chains()` result.

The on-chain provider whitelist for Base contains:

- Alchemy — `https://base-sepolia.g.alchemy.com/v2/`
- QuickNode — `https://{}.base-sepolia.quiknode.pro`
- Tatum — `https://base-sepolia.gateway.tatum.io`

Configured provider quorum: **2**. This is the RPC-provider response quorum, not the MPC signing threshold. Provider credentials are not included in the on-chain configuration.

## Live domain registry

| Domain ID | Protocol | Purpose | Key type in active keyset |
|---:|---|---|---|
| 0 | CaitSith | Sign | Secp256k1 |
| 1 | Frost | Sign | Ed25519 |
| 2 | ConfidentialKeyDerivation | CKD | BLS12-381 |
| 3 | CaitSith | ForeignTx | Secp256k1 |
| 4 | CaitSith | Sign | Secp256k1 |
| 5 | Frost | Sign | Ed25519 |
| 6 | ConfidentialKeyDerivation | CKD | BLS12-381 |
| 7 | CaitSith | ForeignTx | Secp256k1 |

**Initial integration selection:** ForeignTx domain **3** and Sign domain **0**. They are distinct and have the required purposes.

- Domain 3 ForeignTx root public key: `secp256k1:2KGCoy2pZt7n85QfJnQzCT1eHySuNpquUfNc8ySpRbr15F5kqU7agJYjfo5RkrzNd4tZimDFv2wAZri7RRZ32qj1`
- Domain 0 Sign root public key: `secp256k1:4NfTiv3UsGahebgTaHyD9vF8KYKMBnfd6kh94mK6xv8fGBiJB8TBtFMP5WWXz6B89Ac1fbpzPwAvoyQebemHFwx3`

The workflow now discovers the first live ForeignTx domain and the first Sign domain with a Secp256k1 key, then queries both public keys. Domain 0 is the current EVM Sign selection. This selection is a snapshot, not a permanent guarantee: re-read live state immediately before deployment and abort if either domain's purpose or key type changed.

## Gas requirements observed

The live MPC `config()` reports:

- `sign_call_gas_attachment_requirement_tera_gas`: **15 Tgas**
- `return_signature_and_clean_state_on_success_call_tera_gas`: **7 Tgas**

The pinned MPC contract implementation applies the sign-call gas attachment requirement to `verify_foreign_transaction` as well as `sign`. The adapter now rejects either outbound MPC call configured below 15 Tgas. Callback gas remains a separate adapter execution budget and must be measured on testnet before final deployment.


## Account-specific signer derivation

A second read-only query on **2026-11-12 15:38 UTC** ran against the exact intended NEAR adapter account:

- Adapter account: `b6739911d06797c8d55539944a5226d132c2001e34d7bf38993e735954b162d3`
- Derived EVM public key: `secp256k1:4Cj6zxNFR23aeWAv8ZcYnRvCN5y6njXZRHQzXx18QcsrV4j3Nn7b4VE6NDD9X5Z1D7FFSWMySDGi72zfHN2Q6cvT`
- **Derived EVM signer address:** `0x5073ec8a42ad4dad872fc593de1009ce728a088f`
- Evidence: [account-specific live MPC query run 43249688456](https://github.com/Godwin-web3/AfterState/actions/runs/43249688456)

This is the signer address that must be configured in the Ethereum Sepolia `AfterStateEIP712Verifier` for signer version 1 and passed as `AFTERSTATE_MPC_SIGNER_ADDRESS` to the destination deployment script. Re-run the query immediately before deployment; do not assume the keyset or domain registry is unchanged.

## What this proves — and does not prove

This snapshot proves that the deployed NEAR testnet MPC contract currently exposes Base as available, has a Base RPC-provider quorum of two, and has separate live ForeignTx and Sign domains with Secp256k1 key material.

It does **not** prove that a particular Base Sepolia log has passed MPC verification, that an AfterState adapter account has been deployed, or that destination settlement has executed. Those remain end-to-end acceptance gates.
