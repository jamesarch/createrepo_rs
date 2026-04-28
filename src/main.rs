use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use createrepo_rs::cli::Cli;
use createrepo_rs::db::{self, RepomdDb};
use createrepo_rs::types::{
    CompressionType as TypesCompression, ContentTag, DistroTag, Package, RepoTag, Repomd,
    RepomdRecord,
};
use createrepo_rs::xml::dump;
use sha2::{Digest, Sha256, Sha384, Sha512};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

macro_rules! log {
    ($cli:expr, $level:expr, $($arg:tt)*) => {
        match $level {
            LogLevel::Error => eprintln!($($arg)*),
            LogLevel::Warning => {
                if !$cli.quiet { eprintln!($($arg)*); }
            }
            LogLevel::Normal => {
                if !$cli.quiet { eprintln!($($arg)*); }
            }
            LogLevel::Verbose => {
                if $cli.verbose { eprintln!($($arg)*); }
            }
        }
    };
}

enum LogLevel {
    Error,
    Warning,
    Normal,
    Verbose,
}

fn main() -> ExitCode {
    ctrlc::set_handler(move || {
        INTERRUPTED.store(true, Ordering::SeqCst);
        eprintln!("\nInterrupted... cleaning up");
    })
    .expect("Error setting Ctrl-C handler");

    let cli = Cli::parse_args();
    let repo_path = &cli.path;
    let output_dir = cli.outputdir.as_ref().unwrap_or(repo_path);

    log!(
        &cli,
        LogLevel::Normal,
        "createrepo_rs v{}",
        env!("CARGO_PKG_VERSION")
    );
    log!(
        &cli,
        LogLevel::Normal,
        "Repository path: {}",
        repo_path.display()
    );
    log!(
        &cli,
        LogLevel::Normal,
        "Output path: {}",
        output_dir.display()
    );

    // Cache of packages from existing repodata, keyed by location_href.
    // Populated only when --update is set and we can read the source repodata.
    let mut update_cache: HashMap<String, Arc<createrepo_rs::types::Package>> = HashMap::new();
    if cli.update {
        let cache_source = cli
            .update_md_path
            .clone()
            .unwrap_or_else(|| output_dir.join("repodata"));
        if cache_source.exists() {
            match createrepo_rs::xml::parse::load_cached_packages(&cache_source) {
                Ok(map) => {
                    log!(
                        &cli,
                        LogLevel::Normal,
                        "Update mode: loaded {} cached packages from {}",
                        map.len(),
                        cache_source.display()
                    );
                    update_cache = map;
                }
                Err(e) => {
                    log!(
                        &cli,
                        LogLevel::Warning,
                        "Warning: --update could not read existing repodata at {}: {} — all packages will be reprocessed",
                        cache_source.display(),
                        e
                    );
                }
            }
        } else {
            log!(
                &cli,
                LogLevel::Warning,
                "Warning: --update specified but no existing repodata found at {} — all packages will be reprocessed",
                cache_source.display()
            );
        }
    }

    if cli.simple_md_filenames {
        log!(
            &cli,
            LogLevel::Normal,
            "Note: Using simple-md-filenames (no checksums in metadata filenames)"
        );
    }

    if !repo_path.exists() || !repo_path.is_dir() {
        log!(
            &cli,
            LogLevel::Error,
            "Error: Directory '{}' does not exist or is not a directory",
            repo_path.display()
        );
        return ExitCode::from(1);
    }

    // Collect RPM files from various sources
    let mut rpm_files: Vec<PathBuf> = Vec::new();

    // Handle --pkglist (read package list from file)
    if let Some(ref pkglist_path) = cli.pkglist {
        match std::fs::read_to_string(pkglist_path) {
            Ok(content) => {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        let pkg_path = PathBuf::from(trimmed);
                        if pkg_path.exists() {
                            rpm_files.push(pkg_path);
                        } else {
                            // Try relative to repo_path
                            let full_path = repo_path.join(trimmed);
                            if full_path.exists() {
                                rpm_files.push(full_path);
                            } else {
                                log!(
                                    &cli,
                                    LogLevel::Warning,
                                    "Warning: Package from pkglist not found: {}",
                                    trimmed
                                );
                            }
                        }
                    }
                }
                log!(
                    &cli,
                    LogLevel::Normal,
                    "Read {} packages from pkglist",
                    rpm_files.len()
                );
            }
            Err(e) => {
                log!(&cli, LogLevel::Error, "Error reading pkglist file: {}", e);
                return ExitCode::from(1);
            }
        }
    }

    // Handle --includepkg (add specific packages)
    if !cli.includepkg.is_empty() {
        for pkg_spec in &cli.includepkg {
            let pkg_path = PathBuf::from(pkg_spec);
            if pkg_path.exists() {
                rpm_files.push(pkg_path);
            } else {
                // Try relative to repo_path
                let full_path = repo_path.join(pkg_spec);
                if full_path.exists() {
                    rpm_files.push(full_path);
                } else {
                    log!(
                        &cli,
                        LogLevel::Warning,
                        "Warning: Included package not found: {}",
                        pkg_spec
                    );
                }
            }
        }
        log!(
            &cli,
            LogLevel::Normal,
            "Added {} packages from --includepkg",
            cli.includepkg.len()
        );
    }

    // If no pkglist or includepkg, scan the directory
    if rpm_files.is_empty() {
        let exclude_patterns = cli.exclude_patterns();
        let walker = match createrepo_rs::walk::DirectoryWalker::new(repo_path) {
            Ok(w) => w,
            Err(e) => {
                log!(
                    &cli,
                    LogLevel::Error,
                    "Error creating directory walker: {}",
                    e
                );
                return ExitCode::from(1);
            }
        };

        let walker = if exclude_patterns.is_empty() {
            walker
        } else {
            match walker.exclude_patterns(exclude_patterns) {
                Ok(w) => w,
                Err(e) => {
                    log!(&cli, LogLevel::Error, "Error with exclude patterns: {}", e);
                    return ExitCode::from(1);
                }
            }
        };

        let walker = walker.skip_symlinks(cli.skip_symlinks);
        rpm_files = walker.collect();
    }

    let package_count = rpm_files.len();
    log!(&cli, LogLevel::Normal, "Found {} packages", package_count);

    if package_count == 0 {
        log!(
            &cli,
            LogLevel::Error,
            "Error: No packages found in directory"
        );
        return ExitCode::from(1);
    }

    // Use atomic update: write to temp dir, then rename
    let repodata_dir = output_dir.join("repodata");
    let repodata_old = output_dir.join("repodata.old");
    let repodata_tmp = if cli.local_sqlite {
        // For --local-sqlite, use system temp dir for NFS-safe SQLite creation
        std::env::temp_dir().join(".repodata")
    } else if let Some(ref cachedir) = cli.cachedir {
        cachedir.join("repodata.tmp")
    } else {
        output_dir.join("repodata.tmp")
    };

    if repodata_tmp.exists() {
        if let Err(e) = std::fs::remove_dir_all(&repodata_tmp) {
            log!(
                &cli,
                LogLevel::Error,
                "Error removing temp directory: {}",
                e
            );
            return ExitCode::from(1);
        }
    }

    if let Err(e) = std::fs::create_dir_all(&repodata_tmp) {
        log!(
            &cli,
            LogLevel::Error,
            "Error creating temp repodata directory: {}",
            e
        );
        return ExitCode::from(1);
    }

    // Initialize database in temp directory
    let db_path = repodata_tmp.join("repomd.sqlite");
    let db: Option<RepomdDb> = if cli.no_database {
        None
    } else {
        match db::db_init(&db_path) {
            Ok(db) => {
                log!(
                    &cli,
                    LogLevel::Normal,
                    "Database enabled: {}",
                    db_path.display()
                );
                Some(db)
            }
            Err(e) => {
                log!(
                    &cli,
                    LogLevel::Warning,
                    "Warning: Failed to initialize database: {}",
                    e
                );
                None
            }
        }
    };

    let num_workers = cli.workers();
    log!(
        &cli,
        LogLevel::Normal,
        "Processing packages with {} workers...",
        num_workers
    );

    let mut packages: Vec<Package> = Vec::new();
    let mut errors = 0;

    // Parse changelog_limit if provided
    let changelog_limit: Option<usize> = cli
        .changelog_limit
        .as_ref()
        .and_then(|s| s.parse::<usize>().ok());

    // In --update mode, partition discovered RPMs into cache hits (reuse stored
    // metadata) vs. misses (must re-read the RPM).
    let mut to_process: Vec<PathBuf> = Vec::with_capacity(rpm_files.len());
    let mut cache_hits = 0usize;
    if update_cache.is_empty() {
        to_process.extend(rpm_files.iter().cloned());
    } else {
        for rpm_path in &rpm_files {
            if let Some(cached) = lookup_cached(&update_cache, rpm_path, repo_path, cli.skip_stat) {
                // Try to get owned Package without cloning (if refcount is 1)
                let mut pkg = match Arc::try_unwrap(cached) {
                    Ok(pkg) => pkg,
                    Err(arc) => (*arc).clone(), // Clone only if multiple references exist
                };
                if let Some(limit) = changelog_limit {
                    pkg.changelogs.truncate(limit);
                }
                if let Some(ref db) = db {
                    if let Err(e) = db.insert_package(&pkg) {
                        log!(
                            &cli,
                            LogLevel::Warning,
                            "Warning: Failed to insert cached package {}: {}",
                            pkg.name,
                            e
                        );
                    }
                }
                packages.push(pkg);
                cache_hits += 1;
            } else {
                to_process.push(rpm_path.clone());
            }
        }
        log!(
            &cli,
            LogLevel::Normal,
            "Update mode: {} cached, {} to process",
            cache_hits,
            to_process.len()
        );
    }

    let rpm_files = to_process;

    if num_workers == 1 {
        for rpm_path in &rpm_files {
            if INTERRUPTED.load(Ordering::SeqCst) {
                log!(&cli, LogLevel::Normal, "\nInterrupted! Cleaning up...");
                let _ = std::fs::remove_dir_all(&repodata_tmp);
                return ExitCode::from(130);
            }
            log!(
                &cli,
                LogLevel::Verbose,
                "Processing: {}",
                rpm_path.display()
            );
            match createrepo_rs::rpm::RpmReader::open(rpm_path) {
                Ok(mut reader) => {
                    match reader.read_package() {
                        Ok(rpm_pkg) => {
                            let mut pkg = convert_package(rpm_pkg, &cli.basedir);
                            // Apply changelog limit
                            if let Some(limit) = changelog_limit {
                                pkg.changelogs.truncate(limit);
                            }
                            if let Some(ref db) = db {
                                if let Err(e) = db.insert_package(&pkg) {
                                    log!(
                                        &cli,
                                        LogLevel::Warning,
                                        "Warning: Failed to insert package {}: {}",
                                        pkg.name,
                                        e
                                    );
                                }
                            }

                            packages.push(pkg);
                        }
                        Err(e) => {
                            log!(
                                &cli,
                                LogLevel::Warning,
                                "Warning: Failed to read package {}: {}",
                                rpm_path.display(),
                                e
                            );
                            errors += 1;
                        }
                    }
                }
                Err(e) => {
                    log!(
                        &cli,
                        LogLevel::Warning,
                        "Warning: Failed to open package {}: {}",
                        rpm_path.display(),
                        e
                    );
                    errors += 1;
                }
            }
        }
    } else if rpm_files.is_empty() {
        // All packages came from the update cache; nothing for the pool to do.
    } else {
        let (mut pool, receiver) = createrepo_rs::pool::WorkerPool::new(num_workers);

        for rpm_path in &rpm_files {
            let _submit_ok =
                pool.submit(createrepo_rs::pool::Job::ProcessPackage(rpm_path.clone()));
        }

        let total_jobs = rpm_files.len();
        let mut collected = 0;

        while let Ok(result) = receiver.recv() {
            if INTERRUPTED.load(Ordering::SeqCst) {
                log!(&cli, LogLevel::Normal, "\nInterrupted! Cleaning up...");
                let _ = std::fs::remove_dir_all(&repodata_tmp);
                return ExitCode::from(130);
            }
            collected += 1;
            match result {
                createrepo_rs::pool::ProcessingResult::Success(_path, mut pkg) => {
                    // Apply changelog limit
                    if let Some(limit) = changelog_limit {
                        pkg.changelogs.truncate(limit);
                    }
                    if let Some(ref db) = db {
                        if let Err(e) = db.insert_package(&pkg) {
                            log!(
                                &cli,
                                LogLevel::Warning,
                                "Warning: Failed to insert package {}: {}",
                                pkg.name,
                                e
                            );
                        }
                    }
                    packages.push(pkg);
                }
                createrepo_rs::pool::ProcessingResult::Error(path, err) => {
                    log!(
                        &cli,
                        LogLevel::Warning,
                        "Warning: Failed to process {}: {}",
                        path.display(),
                        err
                    );
                    errors += 1;
                }
            }
            if collected == total_jobs {
                break;
            }
        }

        pool.close();
    }

    if let Some(db) = db {
        if let Err(e) = db.finish() {
            log!(
                &cli,
                LogLevel::Warning,
                "Warning: Failed to finalize database: {}",
                e
            );
        }
    }

    log!(
        &cli,
        LogLevel::Normal,
        "Successfully processed {} packages ({} errors)",
        packages.len(),
        errors
    );

    if let Some(ref read_pkgs_list_path) = cli.read_pkgs_list {
        let pkg_paths: Vec<String> = packages
            .iter()
            .filter_map(|p| p.location_href.clone())
            .collect();
        match std::fs::write(read_pkgs_list_path, pkg_paths.join("\n")) {
            Ok(()) => {
                log!(
                    &cli,
                    LogLevel::Normal,
                    "Wrote package list to {}",
                    read_pkgs_list_path.display()
                );
            }
            Err(e) => {
                log!(
                    &cli,
                    LogLevel::Warning,
                    "Warning: Failed to write package list: {}",
                    e
                );
            }
        }
    }

    if let Some(ref basedir) = cli.basedir {
        for pkg in &mut packages {
            if let Some(ref location) = pkg.location_href {
                let rpm_path = PathBuf::from(location);
                if let Ok(relative) = rpm_path.strip_prefix(basedir) {
                    pkg.location_href = Some(relative.to_string_lossy().into_owned());
                } else if let Some(parent) = basedir.parent() {
                    if let Ok(relative) = rpm_path.strip_prefix(parent) {
                        pkg.location_href = Some(relative.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    if let Some(cut) = cli.cut_dirs {
        for pkg in &mut packages {
            if let Some(ref href) = pkg.location_href {
                pkg.location_href = Some(cut_directory_components(href, cut));
            }
        }
    }

    if let Some(ref prefix) = cli.location_prefix {
        for pkg in &mut packages {
            if let Some(ref href) = pkg.location_href {
                pkg.location_href = Some(format!("{prefix}/{href}"));
            }
        }
    }

    if let Some(ref policy) = cli.duplicated_nevra {
        let mut seen: HashSet<String> = HashSet::new();
        let mut duplicates = Vec::new();
        for pkg in &packages {
            let nevra = format!("{}-{}-{}.{}", pkg.name, pkg.version, pkg.release, pkg.arch);
            if seen.contains(&nevra) {
                duplicates.push(nevra);
            } else {
                seen.insert(nevra);
            }
        }
        if !duplicates.is_empty() {
            match policy.as_str() {
                "error" => {
                    for dup in &duplicates {
                        log!(
                            &cli,
                            LogLevel::Error,
                            "Error: Duplicate NEVRA found: {}",
                            dup
                        );
                    }
                    return ExitCode::from(1);
                }
                "keep-latest" => {
                    log!(
                        &cli,
                        LogLevel::Warning,
                        "Warning: {} duplicate NEVRA(s) found (keeping first occurrence)",
                        duplicates.len()
                    );
                }
                _ => {
                    log!(
                        &cli,
                        LogLevel::Warning,
                        "Warning: {} duplicate NEVRA(s) found (keeping first, removing rest)",
                        duplicates.len()
                    );
                }
            }
        }
    }

    let base_compression = if cli.compatibility {
        TypesCompression::Gzip
    } else if cli.xz {
        TypesCompression::Xz
    } else {
        convert_compression(cli.compression())
    };
    let xml_compression = if let Some(ref general_type) = cli.general_compress_type {
        match general_type.parse::<createrepo_rs::cli::CompressionType>() {
            Ok(t) => convert_compression(t),
            Err(_) => base_compression,
        }
    } else {
        base_compression
    };
    let compression_suffix = match xml_compression {
        TypesCompression::Gzip => ".gz",
        TypesCompression::Bzip2 => ".bz2",
        TypesCompression::Xz => ".xz",
        TypesCompression::Zstd => ".zst",
        TypesCompression::None => "",
    };

    let unique_md_filenames =
        cli.unique_md_filenames && !cli.simple_md_filenames && !cli.compatibility;

    let pretty = cli.pretty && !cli.no_pretty;

    let override_timestamp: Option<i64> = if cli.set_timestamp_to_revision {
        cli.revision.as_ref().and_then(|r| r.parse::<i64>().ok())
    } else {
        None
    };

    let repomd_checksum_type = cli.repomd_checksum.as_deref().unwrap_or("sha256");

    // Generate primary.xml
    let primary_xml = match dump::primary::dump_primary_xml(&packages, pretty) {
        Ok(xml) => xml,
        Err(e) => {
            log!(&cli, LogLevel::Error, "Error generating primary.xml: {}", e);
            return ExitCode::from(1);
        }
    };
    let primary_checksum_uncompressed = compute_checksum(&primary_xml, repomd_checksum_type);
    let (primary_filename, primary_location) = if unique_md_filenames {
        let filename = format!("{repomd_checksum_type}-primary.xml{compression_suffix}");
        let location = format!("repodata/{filename}");
        (filename, location)
    } else {
        let filename = format!("primary.xml{compression_suffix}");
        let location = format!("repodata/{filename}");
        (filename, location)
    };
    let primary_path = repodata_tmp.join(&primary_filename);
    log!(&cli, LogLevel::Normal, "Writing primary.xml...");
    let primary_compressed = match write_compressed(&primary_xml, &primary_path, xml_compression) {
        Ok(data) => data,
        Err(e) => {
            log!(&cli, LogLevel::Error, "Error writing primary.xml: {}", e);
            return ExitCode::from(1);
        }
    };
    let primary_checksum = compute_checksum(&primary_compressed, repomd_checksum_type);
    let primary_size = Some(primary_compressed.len() as i64);
    let primary_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64);

    // Generate filelists.xml
    let filelists_xml = match dump::filelists::dump_filelists_xml(&packages, false, pretty) {
        Ok(xml) => xml,
        Err(e) => {
            log!(
                &cli,
                LogLevel::Error,
                "Error generating filelists.xml: {}",
                e
            );
            return ExitCode::from(1);
        }
    };
    let filelists_checksum_uncompressed = compute_checksum(&filelists_xml, repomd_checksum_type);
    let (filelists_filename, filelists_location) = if unique_md_filenames {
        let filename = format!("{repomd_checksum_type}-filelists.xml{compression_suffix}");
        let location = format!("repodata/{filename}");
        (filename, location)
    } else {
        let filename = format!("filelists.xml{compression_suffix}");
        let location = format!("repodata/{filename}");
        (filename, location)
    };
    let filelists_path = repodata_tmp.join(&filelists_filename);
    log!(&cli, LogLevel::Normal, "Writing filelists.xml...");
    let filelists_compressed =
        match write_compressed(&filelists_xml, &filelists_path, xml_compression) {
            Ok(data) => data,
            Err(e) => {
                log!(&cli, LogLevel::Error, "Error writing filelists.xml: {}", e);
                return ExitCode::from(1);
            }
        };
    let filelists_checksum = compute_checksum(&filelists_compressed, repomd_checksum_type);
    let filelists_size = Some(filelists_compressed.len() as i64);
    let filelists_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64);

    // Generate other.xml
    let other_xml = match dump::other::dump_other_xml(&packages, pretty) {
        Ok(xml) => xml,
        Err(e) => {
            log!(&cli, LogLevel::Error, "Error generating other.xml: {}", e);
            return ExitCode::from(1);
        }
    };
    let other_checksum_uncompressed = compute_checksum(&other_xml, repomd_checksum_type);
    let (other_filename, other_location) = if unique_md_filenames {
        let filename = format!("{repomd_checksum_type}-other.xml{compression_suffix}");
        let location = format!("repodata/{filename}");
        (filename, location)
    } else {
        let filename = format!("other.xml{compression_suffix}");
        let location = format!("repodata/{filename}");
        (filename, location)
    };
    let other_path = repodata_tmp.join(&other_filename);
    log!(&cli, LogLevel::Normal, "Writing other.xml...");
    let other_compressed = match write_compressed(&other_xml, &other_path, xml_compression) {
        Ok(data) => data,
        Err(e) => {
            log!(&cli, LogLevel::Error, "Error writing other.xml: {}", e);
            return ExitCode::from(1);
        }
    };
    let other_checksum = compute_checksum(&other_compressed, repomd_checksum_type);
    let other_size = Some(other_compressed.len() as i64);
    let other_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64);

    // Create repomd records
    let primary_record = RepomdRecord {
        record_type: "primary".to_string(),
        location: primary_location,
        checksum: Some(primary_checksum),
        timestamp: override_timestamp.or(primary_timestamp),
        size: primary_size,
        open_size: Some(primary_xml.len() as i64),
        open_checksum: Some(primary_checksum_uncompressed),
        checksum_type: Some(repomd_checksum_type.to_string()),
    };
    let filelists_record = RepomdRecord {
        record_type: "filelists".to_string(),
        location: filelists_location,
        checksum: Some(filelists_checksum),
        timestamp: override_timestamp.or(filelists_timestamp),
        size: filelists_size,
        open_size: Some(filelists_xml.len() as i64),
        open_checksum: Some(filelists_checksum_uncompressed),
        checksum_type: Some(repomd_checksum_type.to_string()),
    };
    let other_record = RepomdRecord {
        record_type: "other".to_string(),
        location: other_location,
        checksum: Some(other_checksum),
        timestamp: override_timestamp.or(other_timestamp),
        size: other_size,
        open_size: Some(other_xml.len() as i64),
        open_checksum: Some(other_checksum_uncompressed),
        checksum_type: Some(repomd_checksum_type.to_string()),
    };
    let distro_tags: Vec<DistroTag> = cli
        .distro_tags()
        .into_iter()
        .map(|(cpeid, tag)| DistroTag { cpeid, tag })
        .collect();

    let content_tags: Vec<ContentTag> = cli
        .content_tags()
        .into_iter()
        .map(|tag| ContentTag { tag })
        .collect();

    let repo_tags: Vec<RepoTag> = cli
        .repo_tags()
        .into_iter()
        .map(|tag| RepoTag { tag })
        .collect();

    // Handle filelists-ext if requested
    let mut filelists_ext_record: Option<RepomdRecord> = None;
    if cli.filelists_ext {
        let filelists_ext_xml = match dump::filelists::dump_filelists_xml(&packages, true, pretty) {
            Ok(xml) => xml,
            Err(e) => {
                log!(
                    &cli,
                    LogLevel::Error,
                    "Error generating filelists-ext.xml: {}",
                    e
                );
                return ExitCode::from(1);
            }
        };
        let (filelists_ext_filename, filelists_ext_location) = if unique_md_filenames {
            let filename = format!("{repomd_checksum_type}-filelists-ext.xml{compression_suffix}");
            let location = format!("repodata/{filename}");
            (filename, location)
        } else {
            let filename = format!("filelists-ext.xml{compression_suffix}");
            let location = format!("repodata/{filename}");
            (filename, location)
        };
        let filelists_ext_path = repodata_tmp.join(&filelists_ext_filename);
        log!(&cli, LogLevel::Normal, "Writing filelists-ext.xml...");
        let filelists_ext_compressed =
            match write_compressed(&filelists_ext_xml, &filelists_ext_path, xml_compression) {
                Ok(data) => data,
                Err(e) => {
                    log!(
                        &cli,
                        LogLevel::Error,
                        "Error writing filelists-ext.xml: {}",
                        e
                    );
                    return ExitCode::from(1);
                }
            };
        let filelists_ext_checksum =
            compute_checksum(&filelists_ext_compressed, repomd_checksum_type);
        let filelists_ext_size = filelists_ext_path.metadata().map(|m| m.len() as i64).ok();
        let filelists_ext_timestamp = filelists_ext_path.metadata().ok().and_then(|m| {
            m.modified().ok().and_then(|t| {
                t.duration_since(UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs() as i64)
            })
        });

        filelists_ext_record = Some(RepomdRecord {
            record_type: "filelists-ext".to_string(),
            location: filelists_ext_location,
            checksum: Some(filelists_ext_checksum),
            timestamp: override_timestamp.or(filelists_ext_timestamp),
            size: filelists_ext_size,
            open_size: Some(filelists_ext_xml.len() as i64),
            open_checksum: Some(compute_checksum(&filelists_ext_xml, repomd_checksum_type)),
            checksum_type: Some(repomd_checksum_type.to_string()),
        });
    }

    // Build records list including filelists-ext if generated
    let mut records = vec![primary_record, filelists_record, other_record];
    if let Some(ext_record) = filelists_ext_record {
        records.push(ext_record);
    }

    let repomd = Repomd {
        revision: cli.revision.clone(),
        records,
        distro_tags,
        content_tags,
        repo_tags,
    };

    let repomd_path = repodata_tmp.join("repomd.xml");
    log!(&cli, LogLevel::Normal, "Writing repomd.xml...");
    if let Err(e) = dump::repomd::dump_repomd(&repomd, &repomd_path, pretty) {
        log!(&cli, LogLevel::Error, "Error writing repomd.xml: {}", e);
        return ExitCode::from(1);
    }

    log!(&cli, LogLevel::Normal, "Repodata generation complete.");

    if repodata_dir.exists() {
        if let Err(e) = std::fs::rename(&repodata_dir, &repodata_old) {
            log!(&cli, LogLevel::Error, "Error renaming old repodata: {}", e);
            return ExitCode::from(1);
        }
    }

    if cli.local_sqlite {
        if let Err(e) = copy_dir_all(&repodata_tmp, &repodata_dir) {
            log!(
                &cli,
                LogLevel::Error,
                "Error copying repodata to output: {}",
                e
            );
            return ExitCode::from(1);
        }
        if let Err(e) = std::fs::remove_dir_all(&repodata_tmp) {
            log!(
                &cli,
                LogLevel::Warning,
                "Warning: Failed to remove temp repodata: {}",
                e
            );
        }
    } else if let Err(e) = std::fs::rename(&repodata_tmp, &repodata_dir) {
        log!(&cli, LogLevel::Error, "Error renaming temp repodata: {}", e);
        return ExitCode::from(1);
    }

    if repodata_old.exists() && !cli.retain_old_md {
        if let Err(e) = std::fs::remove_dir_all(&repodata_old) {
            log!(
                &cli,
                LogLevel::Warning,
                "Warning: Failed to remove old repodata: {}",
                e
            );
        }
    }

    if let Some(ref age_str) = cli.retain_old_md_by_age {
        if let Some(max_age) = parse_age_duration(age_str) {
            let cutoff = SystemTime::now() - max_age;
            if let Ok(entries) = std::fs::read_dir(&repodata_dir) {
                for entry in entries.filter_map(std::result::Result::ok) {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if modified < cutoff {
                                let path = entry.path();
                                let _ = std::fs::remove_file(&path);
                                let _ = std::fs::remove_dir_all(&path);
                            }
                        }
                    }
                }
            }
        }
    }

    ExitCode::SUCCESS
}

