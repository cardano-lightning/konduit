#!/usr/bin/env bash
# Funds the three CLN demo wallets (consumer, gateway, merchant) from the
# faucet of an already-running `process-compose-dev-env`, and prints what
# each participant needs to open its own channel by hand.
#
# This opens nothing. Channel opens are left to you, one konduit-cli call
# per participant, so each party's open is its own deliberate act. What
# this script does is the part you cannot do by hand: it moves faucet
# money, and it resolves the three values an open needs that aren't
# guessable — the counterparty's adaptor sub-key and close-period (pulled
# live from that server's own /info, the one place they're guaranteed
# correct) and the reference-script host address.
#
# Each party's wallet key is read from the `.env.<role>` that
# cln-demo-configs wrote alongside its CLN TOML (see
# packages/cln/rail/examples/consumer_gateway_merchant.rs) — the same key
# that party's konduit-server uses as KONDUIT_SIGNING_KEY, so the on-chain
# channel's identity always lines up with the key its CLN context signs
# commits with.
#
# Output discipline: stdout is nothing but `export` lines, so it can be
# sourced or eval'd; the narration and the suggested commands go to
# stderr. The same exports are written to $RUN_DIR/cln-channels.env.
#
# Usage:
#   flake/heavy/scripts/fund-cln-wallets.sh              # fund, then print
#   eval "$(flake/heavy/scripts/fund-cln-wallets.sh)"    # ... and load into this shell
#   source "$RUN_DIR/cln-channels.env"       # reload later, no funding
#   flake/heavy/scripts/fund-cln-wallets.sh --print-only # resolve and print, fund nothing
#
# Env knobs:
#   CHANNEL_ADA (default 10)  amount the printed opens carry
#   TAG_SUFFIX  (default "")  appended to both tag names, so the same keys
#                             can open further channels: TAG_SUFFIX=-2 gives
#                             consumer-gateway-2 / gateway-merchant-2
#   CONSUMER_GATEWAY_TAG_ASCII / GATEWAY_MERCHANT_TAG_ASCII
#                             override a tag name outright
#
# Only the default tag names are wired into the CLN rail configs, so channels
# on other tags are ordinary Konduit channels that the CLN demo won't route
# through.
set -euo pipefail

: "${CARDANO_NODE_SOCKET_PATH:?}"
: "${CARDANO_NODE_NETWORK_ID:?}"
: "${FAUCET_ADDR_FILE:?}"
: "${FAUCET_SKEY_FILE:?}"
: "${CLN_DEMO_CONFIG_DIR:?}"
: "${KONDUIT_SERVER_GATEWAY_PORT:?}"
: "${KONDUIT_SERVER_MERCHANT_PORT:?}"
: "${KUPO_UTXO_PORT:?}"

PRINT_ONLY=0
case "${1:-}" in
  --print-only) PRINT_ONLY=1 ;;
  "") ;;
  *) echo "unknown argument: $1 (expected --print-only)" >&2; exit 2 ;;
esac

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${RUN_DIR:-./.run}/cargo-target-cli}"
mkdir -p "$CARGO_TARGET_DIR"

KUPO_URI="http://127.0.0.1:$KUPO_UTXO_PORT"
export KONDUIT_CARDANO_BACKEND="kupo-node"
export KONDUIT_NODE_SOCKET="$CARDANO_NODE_SOCKET_PATH"
export KONDUIT_NETWORK="$CARDANO_NODE_NETWORK_ID"
export KONDUIT_KUPO_URI="$KUPO_URI"

CHANNEL_ADA="${CHANNEL_ADA:-10}"
TAG_SUFFIX="${TAG_SUFFIX:-}"
CONSUMER_GATEWAY_TAG_ASCII="${CONSUMER_GATEWAY_TAG_ASCII:-consumer-gateway$TAG_SUFFIX}"
GATEWAY_MERCHANT_TAG_ASCII="${GATEWAY_MERCHANT_TAG_ASCII:-gateway-merchant$TAG_SUFFIX}"
WALLET_MIN_LOVELACE=15000000 # channel amount + buffer/fees headroom
WALLET_FUNDS=50000000

say() { echo "$@" >&2; }

konduit_cli() {
  cargo run -q -p konduit-cli -- "$@"
}

# Resolved the same way publish-reference-script resolves it, so every
# party agrees on which reference-script deployment is live.
admin_config="$(konduit_cli admin show config)"
export KONDUIT_HOST_ADDRESS
KONDUIT_HOST_ADDRESS="$(sed -n 's/^host_address = //p' <<< "$admin_config")"
: "${KONDUIT_HOST_ADDRESS:?fund-cln-wallets needs the host address}"

