#!/usr/bin/env bash
# cargo-check-combos.sh
set -e

MANIFEST="--manifest-path $(dirname "$0")/Cargo.toml"
WASM="--target wasm32-unknown-unknown"

ALL_FEATURES=("std" "json" "cbor" "reqwest" "gloo" "bindgen")
# Reqwest has issues when targetting wasm. That's why we have gloo
WASM_FEATURES=("std" "json" "cbor" "gloo" "bindgen")

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
  echo "--- checking: $features ---"
  cargo check $MANIFEST --no-default-features --features "$features"
done

echo "=== Wasm combinations ==="
for features in $(subsets "${WASM_FEATURES[@]}"); do
  echo "--- checking: $features ---"
  cargo check $MANIFEST --no-default-features --features "$features" $WASM
done

echo "✅ All combinations passed"
