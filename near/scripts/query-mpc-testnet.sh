#!/usr/bin/env bash
set -euo pipefail

RPC_URL="${NEAR_RPC_URL:-https://rpc.testnet.near.org}"
MPC_ACCOUNT_ID="${MPC_ACCOUNT_ID:-v1.signer-prod.testnet}"
ADAPTER_ACCOUNT_ID="${ADAPTER_ACCOUNT_ID:-}"
FOREIGN_TX_DOMAIN_ID="${FOREIGN_TX_DOMAIN_ID:-}"
SIGN_DOMAIN_ID="${SIGN_DOMAIN_ID:-}"
OUTPUT_DIR="${OUTPUT_DIR:-near-mpc-state}"
mkdir -p "$OUTPUT_DIR"

view_call() {
  local method="$1"
  local args_json="${2-}"
  local args_b64
  if [[ -z "$args_json" ]]; then args_json='{}'; fi
  args_json="$(printf '%s' "$args_json" | jq -c .)"
  args_b64="$(printf '%s' "$args_json" | base64 -w0)"
  local payload
  payload="$(jq -n \
    --arg account "$MPC_ACCOUNT_ID" \
    --arg method "$method" \
    --arg args "$args_b64" \
    '{jsonrpc:"2.0",id:"afterstate-state-check",method:"query",params:{request_type:"call_function",finality:"final",account_id:$account,method_name:$method,args_base64:$args}}')"
  local response
  local attempt
  for attempt in 1 2 3; do
    response="$(curl --fail --silent --show-error \
      --connect-timeout 10 --max-time 45 --retry 2 --retry-all-errors --retry-delay 2 \
      -H 'Content-Type: application/json' \
      -d "$payload" "$RPC_URL")"
    if printf '%s' "$response" | jq -e '.error' >/dev/null; then
      printf 'NEAR view %s returned RPC error on attempt %s:\n%s\n' "$method" "$attempt" "$response" >&2
      sleep 2
      continue
    fi
    if printf '%s' "$response" | jq -e '.result.result | type == "array"' >/dev/null; then
      printf '%s' "$response" | jq -er '[.result.result[]] | implode'
      return 0
    fi
    printf 'NEAR view %s returned an unexpected response on attempt %s:\n%s\n' "$method" "$attempt" "$response" >&2
    sleep 2
  done
  printf 'NEAR view %s failed after retries\n' "$method" >&2
  return 1
}

view_call version '{}' > "$OUTPUT_DIR/version.json"
view_call config '{}' > "$OUTPUT_DIR/config.json"
view_call state '{}' > "$OUTPUT_DIR/state.json"
view_call get_available_foreign_chains '{}' > "$OUTPUT_DIR/available_foreign_chains.json"
view_call get_foreign_chains_configs '{}' > "$OUTPUT_DIR/foreign_chains_configs.json"
view_call allowed_foreign_chain_providers '{}' > "$OUTPUT_DIR/allowed_foreign_chain_providers.json"

# Normalize the state response and discover current domains unless overridden.
jq -c 'if type == "string" then fromjson else . end' \
  "$OUTPUT_DIR/state.json" > "$OUTPUT_DIR/state.normalized.json"
if [[ -z "$FOREIGN_TX_DOMAIN_ID" ]]; then
  FOREIGN_TX_DOMAIN_ID="$(jq -er '
    (if has("Running") then .Running else .state.Running end) as $running |
    [$running.domains.domains[] | select(.purpose == "ForeignTx") | .id] | first
  ' "$OUTPUT_DIR/state.normalized.json")"
fi
if [[ -z "$SIGN_DOMAIN_ID" ]]; then
  SIGN_DOMAIN_ID="$(jq -er '
    (if has("Running") then .Running else .state.Running end) as $running |
    [$running.domains.domains[] | select(.purpose == "Sign" and .protocol == "CaitSith") | .id] | first
  ' "$OUTPUT_DIR/state.normalized.json")"
fi

if [[ -n "$FOREIGN_TX_DOMAIN_ID" ]]; then
  foreign_args="$(jq -n --argjson domain_id "$FOREIGN_TX_DOMAIN_ID" '{domain_id:$domain_id}')"
  view_call public_key "$foreign_args" > "$OUTPUT_DIR/foreign_tx_domain_public_key.json"
fi

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
    cargo run --quiet --manifest-path near/foreign-tx-fixtures/Cargo.toml --bin derive-evm-address -- "$OUTPUT_DIR/derived_evm_public_key.json" > "$OUTPUT_DIR/derived_evm_address.txt"
  fi
fi

jq -n \
  --arg mpc_account "$MPC_ACCOUNT_ID" \
  --arg adapter_account "$ADAPTER_ACCOUNT_ID" \
  --arg foreign_tx_domain_id "$FOREIGN_TX_DOMAIN_ID" \
  --arg sign_domain_id "$SIGN_DOMAIN_ID" \
  --slurpfile version "$OUTPUT_DIR/version.json" \
  --slurpfile config "$OUTPUT_DIR/config.json" \
  --slurpfile state "$OUTPUT_DIR/state.json" \
  --slurpfile chains "$OUTPUT_DIR/available_foreign_chains.json" \
  --slurpfile foreign_configs "$OUTPUT_DIR/foreign_chains_configs.json" \
  --slurpfile allowed_providers "$OUTPUT_DIR/allowed_foreign_chain_providers.json" \
  '{mpc_account:$mpc_account,adapter_account:$adapter_account,foreign_tx_domain_id:$foreign_tx_domain_id,sign_domain_id:$sign_domain_id,version:$version[0],config:$config[0],state:$state[0],available_foreign_chains:$chains[0],foreign_chains_configs:$foreign_configs[0],allowed_foreign_chain_providers:$allowed_providers[0]}' \
  > "$OUTPUT_DIR/report.json"

if [[ -f "$OUTPUT_DIR/foreign_tx_domain_public_key.json" ]]; then
  jq --slurpfile key "$OUTPUT_DIR/foreign_tx_domain_public_key.json" '. + {foreign_tx_domain_public_key:$key[0]}' \
    "$OUTPUT_DIR/report.json" > "$OUTPUT_DIR/report.tmp"
  mv "$OUTPUT_DIR/report.tmp" "$OUTPUT_DIR/report.json"
fi

if [[ -f "$OUTPUT_DIR/sign_domain_public_key.json" ]]; then
  jq --slurpfile key "$OUTPUT_DIR/sign_domain_public_key.json" '. + {sign_domain_public_key:$key[0]}' \
    "$OUTPUT_DIR/report.json" > "$OUTPUT_DIR/report.tmp"
  mv "$OUTPUT_DIR/report.tmp" "$OUTPUT_DIR/report.json"
fi

if [[ -f "$OUTPUT_DIR/derived_evm_public_key.json" ]]; then
  jq --slurpfile key "$OUTPUT_DIR/derived_evm_public_key.json" --rawfile address "$OUTPUT_DIR/derived_evm_address.txt" \
    '. + {derived_evm_public_key:$key[0],derived_evm_address:($address | rtrimstr("\n"))}' \
    "$OUTPUT_DIR/report.json" > "$OUTPUT_DIR/report.tmp"
  mv "$OUTPUT_DIR/report.tmp" "$OUTPUT_DIR/report.json"
fi

cat "$OUTPUT_DIR/report.json"