wallet_of() {
  sed -n 's/^KONDUIT_WALLET=//p' "$CLN_DEMO_CONFIG_DIR/.env.$1"
}

tag_hex() {
  printf '%s' "$1" | od -An -tx1 | tr -d ' \n'
}

address_of() {
  KONDUIT_WALLET="$1" konduit_cli consumer show address
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
    say "$role wallet already funded"
    return 0
  fi

  local faucet_address faucet_utxo tx_body tx_signed
  faucet_address="$(cat "$FAUCET_ADDR_FILE")"
  faucet_utxo="$(cardano-cli query utxo --address "$faucet_address" \
    --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
    --socket-path "$CARDANO_NODE_SOCKET_PATH" \
    --output-json | jq -r 'keys[0] // empty')"
  if [ -z "$faucet_utxo" ]; then
    say "the faucet $faucet_address has no utxo to fund $address with"
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
  say "funding $role wallet from faucet: tx $tx_hash"

  for _ in $(seq 60); do
    if [ "$(balance_of "$address")" -ge "$WALLET_MIN_LOVELACE" ]; then
      say "$role wallet funded"
      return 0
    fi
    sleep 2
  done
  say "funding $role wallet ($address) did not confirm"
  exit 1
}

# The counterparty's half of an open: whatever its own server says it is.
# Best effort — funding must not depend on a server being up — so an
# unreachable server leaves its two values empty and says so.
adaptor_info() {
  local url="$1"
  curl --fail --silent --show-error -H 'Accept: application/json' "$url/info" 2>/dev/null || true
}

CONSUMER_WALLET="$(wallet_of consumer)"
GATEWAY_WALLET="$(wallet_of gateway)"
MERCHANT_WALLET="$(wallet_of merchant)"

CONSUMER_ADDRESS="$(address_of "$CONSUMER_WALLET")"
GATEWAY_ADDRESS="$(address_of "$GATEWAY_WALLET")"
MERCHANT_ADDRESS="$(address_of "$MERCHANT_WALLET")"

# consumer and gateway each fund an on-chain channel open; merchant never
# opens one, but its konduit-server's own admin wallet (same key, see
# wallet_of) still needs fee money to submit its on-chain claim/respond
# txs, so it gets funded too.
if [ "$PRINT_ONLY" = 1 ]; then
  say "--print-only: funding nothing"
else
  fund_if_needed consumer "$CONSUMER_ADDRESS"
  fund_if_needed gateway "$GATEWAY_ADDRESS"
  fund_if_needed merchant "$MERCHANT_ADDRESS"
fi

GATEWAY_URL="http://127.0.0.1:$KONDUIT_SERVER_GATEWAY_PORT"
MERCHANT_URL="http://127.0.0.1:$KONDUIT_SERVER_MERCHANT_PORT"

# $1 = counterparty url, $2 = tag of the channel opened against it.
# Prints the --open CSV konduit-cli wants: TAG,ADAPTOR_KEY,CLOSE_PERIOD,ADA.
open_csv() {
  local url="$1" tag="$2" info sub_vkey close_period_secs
  info="$(adaptor_info "$url")"
  if [ -z "$info" ]; then
    say "WARNING: $url/info unreachable; its open CSV is left unresolved"
    echo ""
    return 0
  fi
  sub_vkey="$(jq -r '.channel_parameters.adaptor_key' <<< "$info")"
  close_period_secs="$(jq -r '.channel_parameters.close_period.secs' <<< "$info")"
  echo "$tag,$sub_vkey,${close_period_secs}s,$CHANNEL_ADA"
}

CONSUMER_GATEWAY_TAG="$(tag_hex "$CONSUMER_GATEWAY_TAG_ASCII")"
GATEWAY_MERCHANT_TAG="$(tag_hex "$GATEWAY_MERCHANT_TAG_ASCII")"
CONSUMER_GATEWAY_OPEN="$(open_csv "$GATEWAY_URL" "$CONSUMER_GATEWAY_TAG")"
GATEWAY_MERCHANT_OPEN="$(open_csv "$MERCHANT_URL" "$GATEWAY_MERCHANT_TAG")"

ENV_FILE="${RUN_DIR:-./.run}/cln-channels.env"

