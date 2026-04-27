#!/bin/bash
# Benchmark createrepo_rs
# Usage: ./run_bench_rs.sh <RPM_DIR> <COUNT> <WORKERS>
set -e

RPM_DIR="${1:-/tmp/bench_rpms}"
COUNT="${2:-500}"
WORKERS="${3:-4}"
BINARY="${CREATEREPO_RS_BIN:-./target/release/createrepo_rs}"
OUT_DIR="/tmp/bench_rs_out"

echo "=== createrepo_rs benchmark ==="
echo "RPMs: $COUNT | Workers: $WORKERS | Binary: $BINARY"
echo ""

for run in 1 2 3; do
    rm -rf "$OUT_DIR"
    echo -n "Run $run: "
    /usr/bin/time -p "$BINARY" "$RPM_DIR" \
        -o "$OUT_DIR" \
        --workers="$WORKERS" \
        --compress-type=zstd \
        --no-database \
        2>&1 | grep real | awk '{print $2"s"}'
done

echo ""
echo "Output size: $(du -sh "$OUT_DIR/repodata" 2>/dev/null | cut -f1)"

echo ""
echo "=== Update mode ==="
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"
"$BINARY" "$RPM_DIR" -o "$OUT_DIR" --workers="$WORKERS" --compress-type=zstd --no-database -q
for run in 1 2 3; do
    echo -n "Update run $run: "
    /usr/bin/time -p "$BINARY" "$RPM_DIR" \
        -o "$OUT_DIR" \
        --update \
        --skip-stat \
        --workers="$WORKERS" \
        --compress-type=zstd \
        --no-database \
        2>&1 | grep real | awk '{print $2"s"}'
done
