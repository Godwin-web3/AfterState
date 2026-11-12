# AfterState

**Verified conditions. Enforced outcomes.**

AfterState is an execution-assurance protocol for cross-chain workflows. It binds a specific source-chain event to a destination EIP-712 authorization, then leaves destination execution to an agreement-bound registry and settlement adapter.

**Reference path:** Base Sepolia (source) → NEAR MPC (ForeignTx verification + Chain Signatures) → Ethereum Sepolia (destination).

## Current status

- Solidity contracts and Foundry tests are implemented.
- The NEAR adapter and MPC 3.18.0 ForeignTx request path are implemented and WASM-target checked.
- A live read-only query confirmed Base support, ForeignTx domain 3, Secp256k1 Sign domain 0, and derived the adapter's MPC signer address.
- The NEAR adapter is not yet deployed. No live ForeignTx verification or destination settlement has been executed.
- Experimental prototype; unaudited; not production-ready.

## Frontend

Live interface: [afterstate-app.vercel.app](https://afterstate-app.vercel.app).

The Vite/React interface is in the frontend directory. It supports EIP-1193 EVM wallet connection, NEAR testnet wallet connection through NEAR Connect, destination agreement registration/import, Base Sepolia token approval and source payment, NEAR adapter submission, MPC outcome polling, destination fact consumption, and destination settlement. Contract addresses and activity are stored locally in the browser; signing keys remain in user wallets.

The frontend integration is implemented but has not yet completed a live MPC-to-EVM acceptance test. The adapter must be deployed and initialized, its configured submitter must connect through a NEAR wallet, and both EVM networks must have the required contracts deployed. The adapter is initialized for one immutable agreement, so the frontend requires the exact agreement ID and terms configured at deployment. v0.1 assumes the same ERC-20 address and units on both EVM testnets.

See [the frontend execution flow](./docs/FRONTEND_FLOW.md) for the exact transaction sequence and remaining live-test requirements.

## Security boundaries

NEAR MPC authenticates the foreign transaction observation under its provider and threshold assumptions; it is not a trustless EVM light client. Cross-chain execution is not atomic. Source payments are irreversible and are not automatically refunded if destination execution fails. The EVM verifier authenticates a configured signer; it does not independently prove source-chain consensus or prove that a signature came from NEAR MPC.

See [the protocol specification](./spec/PROTOCOL.md), [NEAR deployment checklist](./near/DEPLOYMENT_CHECKLIST.md), and [security review](./docs/SECURITY_REVIEW.md).
