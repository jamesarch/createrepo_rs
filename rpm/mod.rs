use rpm as rpm_crate;
use sha2::{Digest, Sha256};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpmError {
    #[error("Failed to open RPM: {0}")]
    OpenError(String),
    #[error("Failed to read RPM metadata: {0}")]
    MetadataError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Helper struct to represent file type from file entries
#[derive(Debug, Clone)]
pub struct FileTypeInfo {
    pub path: String,
    pub file_type: String,
    pub digest: Option<String>,
    pub size: i64,
}

/// RPM Dependency information extracted from the package
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    pub name: String,
    pub flags: String,
    pub epoch: Option<i32>,
    pub version: Option<String>,
    pub release: Option<String>,
    pub pre: bool,
}

/// Changelog entry from the RPM
#[derive(Debug, Clone)]
pub struct ChangelogInfo {
    pub author: String,
    pub date: i64,
    pub content: String,
}

pub struct PackageFile {
    pub path: String,
    pub file_type: Option<String>,
    pub digest: Option<String>,
}

pub struct Package {
    pub name: String,
    pub arch: String,
    pub version: String,
    pub epoch: Option<String>,
    pub release: String,
    pub location: String,
    pub time_file: i64,
    pub time_build: i64,
    pub size: i64,
    pub file_size: i64,
    pub sha256: String,
    pub files: Vec<PackageFile>,
    // New fields for full metadata extraction
    pub summary: Option<String>,
    pub description: Option<String>,
    pub packager: Option<String>,
    pub url: Option<String>,
    pub license: Option<String>,
    pub vendor: Option<String>,
    pub group: Option<String>,
    pub buildhost: Option<String>,
    pub sourcerpm: Option<String>,
    pub provides: Vec<DependencyInfo>,
    pub requires: Vec<DependencyInfo>,
    pub conflicts: Vec<DependencyInfo>,
    pub obsoletes: Vec<DependencyInfo>,
    pub suggests: Vec<DependencyInfo>,
    pub enhances: Vec<DependencyInfo>,
    pub recommends: Vec<DependencyInfo>,
    pub supplements: Vec<DependencyInfo>,
    pub changelogs: Vec<ChangelogInfo>,
}

pub struct RpmReader {
    path: std::path::PathBuf,
}

fn convert_dependency_flags(flags: rpm_crate::DependencyFlags) -> String {
    // C version: cr_flag_to_str(flags & 0xf)
    //  0 -> ""
    //  2 -> "LT" (LESS)
    //  4 -> "GT" (GREATER)
    //  8 -> "EQ" (EQUAL)
    // 10 -> "LE" (LESS | EQUAL)
    // 12 -> "GE" (GREATER | EQUAL)
    // default -> ""
    let bits = flags.bits() as u8 & 0x0f;
    match bits {
        2 => "LT",
        4 => "GT",
        8 => "EQ",
        10 => "LE",
        12 => "GE",
        _ => "",
    }
    .to_string()
}

fn convert_file_mode(mode: rpm_crate::FileMode) -> String {
    match mode {
        rpm_crate::FileMode::Dir { .. } => "dir".to_string(),
        rpm_crate::FileMode::Regular { .. } => "file".to_string(),
        rpm_crate::FileMode::SymbolicLink { .. } => "symlink".to_string(),
        rpm_crate::FileMode::Invalid { .. } => "unknown".to_string(),
        _ => "unknown".to_string(),
    }
}

