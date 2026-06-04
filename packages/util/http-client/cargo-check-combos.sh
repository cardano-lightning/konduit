#!/usr/bin/env bash
# cargo-check-combos.sh
set -e

MANIFEST="--manifest-path $(dirname "$0")/Cargo.toml"
WASM="--target wasm32-unknown-unknown"

# NOTE: `http` crate (1.x) requires std unconditionally via compile_error!
# Until hyperium/http lands no_std support, all feature combos must include std.
# Re-evaluate when http >= X.Y.Z.
REQUIRED="std"

ALL_FEATURES=("json" "cbor" "reqwest" "gloo" "bindgen")
# Reqwest has issues when targetting wasm. That's why we have gloo
WASM_FEATURES=("json" "cbor" "gloo" "bindgen")

subsets() {
  local arr=("$@")
  local n=${#arr[@]}
  for ((i = 1; i < (1 << n); i++)); do
    local subset=()
    for ((j = 0; j < n; j++)); do
      if (( (i >> j) & 1 )); then
        subset+=("${arr[$j]}")
      fi
    done
    echo "$(IFS=,; echo "${subset[*]}")"
  done
}

echo "=== Native combinations ==="
for features in $(subsets "${ALL_FEATURES[@]}"); do
  echo "--- checking: $REQUIRED,$features ---"
  cargo check $MANIFEST --no-default-features --features "$REQUIRED,$features"
done

echo "=== Wasm combinations ==="
for features in $(subsets "${WASM_FEATURES[@]}"); do
  echo "--- checking: $REQUIRED,$features ---"
  cargo check $MANIFEST --no-default-features --features "$REQUIRED,$features" $WASM
done

echo "✅ All combinations passed"
