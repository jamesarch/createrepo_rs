//! # `createrepo_rs`
//!
//! Pure Rust implementation of `createrepo_c` — generates RPM repository metadata
//! (repodata) from a directory of RPM packages.
//!
//! ## Library Usage
//!
//! ```ignore
//! use std::path::Path;
//! use createrepo_rs::prelude::*;
//!
//! let mut reader = RpmReader::open(Path::new("my-package.rpm")).unwrap();
//! let pkg = reader.read_package().unwrap();
//! println!("{} {}-{}", pkg.name, pkg.version, pkg.release);
//! ```
//!
//! ## Modules
//!
//! - [`cli`] — CLI argument parsing (52+ parameters)
//! - [`types`] — Core types: Package, Dependency, `RepomdRecord`
//! - [`rpm`] — RPM header parsing
//! - [`compression`] — gzip, bzip2, zstd, xz
//! - [`xml`] — XML generation (primary, filelists, other, repomd)
//! - [`pool`] — Parallel worker pool
//! - [`db`] — `SQLite` database generation
//! - [`walk`] — Directory traversal

pub mod cli;
pub mod compression;
pub mod db;
pub mod pool;
pub mod rpm;
pub mod types;
pub mod walk;
pub mod xml;

/// Convenience re-exports for library users.
///
/// ```ignore
/// use createrepo_rs::prelude::*;
/// ```
pub mod prelude {
    pub use crate::compression::{
        bzip2_compress, bzip2_decompress, gzip_compress, gzip_decompress, xz_compress,
        xz_decompress, zstd_compress, zstd_decompress,
    };
    pub use crate::db::{DbError, RepomdDb};
    pub use crate::pool::{Job, ProcessingResult, WorkerPool};
    pub use crate::rpm::parse_dep_version;
    pub use crate::rpm::RpmError;
    pub use crate::rpm::RpmReader;
    pub use crate::types::{
        ChangelogEntry, ChecksumType, CompressionType, Dependency, Package, PackageFile,
    };
    pub use crate::walk::{DirectoryWalker, WalkError};
    pub use crate::xml::error::XmlError;
}
