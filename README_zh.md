# createrepo_rs 🦀

[![Rust](https://img.shields.io/badge/Rust-1.76%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-GPL--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/jamesarch/createrepo_rs/actions/workflows/ci.yml/badge.svg)](https://github.com/jamesarch/createrepo_rs/actions)
[![crates.io](https://img.shields.io/crates/v/createrepo_rs.svg)](https://crates.io/crates/createrepo_rs)

**100% 纯 Rust 实现的 `createrepo_c`** — 生成 RPM 仓库元数据（repodata）。

C 版本的直接替代品，**dnf 兼容输出、零 FFI、3.5MB 静态二进制。**

[English](README.md)

> ##### 已在 Zabbix 7.2 生产环境验证（Debian 13，80 核）。

## 🎯 为什么选择 createrepo_rs？

| | createrepo_c (C) | createrepo_rs (Rust) |
|---|---|---|
| 输出兼容性 | ✅ | ✅ dnf 兼容 |
| CLI 参数 | 55 | 53 |
| 依赖 | librpm, libxml2, glib2, zchunk... | **零** FFI — 纯 Rust 生态 |
| 二进制大小 | ~200KB + 动态库 | **3.5MB 静态链接** (musl) |
| 内存安全 | ❌ 手动 malloc/free | ✅ 借⽤检查器 |
| 交叉编译 | 困难 | `cargo zigbuild` 一键搞定 |
| 线程安全 | ⚠️ 容易出现竞态 | ✅ 全局 `Send + Sync` |
| I/O 超时保护 | ❌ | ✅ `--timeout` 看门狗 |
| `dnf` 兼容 | ✅ | ✅ **已验证** |
| 包清单扫描 | ❌ | ✅ `--dump-manifest` 0.08s |
| 签名检测 | ❌ 需借助 `rpm -K` | ✅ 内置 |
| 内存级 SQLite | ❌ | ✅ 内存构建 + VACUUM INTO 落盘 |

## 🚀 快速开始

```bash
# 从 crates.io 安装
cargo install createrepo_rs

# 或从 git 安装
cargo install --git https://github.com/jamesarch/createrepo_rs createrepo_rs

# 为 RPM 目录生成元数据
createrepo_rs /path/to/rpms/

# 生产环境示例（Zabbix 镜像站）
createrepo_rs /srv/repo/ \
  --baseurl=https://mirrors.example.com/repo \
  --compress-type=zstd \
  --timeout=300 \
  --verbose
```

## 📊 性能

基于 Zabbix 生产服务器实测（Debian 13，80 核，254 个 RPM 包）：

### 完整生成（254 个包，zstd 压缩）

| 工具 | 耗时 | CPU | 输出 |
|------|------|-----|--------|
| createrepo_c | 2.15s | 454% | 232K |
| **createrepo_rs** | **1.87s** | 1724% | 14M¹ |

> ¹ 14M 包含 SQLite 数据库。使用 `--no-database` 后输出约 200K。

### 增量更新（热缓存，`--update --skip-stat`）

| 工具 | 耗时 | CPU | 备注 |
|------|------|-----|-------|
| createrepo_c | 0.85s | 214% | 基于 mtime |
| createrepo_rs | 1.4s | 100% | 含 SQLite 重建 |

### `--dump-manifest` — 包清单扫描

| 范围 | 方法 | 耗时 | 线程数 |
|-------|--------|------|---------|
| 254 包 | `rpm -K` 循环 (bash) | 10s+ | 1 |
| 254 包 | **`--dump-manifest`** | **0.078s** | 80 |
| 5 包 | 受限范围扫描 | 0.2s | 80 |

### 80 核环境下的线程缩放

| 线程数 | 完整生成 | 清单扫描 |
|---------|---------------|---------------|
| 1 | 10.5s | 10.5s |
| 4 | 3.6s | 0.5s |
| **80** (自动) | **1.87s** | **0.078s** ✓ |

> createrepo_rs 默认自动使用全部 CPU。C 版本最多仅支持 5 个线程。

## 📦 功能特性

### 核心功能
- ✅ primary.xml、filelists.xml、other.xml 生成 — dnf 兼容
- ✅ repomd.xml 支持多哈希校验（sha256/sha512）
- ✅ 内存级 SQLite — 内存速度写入，完成时一次性落盘
- ✅ `--no-database` 完全跳过 SQLite 生成
- ✅ 多线程 RPM 解析（自动检测 CPU 数量）
- ✅ `--timeout=N` I/O 卡死看门狗（网络挂载、坏盘等）
- ✅ `--dump-manifest` — 并行 JSON-lines 包清单 + 签名检测
- ✅ 优雅的 Ctrl+C 处理，线程 panic 恢复（`catch_unwind`）
- ✅ `--update` 增量模式，使用 Arc\<Package\> 缓存

### 依赖提取（从 RPM 头）
- ✅ Provides / Requires / Conflicts / Obsoletes
- ✅ Suggests / Enhances / Recommends / Supplements
- ✅ 完整 EVR（Epoch:Version-Release）解析
- ✅ 依赖标志（EQ、LT、GT、LE、GE）

### 元数据覆盖
- ✅ Summary、description、packager、URL、license、vendor、group
- ✅ Build host、source RPM、build time、file time
- ✅ Changelog 提取，支持 `--changelog-limit`
- ✅ 文件类型检测（目录、符号链接、常规文件）
- ✅ 文件摘要（从 RPM header 读取）

### 压缩
- ✅ gzip（默认）— 最大兼容性
- ✅ zstd — 最佳速度/体积平衡
- ✅ xz — 最小体积
- ✅ bzip2 — 兼容旧系统
- ✅ XML 与元数据文件独立压缩类型（`--general-compress-type`）

### CLI（53 个参数）

```bash
createrepo_rs --help
```

主要参数：

| 参数 | 说明 |
|------|-------------|
| `--workers=N` | 并行线程数（默认：全部 CPU） |
| `--timeout=N` | 全局超时时间，秒（I/O 卡死保护） |
| `--dump-manifest` | JSON-lines 包清单 + 签名检查 |
| `--compress-type=zstd` | 压缩算法 |
| `--no-database` | 跳过 SQLite 生成 |
| `--checksum=sha512` | 元数据哈希算法 |
| `--revision=12345` | 自定义仓库版本号 |
| `--baseurl=https://...` | 仓库基础 URL |
| `--simple-md-filenames` | 简洁文件名，不含哈希前缀 |
| `--cut-dirs=2` | 移除 location_href 中的 N 层目录 |
| `--update` | 增量模式（跳过未变更的包） |
| `--retain-old-md-by-age=30d` | 自动清理旧元数据 |
| `-q / -v` | 静默 / 详细输出 |
| `--compatibility` | 最大兼容模式（gzip + 简洁文件名） |
| `--update-md-path=PATH` | 从自定义路径加载已有元数据 |
| `--duplicated-nevra=error` | 重复包时报错 |
| `--location-prefix=PREFIX` | location href 前缀 |

完整 53 参数解析器见 `cli/mod.rs`（52 个选项 + PATH）。

## 🧠 架构亮点

### 内存级 SQLite（v0.1.6）

SQLite 数据库完全在内存中构建，仅完成时落盘：

```
insert_package() ──► RAM (RefCell<Connection>) ──► VACUUM INTO repomd.sqlite
                           ▲                              ▲
                    每次 INSERT O(1)           finish() 时 O(1)
```

传统做法中每个包独立事务 + 磁盘 fsync。我们的做法：单一内存连接，所有表共用一个数据库，结束时原子性刷入磁盘。

### `--dump-manifest`（v0.1.7）

二进制内置的轻量级并行包清单扫描器：

- 仅读取 RPM 签名头 + 名称/版本/架构 — **跳过文件列表、依赖、changelog**
- 通过 crossbeam + `std::thread::scope` 并行，自动适配全部 CPU
- JSON lines 格式 — 每行一个包对象，可直接被 Python/Shell 消费
- 签名检测通过 RPM 签名头标签（PGP/RSA/DSA）

80 核上扫描 254 个包仅需 **0.078 秒**。可替代 CI/CD 中的 `rpm -K` + `rpm -qp` 循环。

### I/O 韧性（v0.1.5）

仓库目录在 NFS、CIFS、FUSE 等网络挂载上可能无限期卡死。应对措施：

- `--timeout=N` 生成看门狗线程，超时强制退出
- 结果收集使用 `recv_timeout(300s)` 代替阻塞 `recv()`
- 任务提交使用 `send_timeout(30s)` 检测卡死线程
- 线程 panic 通过 `catch_unwind` 捕获 — 单个坏包不会拖垮整个线程池

### 构建信息嵌入（v0.1.5）

每个二进制自带溯源信息：

```
$ createrepo_rs --version
createrepo_rs 0.1.8
revision  a5afd72
built     2026-05-27T15:59:41
```

Git 哈希 + 编译时间戳通过 `build.rs` 在构建时注入。无运行时依赖，无需配置文件。

## 💪 性能哲学

| 原则 | 实现 |
|-----------|---------------|
| **尽量零拷贝** | `&str` 优先于 `String`，`Arc<Package>` 用于更新缓存 |
| **批量 I/O** | SQLite 全部在内存写入，单次磁盘刷出 |
| **默认并行** | `num_cpus::get()` 线程数，无需手动调优 |
| **轻量读取** | `read_manifest_entry()` 跳过 80% RPM 头解析 |
| **Release 配置** | LTO + opt-level=3 + codegen-units=1 + panic=abort |
| **HEAP 优先** | 64KB 读缓冲区放在堆上而非栈上 |

## 🔨 构建

```bash
# 开发构建
cargo build

# Release 构建（优化）
cargo build --release

# Linux 静态二进制（musl）— 需安装 Zig
# 安装：brew install zig (macOS) / apt install zig (Linux)
# 然后：cargo install cargo-zigbuild
cargo zigbuild --release --target x86_64-unknown-linux-musl

# ARM64 交叉编译
cargo zigbuild --release --target aarch64-unknown-linux-musl

# 交叉编译 Linux（macOS ARM → Linux x86_64）
cargo zigbuild --release --target x86_64-unknown-linux-gnu
```

## 🏗️ 项目结构

```
createrepo_rs/
├── build.rs          # 构建信息嵌入（git hash、时间戳）
├── lib.rs            # 库入口 + prelude 重导出
├── Cargo.toml        # v0.1.8, Rust 1.76+
├── src/main.rs       # 二进制入口，CLI 编排，--dump-manifest
├── cli/mod.rs        # Clap 参数解析器（53 个参数）
├── pool/mod.rs       # 并行线程池（crossbeam + catch_unwind）
├── rpm/mod.rs        # RPM 头解析（基于 `rpm` crate）
├── types/mod.rs      # 核心类型：Package、Dependency、RepomdRecord
├── compression/      # gzip、bzip2、zstd、xz
├── db/mod.rs         # 内存 SQLite + VACUUM INTO 落盘
├── xml/
│   ├── error.rs      # XML 错误类型
│   ├── mod.rs        # XML 辅助函数
│   ├── parse.rs      # XML 解析（用于 --update 缓存）
│   └── repomd.rs     # repomd.xml 生成
└── walk/mod.rs       # 目录遍历 + glob 排除
```

## 🐳 Docker 测试

```bash
cd docker-test
docker compose up -d
docker logs -f createrepo_client
```

输出：
```
✅✅✅ Success! createrepo_rs generated metadata recognized and downloaded by dnf!
```

## 📚 作为库使用

```toml
[dependencies]
createrepo_rs = "0.1"
```

```rust
use std::path::Path;
use createrepo_rs::prelude::*;

// 解析 RPM 包
let mut reader = RpmReader::open(Path::new("my-package.rpm")).unwrap();
let pkg = reader.read_package().unwrap();
println!("{} {}-{}", pkg.name, pkg.version, pkg.release);

// 检查签名（轻量 — 仅读取头部）
println!("signed: {}", reader.is_signed());

// 轻量清单扫描
let entry = reader.read_manifest_entry().unwrap();
println!("{} {} {} signed={}", entry.name, entry.version, entry.arch, entry.signed);

// 解析 EVR 依赖版本
let (epoch, ver, rel) = parse_dep_version("0:1.2.3-4");
assert_eq!(epoch, Some(0));
assert_eq!(ver.as_deref(), Some("1.2.3"));
assert_eq!(rel.as_deref(), Some("4"));

// 批量处理的线程池
let (pool, receiver) = WorkerPool::new(8);
pool.submit(Job::ProcessPackage(PathBuf::from("pkg.rpm")));
```

[`prelude`] 模块重导出所有常用类型：
- 压缩：`gzip_compress`、`gzip_decompress`、`zstd_compress`、`zstd_decompress` 等
- 类型：`Package`、`Dependency`、`ChecksumType`、`CompressionType`、`ChangelogEntry`
- RPM：`RpmReader`、`parse_dep_version`、`ManifestEntry`
- DB：`RepomdDb`、`DbError`
- 线程池：`WorkerPool`、`Job`、`ProcessingResult`
- XML：`XmlError`
- 遍历：`DirectoryWalker`、`WalkError`

## 📝 更新日志

### v0.1.8 — 并行清单
- 并行 `--dump-manifest`（`std::thread::scope` + crossbeam）
- 轻量 `read_manifest_entry()` — 仅读头部，跳过文件/依赖/changelog
- 254 个包：10.5s → 0.078s（130 倍加速）

### v0.1.7 — 清单 + 签名检测
- `--dump-manifest` 参数 — JSON-lines 包清单输出
- `is_signed()` — 通过 RPM 签名头检测 PGP/RSA/DSA
- `ManifestEntry` 结构体：name、version、arch、signed

### v0.1.6 — 内存级 SQLite
- SQLite 完全在内存中构建，完成时通过 `VACUUM INTO` 一次性落盘
- 三张表共用一个连接（primary、filelists、other）
- 移除约 120 行冗余样板代码

### v0.1.5 — I/O 超时保护
- `--timeout` 看门狗线程强制退出
- `recv_timeout(300s)` 收集结果（原阻塞 `recv()`）
- `send_timeout(30s)` 提交任务（原阻塞 `send()`）
- `catch_unwind` 线程 panic 恢复
- 构建信息嵌入（`--version` 显示 git hash + 时间戳）
- SQLite `transaction()` 自动回滚

### v0.1.4 — 首次公开发布
- 52/55 CLI 参数
- `dnf` 兼容（Docker 集成测试验证）
- dnf 兼容的 XML 输出

## 📝 许可证

GPL-2.0-or-later — 与 createrepo_c 相同。

## 🙏 致谢

原始 C 实现：[rpm-software-management/createrepo_c](https://github.com/rpm-software-management/createrepo_c)

构建依赖：
- [rpm-rs/rpm](https://github.com/rpm-rs/rpm) — 纯 Rust RPM 解析器
- [dralley/rpmrepo_metadata](https://github.com/dralley/rpmrepo_metadata) — EVR 解析参考（作者 [@dralley](https://github.com/dralley)，Red Hat）
- [quick-xml](https://github.com/tafia/quick-xml) — 快速 XML 写入
- [rusqlite](https://github.com/rusqlite/rusqlite) — SQLite 绑定
- [crossbeam](https://github.com/crossbeam-rs/crossbeam) — MPMC 通道