fn compute_checksum(data: &[u8], checksum_type: &str) -> String {
    match checksum_type {
        "sha384" => {
            let mut hasher = Sha384::new();
            hasher.update(data);
            format!("{:x}", hasher.finalize())
        }
        "sha512" => {
            let mut hasher = Sha512::new();
            hasher.update(data);
            format!("{:x}", hasher.finalize())
        }
        _ => {
            let mut hasher = Sha256::new();
            hasher.update(data);
            format!("{:x}", hasher.finalize())
        }
    }
}

fn write_compressed(
    content: &[u8],
    output: &Path,
    compression: TypesCompression,
) -> Result<Vec<u8>, std::io::Error> {
    use createrepo_rs::compression::{bzip2_compress, gzip_compress, xz_compress, zstd_compress};

    let data_to_write: Vec<u8> = match compression {
        TypesCompression::Gzip => gzip_compress(content, 6)?,
        TypesCompression::Bzip2 => bzip2_compress(content, 6)?,
        TypesCompression::Xz => xz_compress(content, 6)?,
        TypesCompression::Zstd => zstd_compress(content, 6)?,
        TypesCompression::None => content.to_vec(),
    };

    std::fs::write(output, &data_to_write)?;
    Ok(data_to_write)
}

const fn convert_compression(cli_comp: createrepo_rs::cli::CompressionType) -> TypesCompression {
    match cli_comp {
        createrepo_rs::cli::CompressionType::Gzip => TypesCompression::Gzip,
        createrepo_rs::cli::CompressionType::Bzip2 => TypesCompression::Bzip2,
        createrepo_rs::cli::CompressionType::Xz => TypesCompression::Xz,
        createrepo_rs::cli::CompressionType::Zstd => TypesCompression::Zstd,
        createrepo_rs::cli::CompressionType::None => TypesCompression::None,
    }
}

