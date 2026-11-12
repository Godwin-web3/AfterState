# Live NEAR MPC testnet snapshot

Observed: 2026-11-12 14:42 UTC  
Network: NEAR testnet  
MPC account: `v1.signer-prod.testnet`  
Deployed MPC version: `3.18.0`  
Keyset epoch: `27`  
Threshold: `11 of 17`

Evidence: [read-only state workflow run 43239982726](https://github.com/Godwin-web3/AfterState/actions/runs/43239982726)  
Artifact: `near-mpc-testnet-state`  
Artifact SHA-256: `60b8a6d6f9bb67682b9a84b1be4a4f17550b0fe22116668bb6c0142bb265f113`

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

**Initial integration selection:** ForeignTx domain **3** and Sign domain **0**. They are distinct and have the required purposes. Domain 0 has a Secp256k1 root key suitable for EVM address derivation. This selection is a snapshot, not a permanent guarantee: re-read the live state immediately before deployment and abort if either domain's purpose or key type changed.

## Gas requirements observed

The live MPC `config()` reports:

- `sign_call_gas_attachment_requirement_tera_gas`: **15 Tgas**
- `return_signature_and_clean_state_on_success_call_tera_gas`: **7 Tgas**

The pinned MPC contract implementation applies the sign-call gas attachment requirement to `verify_foreign_transaction` as well as `sign`. The adapter now rejects either outbound MPC call configured below 15 Tgas. Callback gas remains a separate adapter execution budget and must be measured on testnet before final deployment.

## What this proves — and does not prove

This snapshot proves that the deployed NEAR testnet MPC contract currently exposes Base as available, has a Base RPC-provider quorum of two, and has separate live ForeignTx and Sign domains with Secp256k1 key material.

It does **not** prove that a particular Base Sepolia log has passed MPC verification, that an AfterState adapter account has been deployed, that an EVM signer address has been derived for that account, or that destination settlement has executed. Those remain end-to-end acceptance gates.
