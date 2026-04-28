# createrepo_rs 🦀

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-GPL--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/jamesarch/createrepo_rs/actions/workflows/ci.yml/badge.svg)](https://github.com/jamesarch/createrepo_rs/actions)

**100% pure Rust implementation of `createrepo_c`** — generates RPM repository metadata (repodata).

Drop-in replacement for the C version with **identical output, zero FFI, and 3.5MB static binary.**

## 🎯 Why createrepo_rs?

| | createrepo_c (C) | createrepo_rs (Rust) |
|---|---|---|
| Output compatibility | ✅ | ✅ 100% byte-compatible |
| CLI arguments | 55 | 52/55 *(3 hard: split, zchunk)* |
| Dependencies | librpm, libxml2, glib2, zchunk... | **zero** FFI — pure Rust crates |
| Binary size | ~200KB + shared libs | **3.5MB static** (musl) |
| Memory safety | ❌ manual malloc/free | ✅ borrow checker |
| Cross-compile | painful | `cargo zigbuild` |
| `dnf` compatible | ✅ | ✅ **verified** |

## 🚀 Quick Start

```bash
# Install
cargo install --git https://github.com/jamesarch/createrepo_rs createrepo_rs

# Or clone and build
git clone https://github.com/jamesarch/createrepo_rs
cd createrepo_rs
cargo build --release

# Generate metadata for a directory of RPMs
createrepo_rs /path/to/rpms/

# Or with options
createrepo_rs /path/to/rpms/ \
  --compress-type=zstd \
  --no-database \
  --workers=8 \
  --simple-md-filenames
```

## 📦 Features

### Core
- ✅ primary.xml, filelists.xml, other.xml generation
- ✅ repomd.xml with correct checksums
- ✅ SQLite database generation (`--no-database` to disable)
- ✅ Multi-threaded RPM parsing (configurable `--workers`)
- ✅ Changlog extraction
- ✅ Provides/Requires/Conflicts/Obsoletes/Suggests/Recommends
- ✅ Weak dependencies (Supplements/Enhances)
- ✅ File type detection (dir/symlink/regular)
- ✅ Graceful Ctrl+C handling
- ✅ `--update` incremental mode

### Compression
- ✅ gzip (default)
- ✅ bzip2
- ✅ zstd
- ✅ xz
- ✅ Configurable level

### CLI (52/55 params — 100% of commonly used)

```bash
createrepo_rs --help   # Full list
```

Notable:
| Flag | Description |
|------|-------------|
| `--workers=N` | Parallel RPM parsing threads |
| `--compress-type=zstd` | Compression algorithm |
| `--checksum=sha512` | Hash algorithm for metadata |
| `--revision=12345` | Custom repository revision |
| `--baseurl=https://...` | Base URL for repository |
| `--simple-md-filenames` | Clean filenames (no hash prefix) |
| `--unique-md-filenames` | Hash-prefixed filenames (default) |
| `--location-prefix=subdir/` | Prefix before location href |
| `--cut-dirs=2` | Strip N directory components |
| `--repomd-checksum=sha512` | Checksum type for repomd.xml |
| `--general-compress-type=xz` | Separate compression for XML files |
| `--duplicated-nevra=error` | Error on duplicate packages |
| `--retain-old-md-by-age=30d` | Auto-cleanup old metadata |
| `--compatibility` | Max compatibility mode (gzip + simple filenames) |
| `-q / -v` | Quiet / Verbose output |
| `--no-pretty` | Compact XML without indentation |

## 📊 Performance

Tested on macOS (M1 Pro, 16GB RAM), createrepo_c run in Docker (Fedora 40).
All tests use `--compress-type=zstd --no-database` unless noted.

### Full Generation Time (lower is better)

| RPMs | createrepo_rs (4w) | createrepo_c (4w) | Speedup |
|------|--------------------|--------------------|---------|
| 10 | 0.01s | 0.10s | **10x** |
| 100 | 0.01s | 0.11s | **11x** |
| 500 | 0.03s | 0.12s | **4x** |
| 1000 | 0.05s | 0.20s¹ | **4x¹** |

> ¹ Estimated: createrepo_c ~O(n) for RPM parsing

### Worker Scaling (500 RPMs)

| Workers | createrepo_rs | createrepo_c |
|---------|---------------|--------------|
| 1 | 0.05s | 0.25s |
| 4 | 0.03s | 0.12s |
| 8 | 0.03s | — |

### Compression Comparison (500 RPMs, 4 workers)

| Algorithm | Time | Output Size | Best For |
|-----------|------|-------------|----------|
| **zstd** | 0.03s | 20KB | Speed + size (default) |
| gzip | 0.03s | 40KB | Max compatibility |
| xz | 0.11s | 20KB | Smallest output |
| bzip2 | 0.14s | 24KB | Legacy support |

### Incremental Update (`--update --skip-stat`)

| RPMs | createrepo_rs | Notes |
|------|---------------|-------|
| 100 | 0.01s | Cache hit, skips re-parsing |
| 500 | 0.03s | Only processes changed packages |
| 1000 | 0.05s | Near-constant time for unchanged repos |

### Optimizations Applied (v0.1.4)

| Optimization | Impact |
|--------------|--------|
| LTO + opt-level=3 + panic=abort | ~5-10% runtime, ~7% binary size |
| Multi-worker deadlock fix | Enables 2+ workers (was broken) |
| Arc\<Package\> update cache | Avoids full clone on `--update` hits |
| SQLite batch transactions | 10-50x filelists insert speed |
| Redundant stat() eliminated | -2 syscalls per XML file |
| XML Vec::with_capacity | Avoids reallocation during dump |
| SHA buffer 8KB→64KB | Fewer read() syscalls |
| From\<DependencyInfo\> impl | -114 lines duplicated code |

Primary XML generation is byte-identical to the createrepo_c C version.

## 🐳 Docker Test

```bash
cd docker-test
docker compose up -d
docker logs -f createrepo_client
```

Output:
```
✅✅✅ Success! createrepo_rs generated metadata recognized and downloaded by dnf!
```

## 🔨 Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Linux static binary (musl) — requires Zig
# Install: https://ziglang.org/download/
# Then: cargo install cargo-zigbuild
cargo zigbuild --release --target x86_64-unknown-linux-musl

# Cross-compile for ARM
cargo zigbuild --release --target aarch64-unknown-linux-musl
```

## 🏗️ Architecture

```
src/
├── main.rs          # Entry point, CLI handling, orchestration
├── lib.rs           # Library root
├── pool/mod.rs      # Parallel worker pool
├── cli/mod.rs       # Clap argument parser (55 params)
├── types/mod.rs     # Core types: Package, Dependency, RepomdRecord
├── rpm/mod.rs       # RPM header parsing via `rpm` crate
├── compression/     # gzip, bzip2, zstd, xz
├── xml/
│   ├── dump/        # XML generation (primary, filelists, other, repomd)
│   └── mod.rs
└── walk/            # Directory traversal
```

## 📝 License

GPL-2.0-or-later — same as createrepo_c.

## 🙏 Credits

Original C implementation: [rpm-software-management/createrepo_c](https://github.com/rpm-software-management/createrepo_c)

Built with:
- [rpm-rs/rpm](https://github.com/rpm-rs/rpm) — Pure Rust RPM parser
- [dralley/rpmrepo_metadata](https://github.com/dralley/rpmrepo_metadata) — EVR parsing reference (by [@dralley](https://github.com/dralley) at Red Hat)
- [quick-xml](https://github.com/tafia/quick-xml) — Fast XML writer
- [rusqlite](https://github.com/rusqlite/rusqlite) — SQLite bindings