fn convert_package(rpm_pkg: createrepo_rs::rpm::Package, basedir: &Option<PathBuf>) -> Package {
    let location = rpm_pkg.location.clone();

    // Calculate location_href based on basedir (clone once for this computation)
    let location_href = if let Some(ref bd) = basedir {
        // If basedir is provided, compute relative path
        let rpm_path = PathBuf::from(&location);
        if let Ok(relative) = rpm_path.strip_prefix(bd) {
            relative.to_string_lossy().into_owned()
        } else {
            // If not relative to basedir, try relative to basedir's parent
            if let Ok(relative) = rpm_path.strip_prefix(bd.parent().unwrap_or(bd)) {
                relative.to_string_lossy().into_owned()
            } else {
                location.clone()
            }
        }
    } else {
        location.clone()
    };

    // Clone location once for filename, reuse original for location field
    let filename = location.clone();

    let convert_deps =
        |deps: Vec<createrepo_rs::rpm::DependencyInfo>| -> Vec<createrepo_rs::types::Dependency> {
            deps.into_iter()
                .map(|d| createrepo_rs::types::Dependency {
                    name: d.name,
                    epoch: d.epoch,
                    version: d.version,
                    release: d.release,
                    flags: d.flags,
                    pre: d.pre,
                })
                .collect()
        };

    let convert_changelogs = |entries: Vec<createrepo_rs::rpm::ChangelogInfo>| -> Vec<createrepo_rs::types::ChangelogEntry> {
        entries.into_iter().map(|e| {
            createrepo_rs::types::ChangelogEntry {
                author: e.author,
                date: e.date,
                content: e.content,
            }
        }).collect()
    };

    Package {
        pkgid: rpm_pkg.sha256.clone(),
        name: rpm_pkg.name,
        arch: rpm_pkg.arch,
        version: rpm_pkg.version,
        epoch: rpm_pkg.epoch.and_then(|e| e.parse().ok()),
        release: rpm_pkg.release,
        filename,
        location,
        checksum_type: createrepo_rs::types::ChecksumType::Sha256,
        checksum: rpm_pkg.sha256,
        source_pkg: rpm_pkg.sourcerpm.clone(),
        size_archive: rpm_pkg.file_size,
        size_installed: 0,
        size_package: rpm_pkg.size,
        time_file: rpm_pkg.time_file,
        time_build: rpm_pkg.time_build,
        summary: rpm_pkg.summary,
        description: rpm_pkg.description,
        packager: rpm_pkg.packager,
        url: rpm_pkg.url,
        license: rpm_pkg.license,
        vendor: rpm_pkg.vendor,
        group: rpm_pkg.group,
        buildhost: rpm_pkg.buildhost,
        sourcerpm: rpm_pkg.sourcerpm,
        requires: convert_deps(rpm_pkg.requires),
        provides: convert_deps(rpm_pkg.provides),
        conflicts: convert_deps(rpm_pkg.conflicts),
        obsoletes: convert_deps(rpm_pkg.obsoletes),
        suggests: convert_deps(rpm_pkg.suggests),
        enhances: convert_deps(rpm_pkg.enhances),
        recommends: convert_deps(rpm_pkg.recommends),
        supplements: convert_deps(rpm_pkg.supplements),
        files: rpm_pkg
            .files
            .into_iter()
            .map(|f| createrepo_rs::types::PackageFile {
                path: f.path,
                file_type: f.file_type.unwrap_or_default(),
                digest: f.digest,
                size: 0,
            })
            .collect(),
        changelogs: convert_changelogs(rpm_pkg.changelogs),
        location_href: Some(location_href),
        header_start: None,
        header_end: None,
    }
}

