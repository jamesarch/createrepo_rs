#!/bin/bash
# Benchmark createrepo_c in Docker
# Usage: ./run_bench_c.sh <RPM_DIR> <COUNT> <WORKERS>
set -e

RPM_DIR="$(cd "${1:-/tmp/bench_rpms}" && pwd)"
COUNT="${2:-500}"
WORKERS="${3:-4}"
OUT_DIR="/tmp/bench_c_out"

echo "=== createrepo_c benchmark (Docker) ==="
echo "RPMs: $COUNT | Workers: $WORKERS"
echo ""

# Check Docker
if ! docker --version &>/dev/null; then
    echo "ERROR: Docker not found"
    exit 1
fi

# Build image
IMAGE="createrepo_c_bench:latest"
if ! docker image inspect "$IMAGE" &>/dev/null; then
    echo "Building Docker image..."
    docker build -t "$IMAGE" -f benchmark/Dockerfile.createrepo_c .
fi

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

for run in 1 2 3; do
    echo -n "Run $run: "
    docker run --rm \
        -v "$RPM_DIR:/rpms:ro" \
        -v "$OUT_DIR:/out" \
        "$IMAGE" \
        time createrepo_c /rpms \
            -o /tmp/bench_out \
            --workers="$WORKERS" \
            --compress-type=zstd \
            --no-database \
            --quiet 2>&1 | grep real | awk '{print $2"s"}'
    docker run --rm -v "$OUT_DIR:/out" "$IMAGE" cp -r /tmp/bench_out /out/repodata_run${run}
done

echo ""
echo "Output size: $(du -sh "$OUT_DIR" 2>/dev/null | cut -f1)"
