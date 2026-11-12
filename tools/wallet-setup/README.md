# AfterState local wallet setup

This utility generates two independent identities **on your own machine**:

- An EVM wallet intended for Base Sepolia deployment transactions (chain ID 84532).
- A NEAR Ed25519 keypair intended for NEAR testnet.

It does not contact a wallet service, broadcast a transaction, register a NEAR account, or fund either identity. It does not generate the separate NEAR Chain Signatures-derived EVM signer.

## Security model

- Key generation happens locally using Node.js cryptographic randomness and the official `ethers` wallet API plus NEAR's `near-seed-phrase` package.
- All secret material is encrypted with AES-256-GCM. The encryption key is derived from a password using scrypt (N=32768, r=8, p=1).
- The password is entered twice with terminal echo disabled. Use a unique password of at least 16 characters.
- Files are created with restrictive permissions (directory 0700, files 0600) where supported.
- The utility refuses to overwrite existing output.
- Never commit, upload, or share `wallet-output/afterstate-wallets.enc.json`. Keep a separate offline backup of the password.
- The public JSON contains only public identifiers and may be shared.

This is a small project utility, not a substitute for a hardware wallet or a professionally audited key-management system. For meaningful funds, use a hardware wallet and independently review the source before running it.

## Run

Requires Node.js 20 or newer. On Android, Termux can provide a local terminal and Node.js runtime:

```sh
pkg update
pkg install nodejs-lts git
 git clone https://github.com/Godwin-web3/AfterState.git
cd AfterState/tools/wallet-setup
npm install
npm run generate
```

On a computer, clone the repository and run the same three project commands from `tools/wallet-setup`. Do not run this in a hosted online IDE or cloud shell; generate the keys on a device you control.

The command creates:

- `tools/wallet-setup/wallet-output/afterstate-wallets.enc.json` — encrypted secret backup.
- `tools/wallet-setup/wallet-output/afterstate-wallets.public.json` — public identifiers.

The output directory is gitignored. Do not move the encrypted file into a public repository.

## Next steps

1. Back up the encrypted file and password separately, offline.
2. Share only the EVM address and NEAR public key with the project operator.
3. Fund the EVM address with Base Sepolia test ETH from a faucet.
4. Register/fund the NEAR testnet account through an official NEAR testnet onboarding flow. A generated public key alone is not a registered named account.
5. Do not use these testnet keys for mainnet funds.