/// Look up a discovered RPM in the cached metadata loaded by `--update`.
///
/// Returns `Some(cached_pkg)` when the cache contains an entry for this RPM
/// and (unless `skip_stat` is set) the on-disk file size and mtime still
/// match the cached values. Returns `None` on any miss so the caller falls
/// back to fully re-reading the RPM.
fn lookup_cached(
    cache: &HashMap<String, Arc<createrepo_rs::types::Package>>,
    rpm_path: &Path,
    repo_path: &Path,
    skip_stat: bool,
) -> Option<Arc<createrepo_rs::types::Package>> {
    let mut keys: Vec<String> = Vec::new();
    if let Some(name) = rpm_path.file_name() {
        keys.push(name.to_string_lossy().into_owned());
    }
    if let Ok(rel) = rpm_path.strip_prefix(repo_path) {
        keys.push(rel.to_string_lossy().into_owned());
    }

    let cached = keys.iter().find_map(|k| cache.get(k))?;

    if skip_stat {
        return Some(cached.clone());
    }

    let meta = std::fs::metadata(rpm_path).ok()?;
    if meta.len() as i64 != cached.size_package {
        return None;
    }
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)?;
    if mtime != cached.time_file {
        return None;
    }
    Some(cached.clone())
}

fn cut_directory_components(path: &str, count: usize) -> String {
    let mut components: Vec<&str> = path.split('/').collect();
    for _ in 0..count {
        if !components.is_empty() {
            components.remove(0);
        }
    }
    components.join("/")
}

