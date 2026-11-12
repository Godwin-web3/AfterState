# Frontend execution flow

The frontend's reference path is:

1. Connect an EVM wallet and a NEAR testnet wallet.
2. Register or import the destination agreement. Its 32-byte agreement ID, recipient, asset, amount bounds, and deadline must match the immutable configuration passed to the NEAR adapter at initialization.
3. Approve the Base Sepolia demo token and call `AfterStateSourceEmitter.pay`.
4. Read the exact receipt log. The frontend preserves the transaction hash, block hash/number, transaction index, log index, emitter, topics, and data in the adapter's `BaseEvmLogCandidate` JSON shape.
5. Read the adapter's public `config()` and check that the connected NEAR account is the configured `authorized_submitter`.
6. Ask the NEAR wallet to call `verify_base_log(candidate)`. The adapter requests MPC ForeignTx verification with Finalized finality and an expected payload hash, then requests a separate Secp256k1 Chain Signatures result.
7. Poll NEAR transaction status until the final callback receipt returns the 65-byte EVM signature.
8. Submit the exact source fact and signature to `AfterStateRegistry.consumeSourceFact` on Ethereum Sepolia.
9. Only after the registry accepts the fact can the settlement adapter be called.

## Wallet and network behavior

- NEAR wallet integration uses `@hot-labs/near-connect` with the testnet wallet feature filter.
- NEAR RPC reads use the public testnet JSON-RPC endpoint. No NEAR private key is stored by the app.
- EVM transactions are signed by the connected EIP-1193 wallet. No EVM private key is stored by the app.
- The adapter account must already be deployed and initialized. Its `authorized_submitter` must be the connected NEAR account.
- The NEAR transaction must return a transaction hash to the app. Wallets that redirect for approval may require returning to the app and resubmitting the verification action if they do not return an outcome to the original page.
- The NEAR account must have enough testnet NEAR to pay for the transaction and MPC promise chain.

## Important fixed-agreement constraint

The current NEAR adapter is initialized with one immutable agreement. The frontend therefore requires an explicit 32-byte agreement ID and checks existing registry terms instead of silently creating a random ID that the adapter cannot verify. Deploy scripts, adapter init arguments, and frontend agreement details must use the same ID and terms.

## Verification status

This frontend path is implemented but is not yet a live end-to-end acceptance test. It must not be described as live MPC-verified until the adapter is deployed, a NEAR wallet call succeeds, the MPC callback returns the signature, and Ethereum Sepolia accepts the fact. The current testnet prototype remains unaudited and is not production-ready.
