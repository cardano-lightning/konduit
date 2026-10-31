#!/usr/bin/env bash
# Opens the two on-chain Konduit channels the CLN demo needs
# (consumer -> gateway, gateway -> merchant), by hand, against an
# already-running `process-compose-dev-env`.
#
# Not wired into process-compose: run this yourself, from inside the nix
# dev shell, once the testnet + kupo + konduit-server-{gateway,merchant}
# processes are up and `cln-demo-configs` has run.
#
# Each party's wallet key is read from the `.env.<role>` that
# cln-demo-configs wrote alongside its CLN TOML (see
# packages/cln/server/examples/consumer_gateway_merchant.rs) — the same key
# that party's konduit-server uses as KONDUIT_SIGNING_KEY, so the on-chain
# channel's identity always lines up with the key its CLN context signs
# commits with. The counterparty's sub-key and close-period are not
# hand-copied: they're pulled live from that server's own `/info`, since
# that's the one place they're guaranteed to be correct.
#
# Usage: scripts/setup-cln-channels.sh
set -euo pipefail

: "${CARDANO_NODE_SOCKET_PATH:?}"
: "${CARDANO_NODE_NETWORK_ID:?}"
: "${FAUCET_ADDR_FILE:?}"
: "${FAUCET_SKEY_FILE:?}"
: "${CLN_DEMO_CONFIG_DIR:?}"
: "${KONDUIT_SERVER_GATEWAY_PORT:?}"
: "${KONDUIT_SERVER_MERCHANT_PORT:?}"
: "${KUPO_UTXO_PORT:?}"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${RUN_DIR:-./.run}/cargo-target-cli}"
mkdir -p "$CARGO_TARGET_DIR"

export KONDUIT_CARDANO_BACKEND="kupo-node"
export KONDUIT_NODE_SOCKET="$CARDANO_NODE_SOCKET_PATH"
export KONDUIT_NETWORK="$CARDANO_NODE_NETWORK_ID"
export KONDUIT_KUPO_URI="http://127.0.0.1:$KUPO_UTXO_PORT"

konduit_cli() {
  cargo run --release -q -p konduit-cli -- "$@"
}

# Resolved the same way publish-reference-script resolves it, so both always
# agree on which reference-script deployment is live.
admin_config="$(konduit_cli admin show config)"
export KONDUIT_HOST_ADDRESS
KONDUIT_HOST_ADDRESS="$(sed -n 's/^host_address = //p' <<< "$admin_config")"
: "${KONDUIT_HOST_ADDRESS:?setup-cln-channels needs the host address}"

CHANNEL_ADA=10
WALLET_MIN_LOVELACE=15000000 # channel amount + buffer/fees headroom
WALLET_FUNDS=50000000

wallet_of() {
  sed -n 's/^KONDUIT_WALLET=//p' "$CLN_DEMO_CONFIG_DIR/.env.$1"
}

balance_of() {
  cardano-cli query utxo --address "$1" \
    --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
    --socket-path "$CARDANO_NODE_SOCKET_PATH" \
    --output-json | jq '[.[] | .value.lovelace] | add // 0'
}

