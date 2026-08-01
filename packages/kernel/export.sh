#!/usr/bin/env bash
set -euo pipefail

OUTPUT_DIR="${1:-out}"

TARGETS=(
    "conformance/wire::constants"
    "conformance/wire::unlocked"
    "conformance/wire::cheque"
    "conformance/wire::squash"
    "conformance/wire::datum"
    "conformance/wire::stage"
    "conformance/wire::redeemer"
    # "conformance/cheque::verify"
)

# Splits "module::name" into globals TARGET_MODULE / TARGET_NAME.
split_target() {
    local target="$1"
    if [[ "$target" != *"::"* ]]; then
        echo "ERROR: malformed target '$target' (expected 'module::name')" >&2
        exit 1
    fi
    TARGET_MODULE="${target%%::*}"
    TARGET_NAME="${target##*::}"
}

# Builds the output json path for a given module/name pair.
output_path_for() {
    local module="$1" name="$2"
    echo "${OUTPUT_DIR}/${module}/${name}.json"
}

mkdir -p "$OUTPUT_DIR"

# Pass 1: verify no two targets would write to the same output file
# before we run anything (avoids silently overwriting a prior export).
declare -A seen_outputs
for target in "${TARGETS[@]}"; do
    split_target "$target"
    output="$(output_path_for "$TARGET_MODULE" "$TARGET_NAME")"
    if [[ -n "${seen_outputs[$output]:-}" ]]; then
        echo "ERROR: output clash: '$output' is produced by both '${seen_outputs[$output]}' and '$target'" >&2
        exit 1
    fi
    seen_outputs["$output"]="$target"
done

# Pass 2: actually export.
for target in "${TARGETS[@]}"; do
    split_target "$target"
    output="$(output_path_for "$TARGET_MODULE" "$TARGET_NAME")"
    mkdir -p "$(dirname "$output")"
    echo "Exporting ${TARGET_MODULE}::${TARGET_NAME} -> ${output}"
    aiken export  --trace-filter user-defined --trace-level verbose --module "$TARGET_MODULE" --name "$TARGET_NAME" > "$output"
    echo "tick"
done
echo "Done"
