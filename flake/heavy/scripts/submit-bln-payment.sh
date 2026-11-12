#!/usr/bin/env bash
set -euo pipefail

: "${KONDUIT_BLN_ENV_FILE:?Set KONDUIT_BLN_ENV_FILE to your private BLN env file}"
: "${CLN_DEMO_CONFIG_DIR:?Start the heavy development shell first}"
: "${KONDUIT_SERVER_GATEWAY_PORT:?}"
[ -r "$KONDUIT_BLN_ENV_FILE" ] || { echo "BLN env file is not readable" >&2; exit 1; }
set -a
# shellcheck disable=SC1090
source "$KONDUIT_BLN_ENV_FILE"
set +a

case "${KONDUIT_BLN_BACKEND:-}" in
  lnd)
    : "${KONDUIT_BLN_URL:?}"
    : "${KONDUIT_BLN_MACAROON:?}"
    export LND_BASE_URL="$KONDUIT_BLN_URL" LND_MACAROON="$KONDUIT_BLN_MACAROON"
    ;;
  cln)
    : "${KONDUIT_BLN_URL:?}"
    : "${KONDUIT_BLN_RUNE:?}"
    export CLN_BASE_URL="$KONDUIT_BLN_URL" CLN_RUNE="$KONDUIT_BLN_RUNE"
    ;;
  *) echo "KONDUIT_BLN_BACKEND must be lnd or cln" >&2; exit 1 ;;
esac

invoice_json="$(cargo run -q -p bln-client --features cli -- invoice \
  --amount-msat "${KONDUIT_BLN_INVOICE_AMOUNT_MSAT:-1000000}" \
  --description "Konduit local BLN demo")"
invoice="$(jq -er '.bolt11' <<< "$invoice_json")"
echo "Invoice created on configured $KONDUIT_BLN_BACKEND node. Starting consented payment."
set -a
source "$CLN_DEMO_CONFIG_DIR/consumer.env"
set +a
cargo run -q -p konduit-client --features bln -- l2 bln \
  --payee "http://127.0.0.1:$KONDUIT_SERVER_GATEWAY_PORT" \
  --invoice "$invoice" --amount "${KONDUIT_BLN_INBOUND_AMOUNT_LOVELACE:-10000000}"
