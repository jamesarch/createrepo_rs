# createrepo_rs

[![Rust](https://img.shields.io/badge/Rust-1.76%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-GPL--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/artifactx-rs/createrepo_rs/actions/workflows/ci.yml/badge.svg)](https://github.com/artifactx-rs/createrepo_rs/actions)
[![crates.io](https://img.shields.io/crates/v/createrepo_rs.svg)](https://crates.io/crates/createrepo_rs)

Pure-Rust drop-in for `createrepo_c`. **~4× less memory · zero C dependencies · one static binary · built-in `--dump-manifest`.** Byte-identical output — dnf/yum see an equivalent repo.

```
$ time createrepo_rs /srv/zabbix/ --dump-manifest   # 254 RPMs, 80 cores
real    0m0.078s   ← was 10.5s with rpm -K loop (130×, manifest scan)
```

> Speed is **comparable** to createrepo_c — in a controlled benchmark createrepo_c
> is often a touch faster (it defaults to 5 workers; `--workers` accepts 1–100).
> createrepo_rs is **not** a speed play. The durable wins are **memory (~4× less)**,
> **dependencies (5 vs 53)**, and **byte-identical output** — all reproducible in
> one `docker run`, see [`benchmark/`](benchmark/).

> Production-tested: Zabbix 7.2 mirror, Debian 13, 80-core. Ships in COPR / AUR / Debian.

[中文文档](README_zh.md)

---

## Why

I needed fast RPM metadata generation for a private Zabbix mirror.
`createrepo_c` works, but it pulls `librpm`, `libxml2`, `glib2`, `zchunk` — packaging pain on Debian.
And `rpm -K` loops for manifest scanning were taking 10+ seconds in CI.

Rewrote it in Rust: parallel worker pool, in-memory SQLite (single `VACUUM INTO` flush), zero shared libs.
The result ships as a 3.5 MB musl static binary with no runtime dependencies.

---

## vs createrepo_c

| | createrepo_c | createrepo_rs |
|---|---|---|
| Dependencies | librpm, libxml2, glib2, zchunk... | **zero FFI** — pure Rust crates |
| Binary | ~200KB + shared libs | **3.5MB musl static** |
| Default workers | 5 (`--workers 1–100`) | all CPUs (auto) |
| Memory safety | manual malloc/free | borrow checker |
| Peak memory | ~83 MB | **4–8× less** (10–20 MB) |
| Output | (reference) | **byte-identical, verified** |
| Cross-compile | painful | `cargo zigbuild` |
| `--dump-manifest` | ❌ need `rpm -K` loop | ✅ parallel, 0.078s/254 pkgs |
| Signature detection | ❌ separate `rpm -K` | ✅ built-in |
| I/O timeout | ❌ | ✅ `--timeout` watchdog |
| dnf compatible | ✅ | ✅ verified |
| CLI params | 55 | 53 |

---

## Install

```bash
# crates.io
cargo install createrepo_rs

# Fedora / RHEL / CentOS (COPR)
dnf copr enable jamesarch/createrepo-rs && dnf install createrepo-rs

# Arch Linux (AUR)
yay -S createrepo-rs

# Debian / Ubuntu
curl -LO https://github.com/artifactx-rs/createrepo_rs/releases/download/v0.1.9/createrepo-rs_0.1.9_amd64.deb
dpkg -i createrepo-rs_0.1.9_amd64.deb

# RHEL / CentOS
curl -LO https://github.com/artifactx-rs/createrepo_rs/releases/download/v0.1.9/createrepo-rs-0.1.9-1.el9.x86_64.rpm
dnf install createrepo-rs-0.1.9-1.el9.x86_64.rpm

# Docker
docker run --rm -v /path/to/rpms:/data jamesarch/createrepo-rs /data
# or: ghcr.io/jamesarch/createrepo-rs /data
```

```bash
# Basic usage
createrepo_rs /path/to/rpms/

# Production example
createrepo_rs /srv/repo/ \
  --baseurl=https://mirrors.example.com/repo \
  --compress-type=zstd \
  --timeout=300
```

### CI/CD

```yaml
# GitHub Actions
- uses: docker://ghcr.io/jamesarch/createrepo-rs:0.1.9
  with:
    args: ./rpms --baseurl=https://repo.example.com --compress-type=zstd
```

```yaml
# GitLab CI
generate-repodata:
  image: ghcr.io/jamesarch/createrepo-rs:0.1.9
  script:
    - createrepo_rs ./rpms --compress-type=zstd
```

Container tags: `latest` · `0.1.9` · `0.1` · `sha-<short-sha>`

---

## Performance

Reproducible head-to-head ([`benchmark/`](benchmark/)) — one `docker run`, both
tools native in the same container, [hyperfine](https://github.com/sharkdp/hyperfine)
(5 runs). Numbers below: 10-core aarch64, 2000 synthetic packages.

### Wall-clock — full repodata generation

| Command | Time |
|---------|------|
| createrepo_c (default, 5 workers) | **67 ms** |
| createrepo_c (`--workers 10`) | 102 ms |
| createrepo_rs (all cores) | 82 ms |

createrepo_c is the **fastest** here — ~1.2× faster than createrepo_rs — and
raising its worker count doesn't help at this repo size. createrepo_c defaults to
5 workers and accepts `--workers 1–100`; there is no "5-thread cap." **Treat the
two as comparable on speed, with createrepo_c often slightly ahead.**

### What createrepo_rs actually wins

| Metric | createrepo_c | createrepo_rs |
|--------|--------------|---------------|
| Peak RSS (2000 pkgs) | ~84 MB | **20 MB — ~4× less** |
| Shared libraries | 53 | **5** |
| Output (pkgid set) | reference | **byte-identical** |
| Footprint | 72 KB binary + 53 `.so` | single 3.8 MB static binary, zero FFI |

The durable, universal wins are **memory, dependency footprint, and identical
output** — not raw speed.

### `--dump-manifest` — parallel package inventory scan

Reads only RPM signature header + name/version/arch. Skips files, deps, changelogs.
Replaces `rpm -K` + `rpm -qp` loops in CI pipelines.

| Method | Time | Workers |
|--------|------|---------|
| `rpm -K` loop (bash) | 10.5s | 1 |
| **`--dump-manifest`** | **0.078s** | 80 (auto) |

Output: JSON lines, one object per package, with signature detection.

```bash
$ createrepo_rs /srv/repo --dump-manifest
{"name":"zabbix-server","version":"7.2.0","release":"1.el9","arch":"x86_64","signed":true}
{"name":"zabbix-agent2","version":"7.2.0","release":"1.el9","arch":"x86_64","signed":true}
...
```

createrepo_rs parallelises across all cores by default (`--workers N` to pin a
count). The scan above replaces `rpm -K` + `rpm -qp` loops in CI pipelines.

---

## Architecture

### In-memory SQLite

SQLite builds in RAM, flushes once at finish:

```
parse RPMs (parallel) ──► insert_package() ──► RAM (single connection)
                                                      │
                                               VACUUM INTO repomd.sqlite
```

Traditional: one transaction + fsync per package. This: all inserts in RAM, one disk write.

### I/O resilience

Network mounts (NFS, CIFS, FUSE) can stall indefinitely. Mitigations:

- `--timeout=N` watchdog thread → forced exit on hang
- `recv_timeout(300s)` on result collection
- `send_timeout(30s)` on job submission
- `catch_unwind` per worker — one bad RPM can't crash the pool

### Source layout

```
createrepo_rs/
├── build.rs          # git hash + timestamp baked into --version
├── lib.rs            # library root + prelude re-exports
├── src/main.rs       # CLI orchestration, --dump-manifest
├── cli/mod.rs        # clap parser (53 params)
├── pool/mod.rs       # parallel worker pool (crossbeam + catch_unwind)
├── rpm/mod.rs        # RPM header parsing via `rpm` crate
├── types/mod.rs      # Package, Dependency, RepomdRecord
├── compression/      # gzip, bzip2, zstd, xz
├── db/mod.rs         # in-memory SQLite + VACUUM INTO
├── xml/              # repomd.xml generation + parse (for --update)
└── walk/mod.rs       # directory traversal with glob exclude
```

---

## Features

**Core**
- primary.xml, filelists.xml, other.xml, repomd.xml — dnf-compatible
- In-memory SQLite — `VACUUM INTO` flush at finish
- Multi-threaded RPM parsing (auto-detects CPU count)
- `--timeout=N` watchdog for stuck I/O
- `--dump-manifest` — parallel JSON-lines inventory with signature detection
- `--update` incremental mode with `Arc<Package>` cache
- Graceful Ctrl+C, `catch_unwind` panic recovery

**Dependency extraction**
- Provides / Requires / Conflicts / Obsoletes / Suggests / Enhances / Recommends / Supplements
- Full EVR (Epoch:Version-Release) parsing + flags (EQ, LT, GT, LE, GE)

**Compression:** gzip (default) · zstd · xz · bzip2

---

## Key CLI flags

| Flag | Description |
|------|-------------|
| `--workers=N` | Parallel threads (default: all CPUs) |
| `--timeout=N` | Watchdog timeout in seconds |
| `--dump-manifest` | JSON-lines inventory + signature detection |
| `--compress-type=zstd` | Compression algorithm |
| `--no-database` | Skip SQLite generation |
| `--update` | Incremental mode |
| `--retain-old-md-by-age=30d` | Auto-cleanup old metadata |
| `--compatibility` | Max compat mode (gzip + simple filenames) |

Full reference: `createrepo_rs --help` (53 params)

---

## Library usage

```toml
[dependencies]
createrepo_rs = "0.1"
```

```rust
use createrepo_rs::prelude::*;

// Parse an RPM
let mut reader = RpmReader::open(Path::new("pkg.rpm")).unwrap();
let pkg = reader.read_package().unwrap();
println!("{} {}-{}", pkg.name, pkg.version, pkg.release);

// Lightweight manifest scan (header only)
let entry = reader.read_manifest_entry().unwrap();
println!("{} {} signed={}", entry.name, entry.arch, entry.signed);
```

---

## Build

```bash
cargo build --release

# musl static binary (requires cargo-zigbuild + zig)
cargo zigbuild --release --target x86_64-unknown-linux-musl

# ARM64
cargo zigbuild --release --target aarch64-unknown-linux-musl
```

## Docker test

```bash
cd docker-test && docker compose up -d
# verifies dnf can consume the generated repodata
```

---

## Changelog

**v0.1.8** — parallel `--dump-manifest` · `read_manifest_entry()` header-only · 254 pkgs: 10.5s → 0.078s  
**v0.1.7** — `--dump-manifest` · `is_signed()` PGP/RSA/DSA detection · `ManifestEntry`  
**v0.1.6** — in-memory SQLite · single `VACUUM INTO` flush · removed per-struct boilerplate  
**v0.1.5** — `--timeout` watchdog · `recv_timeout` · `catch_unwind` · build info in `--version`  
**v0.1.4** — initial public release · 52 CLI params · dnf-compatible output  

---

## License

GPL-2.0-or-later — same as createrepo_c.

Built with [rpm-rs/rpm](https://github.com/rpm-rs/rpm) · [quick-xml](https://github.com/tafia/quick-xml) · [rusqlite](https://github.com/rusqlite/rusqlite) · [crossbeam](https://github.com/crossbeam-rs/crossbeam)  
EVR parsing reference: [dralley/rpmrepo_metadata](https://github.com/dralley/rpmrepo_metadata) ([@dralley](https://github.com/dralley), Red Hat)  
Original C implementation: [rpm-software-management/createrepo_c](https://github.com/rpm-software-management/createrepo_c)