impl RpmReader {
    pub fn open(path: &Path) -> Result<Self, RpmError> {
        if !path.exists() {
            return Err(RpmError::OpenError(format!(
                "File does not exist: {}",
                path.display()
            )));
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub fn read_package(&mut self) -> Result<Package, RpmError> {
        let metadata = rpm_crate::PackageMetadata::open(&self.path)
            .map_err(|e| RpmError::OpenError(e.to_string()))?;

        let name = metadata
            .get_name()
            .map_err(|e| RpmError::MetadataError(e.to_string()))?
            .to_string();
        let version = metadata
            .get_version()
            .map_err(|e| RpmError::MetadataError(e.to_string()))?
            .to_string();
        let release = metadata
            .get_release()
            .map_err(|e| RpmError::MetadataError(e.to_string()))?
            .to_string();
        let arch = metadata
            .get_arch()
            .map_err(|e| RpmError::MetadataError(e.to_string()))?
            .to_string();

        let epoch = metadata.get_epoch().map(|e| e.to_string()).ok();
        let time_build = metadata.get_build_time().map_or(0, |t| t as i64);

        let file_meta = std::fs::metadata(&self.path).ok();
        let file_size = file_meta.as_ref().map_or(0, |m| m.len() as i64);
        let time_file = file_meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(time_build, |d| d.as_secs() as i64);

        let location = self.path.file_name().map_or_else(
            || self.path.to_string_lossy().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );

        let file_entries = metadata.get_file_entries().unwrap_or_default();
        let files: Vec<PackageFile> = file_entries
            .into_iter()
            .map(|entry| PackageFile {
                path: entry.path.to_string_lossy().into_owned(),
                file_type: Some(convert_file_mode(entry.mode)),
                digest: entry.digest.map(|d| d.to_string()),
            })
            .collect();

        let sha256 = compute_sha256(&self.path)?;

        let summary = metadata
            .get_summary()
            .map(std::string::ToString::to_string)
            .ok();
        let description = metadata
            .get_description()
            .map(std::string::ToString::to_string)
            .ok();
        let packager = metadata
            .get_packager()
            .map(std::string::ToString::to_string)
            .ok();
        let url = metadata
            .get_url()
            .map(std::string::ToString::to_string)
            .ok();
        let license = metadata
            .get_license()
            .map(std::string::ToString::to_string)
            .ok();
        let vendor = metadata
            .get_vendor()
            .map(std::string::ToString::to_string)
            .ok();
        let group = metadata
            .get_group()
            .map(std::string::ToString::to_string)
            .ok();
        let buildhost = metadata
            .get_build_host()
            .map(std::string::ToString::to_string)
            .ok();
        let sourcerpm = metadata
            .get_source_rpm()
            .map(std::string::ToString::to_string)
            .ok();

        let provides = metadata
            .get_provides()
            .unwrap_or_default()
            .into_iter()
            .map(|dep| {
                let flags_str = convert_dependency_flags(dep.flags);
                let (version, release) = if dep.version.is_empty() {
                    (None, None)
                } else {
                    // C format: ver="3.4.5" rel="6" - split by last '-'
                    match dep.version.rfind('-') {
                        Some(pos) => (
                            Some(dep.version[..pos].to_string()),
                            Some(dep.version[pos + 1..].to_string()),
                        ),
                        None => (Some(dep.version.clone()), None),
                    }
                };
                DependencyInfo {
                    name: dep.name,
                    flags: flags_str,
                    epoch: None,
                    version,
                    release,
                    pre: false,
                }
            })
            .collect();

        let requires = metadata
            .get_requires()
            .unwrap_or_default()
            .into_iter()
            .map(|dep| {
                let flags_str = convert_dependency_flags(dep.flags);
                let (version, release) = if dep.version.is_empty() {
                    (None, None)
                } else {
                    match dep.version.rfind('-') {
                        Some(pos) => (
                            Some(dep.version[..pos].to_string()),
                            Some(dep.version[pos + 1..].to_string()),
                        ),
                        None => (Some(dep.version.clone()), None),
                    }
                };
                DependencyInfo {
                    name: dep.name,
                    flags: flags_str,
                    epoch: None,
                    version,
                    release,
                    pre: false,
                }
            })
            .collect();

        let conflicts = metadata
            .get_conflicts()
            .unwrap_or_default()
            .into_iter()
            .map(|dep| {
                let flags_str = convert_dependency_flags(dep.flags);
                let (version, release) = if dep.version.is_empty() {
                    (None, None)
                } else {
                    match dep.version.rfind('-') {
                        Some(pos) => (
                            Some(dep.version[..pos].to_string()),
                            Some(dep.version[pos + 1..].to_string()),
                        ),
                        None => (Some(dep.version.clone()), None),
                    }
                };
                DependencyInfo {
                    name: dep.name,
                    flags: flags_str,
                    epoch: None,
                    version,
                    release,
                    pre: false,
                }
            })
            .collect();

        let obsoletes = metadata
            .get_obsoletes()
            .unwrap_or_default()
            .into_iter()
            .map(|dep| {
                let flags_str = convert_dependency_flags(dep.flags);
                let (version, release) = if dep.version.is_empty() {
                    (None, None)
                } else {
                    match dep.version.rfind('-') {
                        Some(pos) => (
                            Some(dep.version[..pos].to_string()),
                            Some(dep.version[pos + 1..].to_string()),
                        ),
                        None => (Some(dep.version.clone()), None),
                    }
                };
                DependencyInfo {
                    name: dep.name,
                    flags: flags_str,
                    epoch: None,
                    version,
                    release,
                    pre: false,
                }
            })
            .collect();

        let suggests = metadata
            .get_suggests()
            .unwrap_or_default()
            .into_iter()
            .map(|dep| {
                let flags_str = convert_dependency_flags(dep.flags);
                let (version, release) = if dep.version.is_empty() {
                    (None, None)
                } else {
                    match dep.version.rfind('-') {
                        Some(pos) => (
                            Some(dep.version[..pos].to_string()),
                            Some(dep.version[pos + 1..].to_string()),
                        ),
                        None => (Some(dep.version.clone()), None),
                    }
                };
                DependencyInfo {
                    name: dep.name,
                    flags: flags_str,
                    epoch: None,
                    version,
                    release,
                    pre: false,
                }
            })
            .collect();

        let enhances = metadata
            .get_enhances()
            .unwrap_or_default()
            .into_iter()
            .map(|dep| {
                let flags_str = convert_dependency_flags(dep.flags);
                let (version, release) = if dep.version.is_empty() {
                    (None, None)
                } else {
                    match dep.version.rfind('-') {
                        Some(pos) => (
                            Some(dep.version[..pos].to_string()),
                            Some(dep.version[pos + 1..].to_string()),
                        ),
                        None => (Some(dep.version.clone()), None),
                    }
                };
                DependencyInfo {
                    name: dep.name,
                    flags: flags_str,
                    epoch: None,
                    version,
                    release,
                    pre: false,
                }
            })
            .collect();

        let recommends = metadata
            .get_recommends()
            .unwrap_or_default()
            .into_iter()
            .map(|dep| {
                let flags_str = convert_dependency_flags(dep.flags);
                let (version, release) = if dep.version.is_empty() {
                    (None, None)
                } else {
                    match dep.version.rfind('-') {
                        Some(pos) => (
                            Some(dep.version[..pos].to_string()),
                            Some(dep.version[pos + 1..].to_string()),
                        ),
                        None => (Some(dep.version.clone()), None),
                    }
                };
                DependencyInfo {
                    name: dep.name,
                    flags: flags_str,
                    epoch: None,
                    version,
                    release,
                    pre: false,
                }
            })
            .collect();

        let supplements = metadata
            .get_supplements()
            .unwrap_or_default()
            .into_iter()
            .map(|dep| {
                let flags_str = convert_dependency_flags(dep.flags);
                let (version, release) = if dep.version.is_empty() {
                    (None, None)
                } else {
                    match dep.version.rfind('-') {
                        Some(pos) => (
                            Some(dep.version[..pos].to_string()),
                            Some(dep.version[pos + 1..].to_string()),
                        ),
                        None => (Some(dep.version.clone()), None),
                    }
                };
                DependencyInfo {
                    name: dep.name,
                    flags: flags_str,
                    epoch: None,
                    version,
                    release,
                    pre: false,
                }
            })
            .collect();

        let changelogs = metadata
            .get_changelog_entries()
            .unwrap_or_default()
            .into_iter()
            .map(|entry| ChangelogInfo {
                author: entry.name,
                date: entry.timestamp as i64,
                content: entry.description,
            })
            .collect();

        Ok(Package {
            name,
            arch,
            version,
            epoch,
            release,
            location,
            time_file,
            time_build,
            size: file_size,
            file_size,
            sha256,
            files,
            summary,
            description,
            packager,
            url,
            license,
            vendor,
            group,
            buildhost,
            sourcerpm,
            provides,
            requires,
            conflicts,
            obsoletes,
            suggests,
            enhances,
            recommends,
            supplements,
            changelogs,
        })
    }

    pub fn checksum(&self) -> Result<String, RpmError> {
        compute_sha256(&self.path)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn compute_sha256(path: &Path) -> Result<String, RpmError> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = std::io::Read::read(&mut file, &mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    let result = hasher.finalize();
    Ok(format!("{result:x}"))
}
