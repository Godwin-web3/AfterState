#!/usr/bin/env bash
set -euo pipefail

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

: "${ADAPTER_ACCOUNT_ID:?set ADAPTER_ACCOUNT_ID to a NEAR testnet account you control}"
: "${AUTHORIZED_SUBMITTER:?set AUTHORIZED_SUBMITTER to the account allowed to submit source logs}"

cargo build --release --target wasm32-unknown-unknown --manifest-path near/adapter/Cargo.toml
WASM="near/adapter/target/wasm32-unknown-unknown/release/afterstate_near_adapter.wasm"
test -s "$WASM"

INIT_ARGS="$(python3 near/scripts/build_adapter_init_args.py)"
DEPLOY_GAS="${NEAR_DEPLOYMENT_GAS_TGAS:-100}"

near contract deploy "$ADAPTER_ACCOUNT_ID" \
  use-file "$WASM" \
  with-init-call new \
  json-args "$INIT_ARGS" \
  prepaid-gas "$DEPLOY_GAS Tgas" \
  attached-deposit '0 NEAR' \
  network-config testnet \
  sign-with-keychain \
  send
