//! CLI argument parsing for createrepo_c binary.
//!
//! This module provides command-line argument parsing using clap.

use std::path::PathBuf;
use clap::{Parser, ValueHint};

/// Compression type for metadata files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum CompressionType {
    #[default]
    Gzip,
    Bzip2,
    Xz,
    Zstd,
    None,
}


impl std::fmt::Display for CompressionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionType::Gzip => write!(f, "gz"),
            CompressionType::Bzip2 => write!(f, "bz2"),
            CompressionType::Xz => write!(f, "xz"),
            CompressionType::Zstd => write!(f, "zst"),
            CompressionType::None => write!(f, "none"),
        }
    }
}

impl std::str::FromStr for CompressionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gz" | "gzip" => Ok(CompressionType::Gzip),
            "bz2" | "bzip2" => Ok(CompressionType::Bzip2),
            "xz" => Ok(CompressionType::Xz),
            "zst" | "zstd" => Ok(CompressionType::Zstd),
            "none" => Ok(CompressionType::None),
            _ => Err(format!("Unknown compression type: {}", s)),
        }
    }
}

/// CLI arguments for createrepo_c.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to the repository directory.
    #[arg(value_hint = ValueHint::DirPath)]
    pub path: PathBuf,

    /// Run quietly.
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Run verbosely.
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Output directory for repodata.
    #[arg(short, long, value_hint = ValueHint::DirPath)]
    pub outputdir: Option<PathBuf>,

    /// Base URL for the repository.
    #[arg(short = 'b', long)]
    pub baseurl: Option<String>,

    /// Basedir for path to directories.
    #[arg(long, value_hint = ValueHint::DirPath)]
    pub basedir: Option<PathBuf>,

    /// Comma-separated list of glob patterns to exclude.
    #[arg(short = 'x', long)]
    pub excludes: Option<String>,

    /// Skip symlinks when processing.
    #[arg(long)]
    pub skip_symlinks: bool,

    /// Make sure all xml generated is formatted (default).
    #[arg(short = 'p', long, default_value = "true")]
    pub pretty: bool,

    /// No extra indentation in generated xml.
    #[arg(long)]
    pub no_pretty: bool,

    /// Number of worker threads.
    #[arg(short = 'w', long)]
    pub workers: Option<usize>,

    /// Compression type for metadata files.
    #[arg(long, default_value = "gz")]
    pub compress_type: String,

    /// Compression level.
    #[arg(long, default_value = "6")]
    pub compress_level: Option<i32>,

    /// Include checksums in filenames.
    #[arg(long, default_value = "true")]
    pub unique_md_filenames: bool,

    /// Don't generate SQLite databases.
    #[arg(long)]
    pub no_database: bool,

    /// Don't update existing repodata (only create if missing).
    #[arg(long)]
    pub update: bool,

    /// Skip closing the transaction early.
    #[arg(long)]
    pub aggressive: bool,

    /// Set the repository revision.
    #[arg(long)]
    pub revision: Option<String>,

    /// Set the checksum type.
    #[arg(long, default_value = "sha256")]
    pub checksum: String,

    /// Distribution tag (format: "tag" or "cpeid,tag").
    #[arg(long)]
    pub distro: Vec<String>,

    /// Content tag.
    #[arg(long)]
    pub content_tag: Vec<String>,

    /// Repository tag.
    #[arg(long)]
    pub repo_tag: Vec<String>,

    /// Path to groupfile to include in metadata.
    #[arg(short = 'g', long)]
    pub groupfile: Option<PathBuf>,

    /// Generate deltarpms and delta metadata.
    #[arg(long)]
    pub deltarpm: bool,

    /// Create filelists-ext metadata with file hashes.
    #[arg(long)]
    pub filelists_ext: bool,

    /// Paths to look for older pkgs to delta against.
    #[arg(long, value_hint = ValueHint::DirPath)]
    pub oldpackagedirs: Vec<PathBuf>,

    /// The number of older versions to make deltas against.
    #[arg(long)]
    pub num_deltas: Option<usize>,

    /// Max size of an rpm that to run deltarpm against (in bytes).
    #[arg(long)]
    pub max_delta_rpm_size: Option<u64>,

    /// Existing metadata from this path are loaded and reused (works only with --update).
    #[arg(long, value_hint = ValueHint::DirPath)]
    pub update_md_path: Option<PathBuf>,

    /// Skip the stat() call on a --update.
    #[arg(long)]
    pub skip_stat: bool,

    /// Run in split media mode.
    #[arg(long)]
    pub split: bool,

    /// Specify a text file with list of files to include.
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub pkglist: Option<PathBuf>,

    /// Specify pkgs to include on the command line.
    #[arg(long)]
    pub includepkg: Vec<String>,

    /// Only import the last N changelog entries.
    #[arg(long)]
    pub changelog_limit: Option<String>,

    /// Do NOT include checksum in filenames (opposite of --unique-md-filenames).
    #[arg(long)]
    pub simple_md_filenames: bool,

    /// Keep old repodata copies.
    #[arg(long)]
    pub retain_old_md: bool,

    /// Set timestamp to --revision value.
    #[arg(long)]
    pub set_timestamp_to_revision: bool,

    /// Output paths to pkgs actually read (with --update).
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub read_pkgs_list: Option<PathBuf>,

    /// Shortcut for --compress-type xz.
    #[arg(long)]
    pub xz: bool,

    /// Compression type for all XML files (separate from --compress-type).
    #[arg(long)]
    pub general_compress_type: Option<String>,

    /// Generate zchunk files.
    #[arg(long)]
    pub zck: bool,

    /// Directory with zchunk compression dictionaries.
    #[arg(long, value_hint = ValueHint::DirPath)]
    pub zck_dict_dir: Option<PathBuf>,

    /// Keep additional metadata during update (default).
    #[arg(long)]
    pub keep_all_metadata: bool,

    /// Discard additional metadata during update.
    #[arg(long)]
    pub discard_additional_metadata: bool,

    /// Enforce maximal compatibility with classical createrepo.
    #[arg(long)]
    pub compatibility: bool,

    /// Remove old repodata older than specified period.
    #[arg(long)]
    pub retain_old_md_by_age: Option<String>,

    /// Set path to cache dir.
    #[arg(long, value_hint = ValueHint::DirPath)]
    pub cachedir: Option<PathBuf>,

    /// Gen sqlite DBs locally (for NFS mounts).
    #[arg(long)]
    pub local_sqlite: bool,

    /// Ignore NUM of directory components in location_href.
    #[arg(long)]
    pub cut_dirs: Option<usize>,

    /// Append prefix before location_href.
    #[arg(long)]
    pub location_prefix: Option<String>,

    /// Checksum type for repomd.xml.
    #[arg(long)]
    pub repomd_checksum: Option<String>,

    /// Exit with 2 if errors (deprecated, on by default).
    #[arg(long)]
    pub error_exit_val: bool,

    /// Read list from old metadata directory.
    #[arg(long)]
    pub recycle_pkglist: bool,

    /// What to do about duplicates.
    #[arg(long)]
    pub duplicated_nevra: Option<String>,
}