# stdout: exports only, so this is sourceable as-is.
{
  echo "# written by scripts/fund-cln-wallets.sh"
  echo "# the chain konduit-cli should talk to: this run's testnet"
  echo "export KONDUIT_CARDANO_BACKEND=kupo-node"
  echo "export KONDUIT_NODE_SOCKET='$CARDANO_NODE_SOCKET_PATH'"
  echo "export KONDUIT_NETWORK='$CARDANO_NODE_NETWORK_ID'"
  echo "export KONDUIT_KUPO_URI='$KUPO_URI'"
  echo "export KONDUIT_HOST_ADDRESS='$KONDUIT_HOST_ADDRESS'"
  echo
  echo "# each participant's wallet key (same key its konduit-server signs with)"
  echo "export KONDUIT_CONSUMER_WALLET='$CONSUMER_WALLET'"
  echo "export KONDUIT_GATEWAY_WALLET='$GATEWAY_WALLET'"
  echo "export KONDUIT_MERCHANT_WALLET='$MERCHANT_WALLET'"
  echo "export KONDUIT_CONSUMER_ADDRESS='$CONSUMER_ADDRESS'"
  echo "export KONDUIT_GATEWAY_ADDRESS='$GATEWAY_ADDRESS'"
  echo "export KONDUIT_MERCHANT_ADDRESS='$MERCHANT_ADDRESS'"
  echo
  echo "# the two servers the opens are made against"
  echo "export KONDUIT_GATEWAY_URL='$GATEWAY_URL'"
  echo "export KONDUIT_MERCHANT_URL='$MERCHANT_URL'"
  echo
  echo "# channel tags ($CONSUMER_GATEWAY_TAG_ASCII, $GATEWAY_MERCHANT_TAG_ASCII), for --tag / KONDUIT_TAG"
  echo "export KONDUIT_CONSUMER_GATEWAY_TAG='$CONSUMER_GATEWAY_TAG'"
  echo "export KONDUIT_GATEWAY_MERCHANT_TAG='$GATEWAY_MERCHANT_TAG'"
  echo
  echo "# ready-made --open CSVs: TAG,ADAPTOR_KEY,CLOSE_PERIOD,ADA"
  echo "export KONDUIT_CONSUMER_GATEWAY_OPEN='$CONSUMER_GATEWAY_OPEN'"
  echo "export KONDUIT_GATEWAY_MERCHANT_OPEN='$GATEWAY_MERCHANT_OPEN'"
} | tee "$ENV_FILE"

say
say "wrote $ENV_FILE — source it to get these back without re-funding."
say
say "Open each channel yourself, one call per participant. Run each open"
say "ONCE: konduit-cli does not reject an open for a tag that already has a"
say "channel, it opens a second one with the same tag. Check first with the"
say "show tip calls below if you are unsure."
say
say "  # consumer -> gateway, funded by the consumer's wallet"
say "  KONDUIT_WALLET=\"\$KONDUIT_CONSUMER_WALLET\" \\"
say "    konduit-cli consumer tx --open \"\$KONDUIT_CONSUMER_GATEWAY_OPEN\""
say
say "  # gateway -> merchant, funded by the gateway's wallet"
say "  KONDUIT_WALLET=\"\$KONDUIT_GATEWAY_WALLET\" \\"
say "    konduit-cli consumer tx --open \"\$KONDUIT_GATEWAY_MERCHANT_OPEN\""
say
say "Then check each party's view of its own channel:"
say
say "  KONDUIT_WALLET=\"\$KONDUIT_CONSUMER_WALLET\" konduit-cli consumer show tip"
say "  KONDUIT_WALLET=\"\$KONDUIT_GATEWAY_WALLET\"  konduit-cli consumer show tip"
say
say "konduit-client can open them too, now that it speaks kupo-node:"
say "  KONDUIT_SIGNING_KEY=\"\$KONDUIT_CONSUMER_WALLET\" \\"
say "    KONDUIT_TAG=\"\$KONDUIT_CONSUMER_GATEWAY_TAG\" \\"
say "    KONDUIT_SERVER_URL=\"\$KONDUIT_GATEWAY_URL\" \\"
say "    konduit-client l1 open $((CHANNEL_ADA * 1000000))"
say
say "Same keys, another pair of channels: re-run with TAG_SUFFIX set, e.g."
  say "  TAG_SUFFIX=-2 flake/heavy/scripts/fund-cln-wallets.sh --print-only"
say "which names the tags ${CONSUMER_GATEWAY_TAG_ASCII%"$TAG_SUFFIX"}-2 / ${GATEWAY_MERCHANT_TAG_ASCII%"$TAG_SUFFIX"}-2 and leaves the wallets alone."
say
say "The indexer polls every 10s, so give it a cycle after the opens before"
say "paying through them (Data::assert_backed reads each server's own"
say "locally-indexed view, not the chain directly)."
