#!/usr/bin/env bash
# Submits a null (zero-amount) squash for each open Konduit channel in the
# CLN demo (consumer -> gateway, gateway -> merchant), against an
# already-running `process-compose-dev-env`.
#
# Why this is needed: CLN-level payments (quote/commit/secret-reveal)
# settle entirely independently of the L1 receipt that
# admin::Service::claim() reads from its own db (see
# admin/service.rs's "channels in db vs. ones with a receipt" trace).
# Nothing in this demo otherwise ever POSTs a squash to a konduit-server's
# /ch/squash, so that receipt never exists and claim()/respond() always
# have nothing to act on. This submits the initial zero-amount squash the
# server error "no receipt: submit a null squash first" (channel.rs)
# refers to, so there's at least a baseline receipt to build on.
#
# Unlike setup/teardown-cln-channels.sh, this never touches L1 at all: it's
# a pure L2/HTTP action against the running konduit-server processes, via
# konduit-client's own `l2 squash` (packages/konduit/client/src/cli.rs),
# not konduit-cli.
#
# Not wired into process-compose: run this yourself, from inside the nix
# dev shell, once setup-cln-channels.sh has opened the channels.
#
# Usage: scripts/submit-null-squash.sh
set -euo pipefail

: "${CLN_DEMO_CONFIG_DIR:?}"
: "${KONDUIT_SERVER_GATEWAY_PORT:?}"
: "${KONDUIT_SERVER_MERCHANT_PORT:?}"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${RUN_DIR:-./.run}/cargo-target-cli}"
mkdir -p "$CARGO_TARGET_DIR"

wallet_of() {
  sed -n 's/^KONDUIT_WALLET=//p' "$CLN_DEMO_CONFIG_DIR/.env.$1"
}

tag_hex() {
  printf '%s' "$1" | od -An -tx1 | tr -d ' \n'
}

# Posts a zero-amount squash for $tag_ascii, owned by $wallet_role, to
# $adaptor_url, then lets konduit-client's own sync loop settle it
# (-y skips its interactive confirmation, since nothing here is
# unexpected: a first squash is always zero/null).
submit_null_squash() {
  local wallet_role="$1" tag_ascii="$2" adaptor_url="$3"
  echo "submitting null squash for $tag_ascii"
  KONDUIT_SIGNING_KEY="$(wallet_of "$wallet_role")" \
    KONDUIT_TAG="$(tag_hex "$tag_ascii")" \
    KONDUIT_SERVER_URL="$adaptor_url" \
    cargo run --release -q -p konduit-client -- -y l2 squash
}

submit_null_squash consumer consumer-gateway "http://127.0.0.1:$KONDUIT_SERVER_GATEWAY_PORT"
submit_null_squash gateway gateway-merchant "http://127.0.0.1:$KONDUIT_SERVER_MERCHANT_PORT"

echo "null squashes submitted; admin's claim ticker should now see a receipt for each channel"
