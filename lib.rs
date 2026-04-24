//! # createrepo_rs
//!
//! Pure Rust implementation of `createrepo_c` — generates RPM repository metadata
//! (repodata) from a directory of RPM packages.
//!
//! ## Quick Start
//!
//! ```rust
//! use createrepo_rs::types::Package;
//!
//! // Generate repodata from a CLI
//! // createrepo_rs /path/to/rpms/
//! ```
//!
//! ## Modules
//!
//! - [`cli`] — CLI argument parsing (52+ parameters)
//! - [`types`] — Core types: Package, Dependency, RepomdRecord
//! - [`rpm`] — RPM header parsing
//! - [`compression`] — gzip, bzip2, zstd, xz
//! - [`xml`] — XML generation (primary, filelists, other, repomd)
//! - [`pool`] — Parallel worker pool
//! - [`db`] — SQLite database generation
//! - [`walk`] — Directory traversal

pub mod cli;
pub mod compression;
pub mod db;
pub mod pool;
pub mod rpm;
pub mod types;
pub mod walk;
pub mod xml;