#!/bin/bash
# Generate N test RPMs by copying docker-test/repo/*.rpm
# Usage: ./generate_rpms.sh [COUNT] [OUTPUT_DIR]
set -e

COUNT="${1:-500}"
OUTPUT="${2:-/tmp/bench_rpms}"
SOURCE_DIR="docker-test/repo"

rm -rf "$OUTPUT"
mkdir -p "$OUTPUT"

echo "Generating $COUNT test RPMs in $OUTPUT..."

SRC_RPMS=($(find "$SOURCE_DIR" -name "*.rpm" -not -name "*.src.rpm"))

for i in $(seq 1 "$COUNT"); do
    idx=$((RANDOM % ${#SRC_RPMS[@]}))
    src="${SRC_RPMS[$idx]}"
    base=$(basename "$src" .rpm)
    cp "$src" "$OUTPUT/${base}_bench${i}.rpm"
done

echo "Done: $(ls "$OUTPUT"/*.rpm | wc -l) RPMs"
du -sh "$OUTPUT"
