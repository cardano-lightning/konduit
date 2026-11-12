#!/usr/bin/env bash
# Prints a human-readable status report for the CLN demo: each actor's
# wallet address + ADA balance, followed by any Konduit channel it owns
# on-chain (tag, stage, locked amount). Shared by fund-cln-wallets.sh and
# teardown-cln-channels.sh so both report the same way; after teardown
# there are no channels left to list, so it's just the three balances.
#
# Usage: flake/heavy/scripts/show-balances.sh
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

# Resolved the same way setup/teardown resolve it, so all three always
# agree on which reference-script deployment is live.
admin_config="$(konduit_cli admin show config)"
export KONDUIT_HOST_ADDRESS
KONDUIT_HOST_ADDRESS="$(sed -n 's/^host_address = //p' <<< "$admin_config")"
: "${KONDUIT_HOST_ADDRESS:?show-balances needs the host address}"

wallet_of() {
  sed -n 's/^KONDUIT_WALLET=//p' "$CLN_DEMO_CONFIG_DIR/.env.$1"
}

ada() {
  awk -v l="$1" 'BEGIN { printf "%.6f", l / 1000000 }'
}

# Best-effort hex -> ascii for tags; falls back to the hex itself for
# anything that isn't printable ascii (tags in this demo always are).
tag_ascii_of() {
  local hex="$1" ascii="" i byte val
  for ((i = 0; i < ${#hex}; i += 2)); do
    byte="0x${hex:i:2}"
    val=$((byte))
    if [ "$val" -ge 32 ] && [ "$val" -le 126 ]; then
      ascii+="$(printf "\\$(printf '%03o' "$val")")"
    else
      echo "$hex"
      return
    fi
  done
  echo "$ascii"
}

describe_channel() {
  local now_ms="$1"
  jq -r --arg now_ms "$now_ms" '
    ($now_ms | tonumber) as $now |
    "  channel " + .tag_ascii
      + "  " + .stage.label
      + "  locked=" + (.amount_lovelace / 1000000 | tostring) + " ADA"
      + (
          if .stage.label == "opened" then
            "  used=" + (.stage.subbed / 1000000 | tostring) + " ADA"
          elif .stage.label == "closed" then
            "  used=" + (.stage.subbed / 1000000 | tostring) + " ADA"
            + (
                if .stage.elapse_at_ms > $now then
                  "  elapses in " + (((.stage.elapse_at_ms - $now) / 1000) | floor | tostring) + "s"
                else
                  "  ready to end"
                end
              )
          elif .stage.label == "responded" then
            "  pending=" + (.stage.pendings_amount / 1000000 | tostring) + " ADA"
          else
            ""
          end
        )
  '
}

now_ms="$(date +%s%3N)"

for role in consumer gateway merchant; do
  wallet="$(wallet_of "$role")"
  address="$(KONDUIT_WALLET="$wallet" konduit_cli consumer show address)"
  lovelace="$(cardano-cli query utxo --address "$address" \
    --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
    --socket-path "$CARDANO_NODE_SOCKET_PATH" \
    --output-json | jq '[.[] | .value.lovelace] | add // 0')"

  printf '%-10s %s  %s ADA\n' "$role" "$address" "$(ada "$lovelace")"

  channels_json="$(KONDUIT_WALLET="$wallet" konduit_cli consumer show tip --output-json | jq -c '.channels // []')"
  count="$(jq 'length' <<< "$channels_json")"
  for ((i = 0; i < count; i++)); do
    channel="$(jq -c ".[$i]" <<< "$channels_json")"
    tag_ascii="$(tag_ascii_of "$(jq -r '.tag' <<< "$channel")")"
    jq -c --arg tag_ascii "$tag_ascii" '. + {tag_ascii: $tag_ascii}' <<< "$channel" \
      | describe_channel "$now_ms"
  done
done