fund_if_needed() {
  local role="$1" address="$2"
  if [ "$(balance_of "$address")" -ge "$WALLET_MIN_LOVELACE" ]; then
    echo "$role wallet already funded"
    return 0
  fi

  local faucet_address faucet_utxo tx_body tx_signed
  faucet_address="$(cat "$FAUCET_ADDR_FILE")"
  faucet_utxo="$(cardano-cli query utxo --address "$faucet_address" \
    --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
    --socket-path "$CARDANO_NODE_SOCKET_PATH" \
    --output-json | jq -r 'keys[0] // empty')"
  if [ -z "$faucet_utxo" ]; then
    echo "the faucet $faucet_address has no utxo to fund $address with" >&2
    exit 1
  fi

  local tx_hash
  tx_body="$(mktemp)"
  tx_signed="$(mktemp)"
  cardano-cli conway transaction build \
    --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
    --socket-path "$CARDANO_NODE_SOCKET_PATH" \
    --tx-in "$faucet_utxo" \
    --tx-out "$address+$WALLET_FUNDS" \
    --change-address "$faucet_address" \
    --out-file "$tx_body" \
    > /dev/null
  cardano-cli conway transaction sign \
    --tx-body-file "$tx_body" \
    --signing-key-file "$FAUCET_SKEY_FILE" \
    --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
    --out-file "$tx_signed" \
    > /dev/null
  # `submit` splits its own confirmation text across stdout/stderr in a way
  # that drops the hash half when stdout is redirected, so get it ourselves
  # from the signed tx instead of relying on submit's own printout.
  tx_hash="$(cardano-cli conway transaction txid --tx-file "$tx_signed")"
  cardano-cli conway transaction submit \
    --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
    --socket-path "$CARDANO_NODE_SOCKET_PATH" \
    --tx-file "$tx_signed" \
    > /dev/null
  echo "funding $role wallet from faucet: tx $tx_hash"

  for _ in $(seq 60); do
    if [ "$(balance_of "$address")" -ge "$WALLET_MIN_LOVELACE" ]; then
      echo "$role wallet funded"
      return 0
    fi
    sleep 2
  done
  echo "funding $role wallet ($address) did not confirm" >&2
  exit 1
}

channel_opened() {
  local wallet="$1" tag="$2"
  KONDUIT_WALLET="$wallet" konduit_cli consumer show tip --output-json \
    | jq -e --arg tag "$tag" \
        '.channels[] | select(.tag == $tag) | select(.stage.label == "opened")' \
    > /dev/null
}

# Opens $tag_ascii's channel against $adaptor_url, funded by $wallet_role's
# wallet, fetching the counterparty's sub-key and close-period from its own
# /info.
open_channel() {
  local wallet_role="$1" tag_ascii="$2" adaptor_url="$3"
  local wallet tag info sub_vkey close_period_secs

  wallet="$(wallet_of "$wallet_role")"
  tag="$(printf '%s' "$tag_ascii" | od -An -tx1 | tr -d ' \n')"

  if channel_opened "$wallet" "$tag"; then
    echo "channel $tag_ascii already opened"
    return 0
  fi

  info="$(curl --fail --silent --show-error -H 'Accept: application/json' "$adaptor_url/info")"
  sub_vkey="$(jq -r '.channel_parameters.adaptor_key' <<< "$info")"
  close_period_secs="$(jq -r '.channel_parameters.close_period.secs' <<< "$info")"

  local tx_id
  tx_id="$(KONDUIT_WALLET="$wallet" konduit_cli consumer tx \
    --open "$tag,$sub_vkey,${close_period_secs}s,$CHANNEL_ADA" | tr -d '"')"
  echo "opening channel $tag_ascii: tx $tx_id"

  for _ in $(seq 60); do
    if channel_opened "$wallet" "$tag"; then
      echo "channel $tag_ascii opened"
      return 0
    fi
    sleep 2
  done
  echo "channel $tag_ascii did not confirm as opened" >&2
  exit 1
}

# consumer and gateway each fund an on-chain channel open; merchant never
# opens one, but its konduit-server's own admin wallet (same key, see
# wallet_of) still needs fee money to submit its on-chain claim/respond
# txs, so it gets funded too.
fund_if_needed consumer "$(KONDUIT_WALLET="$(wallet_of consumer)" konduit_cli consumer show address)"
fund_if_needed gateway "$(KONDUIT_WALLET="$(wallet_of gateway)" konduit_cli consumer show address)"
fund_if_needed merchant "$(KONDUIT_WALLET="$(wallet_of merchant)" konduit_cli consumer show address)"

open_channel consumer consumer-gateway "http://127.0.0.1:$KONDUIT_SERVER_GATEWAY_PORT"
open_channel gateway gateway-merchant "http://127.0.0.1:$KONDUIT_SERVER_MERCHANT_PORT"

echo "both channels opened; konduit-indexer polls every 10s, so give it a" \
     "cycle before paying through them (Data::assert_cln_peer_backed checks" \
     "each server's own locally-indexed view, not the chain directly)."
echo
"$(dirname "${BASH_SOURCE[0]}")/show-balances.sh"
