# Benchmark Suite for createrepo_rs vs createrepo_c

Comparative benchmarking between `createrepo_rs` (Rust) and `createrepo_c` (C).

## Quick Start

```bash
# 1. Generate test RPMs
./benchmark/generate_rpms.sh 500

# 2. Build createrepo_rs release
cargo build --release

# 3. Run createrepo_rs benchmark
./benchmark/run_bench_rs.sh /tmp/bench_rpms 500 4

# 4. Run createrepo_c benchmark (requires Docker)
./benchmark/run_bench_c.sh /tmp/bench_rpms 500 4

# 5. Compare results
./benchmark/compare.sh
```

## Files

| File | Purpose |
|------|---------|
| `generate_rpms.sh` | Generate N test RPMs by copying existing ones |
| `run_bench_rs.sh` | Benchmark createrepo_rs with `time` |
| `run_bench_c.sh` | Benchmark createrepo_c in Docker |
| `Dockerfile.createrepo_c` | Fedora image with createrepo_c installed |
| `compare.sh` | Compare output and timing between the two |
| `bench_all.sh` | Run full benchmark suite end-to-end |

## Test RPMs

The test RPMs are copies of the fake RPMs from `docker-test/repo/`.
These are minimal valid RPM packages created for testing purposes.

## Results (M1 Pro, 2026-04)

| Metric | createrepo_rs | createrepo_c | Speedup |
|--------|---------------|--------------|---------|
| Full gen (500 RPMs, 4w) | ~0.04s | ~0.20s | **5x** |
| Update (500 RPMs, 4w) | ~0.01s | ~0.15s | **15x** |
| Binary size | 3.5MB static | ~200KB + libs | — |
| Dependencies | 0 FFI | 5+ C libs | — |
