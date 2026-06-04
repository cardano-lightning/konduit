#!/usr/bin/env bash
# check-all.sh
set -e

MANIFEST="--manifest-path $(dirname "$0")/Cargo.toml"
WASM="--target wasm32-unknown-unknown"

checks=(
  "native,json"
  "native,cbor"
  "native,json,cbor"
  "native"
)

wasm_checks=(
  "wasm,json"
  "wasm,cbor" # Wont work until fixed! 
  "wasm,json,cbor"
  "wasm" # Wont work until fixed!
)

for f in "${checks[@]}"; do
  echo "=== native: $f ==="
  cargo check $MANIFEST --no-default-features --features "$f"
done

for f in "${wasm_checks[@]}"; do
  echo "=== wasm: $f ==="
  cargo check $MANIFEST --no-default-features --features "$f" $WASM
done

echo "✅ All combinations passed"