fn parse_age_duration(s: &str) -> Option<std::time::Duration> {
    if let Some(days) = s.strip_suffix('d') {
        days.parse::<u64>()
            .ok()
            .map(|d| std::time::Duration::from_secs(d * 86400))
    } else if let Some(hours) = s.strip_suffix('h') {
        hours
            .parse::<u64>()
            .ok()
            .map(|h| std::time::Duration::from_secs(h * 3600))
    } else if let Some(mins) = s.strip_suffix('m') {
        mins.parse::<u64>()
            .ok()
            .map(|m| std::time::Duration::from_secs(m * 60))
    } else {
        None
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else if ty.is_file() {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let result = compute_checksum(b"hello world", "sha256");
        assert_eq!(
            result,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_compute_sha512() {
        let result = compute_checksum(b"hello", "sha512");
        assert_eq!(result.len(), 128);
    }

    #[test]
    fn test_cut_directory_components() {
        assert_eq!(cut_directory_components("a/b/c/d.rpm", 2), "c/d.rpm");
        assert_eq!(cut_directory_components("a/b.rpm", 0), "a/b.rpm");
        assert_eq!(cut_directory_components("a", 5), "");
    }

    #[test]
    fn test_parse_age_duration() {
        assert!(parse_age_duration("30d").is_some());
        assert!(parse_age_duration("2h").is_some());
        assert!(parse_age_duration("5m").is_some());
        assert!(parse_age_duration("xyz").is_none());
    }
}
