#!/usr/bin/env bash
# Closes and ends the two on-chain Konduit channels opened by
# fund-cln-wallets.sh printed (consumer -> gateway, gateway -> merchant),
# against an already-running `process-compose-dev-env`.
#
# Not wired into process-compose: run this yourself, from inside the nix
# dev shell, before tearing the testnet down, if you want the demo wallets
# to reclaim their locked ADA rather than just abandoning it.
#
# Each party's wallet key is read from the `.env.<role>` that
# cln-demo-configs wrote alongside its CLN TOML, same as fund-cln-wallets.sh.
#
# Usage: scripts/teardown-cln-channels.sh
set -euo pipefail

: "${CARDANO_NODE_SOCKET_PATH:?}"
: "${CARDANO_NODE_NETWORK_ID:?}"
: "${CLN_DEMO_CONFIG_DIR:?}"
: "${KUPO_UTXO_PORT:?}"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${RUN_DIR:-./.run}/cargo-target-cli}"
mkdir -p "$CARGO_TARGET_DIR"

export KONDUIT_CARDANO_BACKEND="kupo-node"
export KONDUIT_NODE_SOCKET="$CARDANO_NODE_SOCKET_PATH"
export KONDUIT_NETWORK="$CARDANO_NODE_NETWORK_ID"
export KONDUIT_KUPO_URI="http://127.0.0.1:$KUPO_UTXO_PORT"

konduit_cli() {
  cargo run -q -p konduit-cli -- "$@"
}

# Resolved the same way fund-cln-wallets.sh resolves it, so both always
# agree on which reference-script deployment is live.
admin_config="$(konduit_cli admin show config)"
export KONDUIT_HOST_ADDRESS
KONDUIT_HOST_ADDRESS="$(sed -n 's/^host_address = //p' <<< "$admin_config")"
: "${KONDUIT_HOST_ADDRESS:?teardown-cln-channels needs the host address}"

wallet_of() {
  sed -n 's/^KONDUIT_WALLET=//p' "$CLN_DEMO_CONFIG_DIR/.env.$1"
}

tag_hex() {
  printf '%s' "$1" | od -An -tx1 | tr -d ' \n'
}

# The label of $tag in $wallet_role's own view of the chain, or empty if the
# channel isn't there any more (already ended, funds already reclaimed).
stage_label() {
  local wallet="$1" tag="$2"
  KONDUIT_WALLET="$wallet" konduit_cli consumer show tip --output-json \
    | jq -r --arg tag "$tag" '.channels[] | select(.tag == $tag) | .stage.label // empty'
}

elapse_at_ms() {
  local wallet="$1" tag="$2"
  KONDUIT_WALLET="$wallet" konduit_cli consumer show tip --output-json \
    | jq -r --arg tag "$tag" '.channels[] | select(.tag == $tag) | .stage.elapse_at_ms // empty'
}

# Submits a close for $tag_ascii if it's still open, then waits for it to
# land as "closed". Leaves $wallet/$tag in the global closing[] / tags[]
# arrays for the end-step pass below, unless the channel is already gone.
close_channel() {
  local wallet_role="$1" tag_ascii="$2"
  local wallet tag label
  wallet="$(wallet_of "$wallet_role")"
  tag="$(tag_hex "$tag_ascii")"
  label="$(stage_label "$wallet" "$tag")"

  case "$label" in
    "")
      echo "channel $tag_ascii already ended"
      return 0
      ;;
    opened)
      local tx_id
      tx_id="$(KONDUIT_WALLET="$wallet" konduit_cli consumer tx --close "$tag" | tr -d '"')"
      echo "closing channel $tag_ascii: tx $tx_id"
      for _ in $(seq 60); do
        label="$(stage_label "$wallet" "$tag")"
        [ "$label" = "closed" ] && break
        sleep 2
      done
      if [ "$label" != "closed" ]; then
        echo "channel $tag_ascii did not confirm as closed" >&2
        exit 1
      fi
      echo "channel $tag_ascii closed"
      ;;
    closed)
      echo "channel $tag_ascii already closed"
      ;;
  esac

  pending_wallets+=("$wallet")
  pending_wallet_roles+=("$wallet_role")
  pending_tags+=("$tag")
  pending_tag_ascii+=("$tag_ascii")
}

pending_wallets=()
pending_wallet_roles=()
pending_tags=()
pending_tag_ascii=()

close_channel gateway gateway-merchant
close_channel consumer consumer-gateway

if [ "${#pending_tags[@]}" -eq 0 ]; then
  echo "nothing left to tear down"
else
  # Every pending channel is "closed"; wait out the latest elapse_at_ms
  # (plus a small margin) before the on-chain "end" step becomes valid.
  max_elapse_ms=0
  for i in "${!pending_tags[@]}"; do
    ms="$(elapse_at_ms "${pending_wallets[$i]}" "${pending_tags[$i]}")"
    [ -n "$ms" ] && [ "$ms" -gt "$max_elapse_ms" ] && max_elapse_ms="$ms"
  done

  if [ "$max_elapse_ms" -gt 0 ]; then
    now_ms="$(date +%s%3N)"
    wait_s=$(( (max_elapse_ms - now_ms) / 1000 + 5 ))
    if [ "$wait_s" -gt 0 ]; then
      echo "waiting ${wait_s}s for the close period to elapse"
      sleep "$wait_s"
    fi
  fi

  # One "end" per owning wallet reclaims every one of its closed channels
  # that's past its elapse time; no --close/--open needed, it's inferred.
  declare -A ended_roles=()
  for role in "${pending_wallet_roles[@]}"; do
    [ -n "${ended_roles[$role]:-}" ] && continue
    ended_roles["$role"]=1
    tx_id="$(KONDUIT_WALLET="$(wallet_of "$role")" konduit_cli consumer tx | tr -d '"')"
    echo "ending $role's channel(s): tx $tx_id"
  done

  for i in "${!pending_tags[@]}"; do
    wallet="${pending_wallets[$i]}"
    tag="${pending_tags[$i]}"
    tag_ascii="${pending_tag_ascii[$i]}"
    for _ in $(seq 60); do
      [ -z "$(stage_label "$wallet" "$tag")" ] && break
      sleep 2
    done
    if [ -n "$(stage_label "$wallet" "$tag")" ]; then
      echo "channel $tag_ascii did not confirm as ended" >&2
      exit 1
    fi
    echo "channel $tag_ascii ended"
  done
fi

echo
"$(dirname "${BASH_SOURCE[0]}")/show-balances.sh"
