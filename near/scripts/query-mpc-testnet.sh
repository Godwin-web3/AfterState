#!/usr/bin/env bash
set -euo pipefail

RPC_URL="${NEAR_RPC_URL:-https://rpc.testnet.near.org}"
MPC_ACCOUNT_ID="${MPC_ACCOUNT_ID:-v1.signer-prod.testnet}"
ADAPTER_ACCOUNT_ID="${ADAPTER_ACCOUNT_ID:-}"
SIGN_DOMAIN_ID="${SIGN_DOMAIN_ID:-}"
OUTPUT_DIR="${OUTPUT_DIR:-near-mpc-state}"
mkdir -p "$OUTPUT_DIR"

view_call() {
  local method="$1"
  local args_json="${2:-{}}"
  local args_b64
  args_b64="$(printf '%s' "$args_json" | base64 -w0)"
  local payload
  payload="$(jq -n \
    --arg account "$MPC_ACCOUNT_ID" \
    --arg method "$method" \
    --arg args "$args_b64" \
    '{jsonrpc:"2.0",id:"afterstate-state-check",method:"query",params:{request_type:"call_function",finality:"final",account_id:$account,method_name:$method,args_base64:$args}}')"
  local response
  response="$(curl --fail --silent --show-error \
    -H 'Content-Type: application/json' \
    -d "$payload" "$RPC_URL")"
  if printf '%s' "$response" | jq -e '.error' >/dev/null; then
    printf '%s\n' "$response" >&2
    return 1
  fi
  printf '%s' "$response" | jq -er '[.result.result[]] | implode'
}

view_call version '{}' > "$OUTPUT_DIR/version.json"
view_call config '{}' > "$OUTPUT_DIR/config.json"
view_call state '{}' > "$OUTPUT_DIR/state.json"
view_call get_available_foreign_chains '{}' > "$OUTPUT_DIR/available_foreign_chains.json"

if [[ -n "$SIGN_DOMAIN_ID" ]]; then
  public_key_args="$(jq -n --argjson domain_id "$SIGN_DOMAIN_ID" '{domain_id:$domain_id}')"
  view_call public_key "$public_key_args" > "$OUTPUT_DIR/sign_domain_public_key.json"

  if [[ -n "$ADAPTER_ACCOUNT_ID" ]]; then
    derived_args="$(jq -n \
      --arg path 'afterstate-v1' \
      --arg predecessor "$ADAPTER_ACCOUNT_ID" \
      --argjson domain_id "$SIGN_DOMAIN_ID" \
      '{path:$path,predecessor:$predecessor,domain_id:$domain_id}')"
    view_call derived_public_key "$derived_args" > "$OUTPUT_DIR/derived_evm_public_key.json"
  fi
fi

jq -n \
  --arg mpc_account "$MPC_ACCOUNT_ID" \
  --arg adapter_account "$ADAPTER_ACCOUNT_ID" \
  --arg sign_domain_id "$SIGN_DOMAIN_ID" \
  --slurpfile version "$OUTPUT_DIR/version.json" \
  --slurpfile config "$OUTPUT_DIR/config.json" \
  --slurpfile state "$OUTPUT_DIR/state.json" \
  --slurpfile chains "$OUTPUT_DIR/available_foreign_chains.json" \
  '{mpc_account:$mpc_account,adapter_account:$adapter_account,sign_domain_id:$sign_domain_id,version:$version[0],config:$config[0],state:$state[0],available_foreign_chains:$chains[0]}' \
  > "$OUTPUT_DIR/report.json"

if [[ -f "$OUTPUT_DIR/sign_domain_public_key.json" ]]; then
  jq --slurpfile key "$OUTPUT_DIR/sign_domain_public_key.json" '. + {sign_domain_public_key:$key[0]}' \
    "$OUTPUT_DIR/report.json" > "$OUTPUT_DIR/report.tmp"
  mv "$OUTPUT_DIR/report.tmp" "$OUTPUT_DIR/report.json"
fi

if [[ -f "$OUTPUT_DIR/derived_evm_public_key.json" ]]; then
  jq --slurpfile key "$OUTPUT_DIR/derived_evm_public_key.json" '. + {derived_evm_public_key:$key[0]}' \
    "$OUTPUT_DIR/report.json" > "$OUTPUT_DIR/report.tmp"
  mv "$OUTPUT_DIR/report.tmp" "$OUTPUT_DIR/report.json"
fi

cat "$OUTPUT_DIR/report.json"