impl Cli {
    /// Parse command line arguments.
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Get the exclude patterns as a vector of strings.
    pub fn exclude_patterns(&self) -> Vec<String> {
        self.excludes
            .as_ref()
            .map(|s| s.split(',').map(String::from).collect())
            .unwrap_or_default()
    }

    /// Get the compression type.
    pub fn compression(&self) -> CompressionType {
        self.compress_type.parse().unwrap_or(CompressionType::Gzip)
    }

    /// Get the workers count or default to number of CPUs.
    pub fn workers(&self) -> usize {
        self.workers.unwrap_or(num_cpus::get())
    }

    pub fn distro_tags(&self) -> Vec<(Option<String>, String)> {
        self.distro
            .iter()
            .map(|s| {
                if let Some((cpeid, tag)) = s.split_once(',') {
                    (Some(cpeid.to_string()), tag.to_string())
                } else {
                    (None, s.clone())
                }
            })
            .collect()
    }

    pub fn content_tags(&self) -> Vec<String> {
        self.content_tag.clone()
    }

    pub fn repo_tags(&self) -> Vec<String> {
        self.repo_tag.clone()
    }

    /// Returns true if any feature that requires zchunk compression is enabled.
    pub fn needs_zchunk(&self) -> bool {
        self.zck || self.zck_dict_dir.is_some()
    }

    /// Returns true if simple_md_filenames is set (no checksum in filenames).
    pub fn is_simple_md_filenames(&self) -> bool {
        self.simple_md_filenames
    }

    /// Returns true if any metadata retention policy is set.
    pub fn has_retain_policy(&self) -> bool {
        self.retain_old_md || self.retain_old_md_by_age.is_some()
    }

    /// Returns true if additional metadata handling is explicit.
    pub fn has_additional_metadata_policy(&self) -> bool {
        self.keep_all_metadata || self.discard_additional_metadata
    }
}