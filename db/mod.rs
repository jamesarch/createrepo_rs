use crate::types::Package;
use rusqlite::{params, Connection};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Combined database handle for all repomd `SQLite` tables.
/// This provides a unified interface for inserting packages into
/// the primary, filelists, and other tables.
pub struct RepomdDb {
    primary: PrimaryDb,
    filelists: FilelistsDb,
    other: OtherDb,
}

impl RepomdDb {
    /// Initialize all three databases.
    pub fn new(path: &Path) -> Result<Self, DbError> {
        Ok(Self {
            primary: PrimaryDb::new(path)?,
            filelists: FilelistsDb::new(path)?,
            other: OtherDb::new(path)?,
        })
    }

    /// Insert a package into all three database tables.
    /// Returns the pkgKey on success.
    pub fn insert_package(&self, pkg: &Package) -> Result<i64, DbError> {
        let pkg_key = self.primary.insert_package(pkg)?;
        self.filelists.insert_package(pkg, pkg_key)?;
        self.other.insert_package(pkg, pkg_key)?;
        Ok(pkg_key)
    }

    /// Finalize all databases (runs ANALYZE, etc).
    pub fn finish(self) -> Result<(), DbError> {
        self.primary.finish()?;
        self.filelists.finish()?;
        self.other.finish()?;
        Ok(())
    }
}

/// Insert a package into all repomd `SQLite` tables.
/// This is a convenience function that creates temporary database
/// connections, inserts the package, and closes the connections.
///
/// Note: For bulk inserts, use `RepomdDb` directly for better performance.
pub fn db_insert_packages(db_path: &Path, packages: &[Package]) -> Result<(), DbError> {
    let db = RepomdDb::new(db_path)?;
    for pkg in packages {
        if let Err(e) = db.insert_package(pkg) {
            eprintln!("Warning: Failed to insert package {}: {}", pkg.name, e);
        }
    }
    db.finish()?;
    Ok(())
}

/// Initialize the database at the given path.
/// Returns a `RepomdDb` handle ready for package insertion.
pub fn db_init(path: &Path) -> Result<RepomdDb, DbError> {
    RepomdDb::new(path)
}

/// Finalize and close the database.
pub fn db_fini(db: RepomdDb) -> Result<(), DbError> {
    db.finish()
}

pub struct PrimaryDb {
    conn: Connection,
}

impl PrimaryDb {
    pub fn new(path: &Path) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA synchronous = OFF;
             PRAGMA journal_mode = OFF;
             PRAGMA cache_size = 10000;
             PRAGMA temp_store = MEMORY;
             CREATE TABLE IF NOT EXISTS \"primary\" (
                 pkgKey INTEGER PRIMARY KEY,
                 pkgId TEXT NOT NULL,
                 name TEXT NOT NULL,
                 arch TEXT,
                 version TEXT,
                 epoch INTEGER,
                 release TEXT,
                 summary TEXT,
                 description TEXT,
                 url TEXT,
                 license TEXT,
                 time_file INTEGER,
                 time_build INTEGER,
                 rpm_license TEXT,
                 rpm_vendor TEXT,
                 rpm_group TEXT,
                 rpm_buildhost TEXT,
                 rpm_sourcerpm TEXT,
                 rpm_header_start INTEGER,
                 rpm_header_end INTEGER,
                 rpm_packager TEXT,
                 size_archive INTEGER,
                 size_installed INTEGER,
                 size_package INTEGER,
                 location_href TEXT,
                 location_base TEXT,
                 checksum TEXT,
                 checksum_type TEXT
             );",
        )?;
        Ok(Self { conn })
    }

    pub fn insert_package(&self, pkg: &Package) -> Result<i64, DbError> {
        self.conn.execute_batch("BEGIN TRANSACTION;")?;
        let result = (|| {
            self.conn.execute(
                "INSERT INTO \"primary\" (pkgId, name, arch, version, epoch, release,
                    summary, description, url, license, time_file, time_build,
                    rpm_license, rpm_vendor, rpm_group, rpm_buildhost, rpm_sourcerpm,
                    rpm_header_start, rpm_header_end, rpm_packager, size_archive,
                    size_installed, size_package, location_href, location_base,
                    checksum, checksum_type)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                        ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)",
                params![
                    pkg.pkgid,
                    pkg.name,
                    pkg.arch,
                    pkg.version,
                    pkg.epoch,
                    pkg.release,
                    pkg.summary,
                    pkg.description,
                    pkg.url,
                    pkg.license,
                    pkg.time_file,
                    pkg.time_build,
                    pkg.license,
                    pkg.vendor,
                    "",
                    pkg.buildhost,
                    pkg.sourcerpm,
                    pkg.header_start,
                    pkg.header_end,
                    pkg.vendor,
                    pkg.size_archive,
                    pkg.size_installed,
                    pkg.size_package,
                    pkg.location_href,
                    "",
                    pkg.checksum,
                    "sha256",
                ],
            )?;
            Ok(self.conn.last_insert_rowid())
        })();
        self.conn.execute_batch("COMMIT;")?;
        result
    }

    pub fn finish(self) -> Result<(), DbError> {
        self.conn.execute_batch("ANALYZE;")?;
        Ok(())
    }
}

pub struct FilelistsDb {
    conn: Connection,
}

impl FilelistsDb {
    pub fn new(path: &Path) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA synchronous = OFF;
             PRAGMA journal_mode = OFF;
             PRAGMA cache_size = 10000;
             PRAGMA temp_store = MEMORY;
CREATE TABLE IF NOT EXISTS \"filelist\" (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  pkgKey INTEGER NOT NULL,
                 pkgId TEXT NOT NULL,
                 name TEXT NOT NULL,
                 arch TEXT,
                 version TEXT,
                 epoch INTEGER,
                 release TEXT,
                 filename TEXT NOT NULL,
                 type TEXT
             );
             CREATE INDEX IF NOT EXISTS filelist_idx ON filelist(pkgId);",
        )?;
        Ok(Self { conn })
    }

    pub fn insert_package(&self, pkg: &Package, pkg_key: i64) -> Result<(), DbError> {
        self.conn.execute_batch("BEGIN TRANSACTION;")?;
        let result = (|| {
            for file in &pkg.files {
                self.conn.execute(
                    "INSERT INTO \"filelist\" (pkgKey, pkgId, name, arch, version, epoch,
                        release, filename, type)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        pkg_key,
                        pkg.pkgid,
                        pkg.name,
                        pkg.arch,
                        pkg.version,
                        pkg.epoch,
                        pkg.release,
                        file.path,
                        file.file_type,
                    ],
                )?;
            }
            Ok(())
        })();
        self.conn.execute_batch("COMMIT;")?;
        result
    }

    pub fn finish(self) -> Result<(), DbError> {
        self.conn.execute_batch("ANALYZE;")?;
        Ok(())
    }
}

pub struct OtherDb {
    conn: Connection,
}

impl OtherDb {
    pub fn new(path: &Path) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA synchronous = OFF;
             PRAGMA journal_mode = OFF;
             PRAGMA cache_size = 10000;
             PRAGMA temp_store = MEMORY;
CREATE TABLE IF NOT EXISTS \"other\" (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  pkgKey INTEGER NOT NULL,
                 pkgId TEXT NOT NULL,
                 name TEXT NOT NULL,
                 arch TEXT,
                 version TEXT,
                 epoch INTEGER,
                 release TEXT,
                 filename TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS other_idx ON other(pkgId);",
        )?;
        Ok(Self { conn })
    }

    pub fn insert_package(&self, pkg: &Package, pkg_key: i64) -> Result<(), DbError> {
        self.conn.execute_batch("BEGIN TRANSACTION;")?;
        let result = (|| {
            self.conn.execute(
                "INSERT INTO \"other\" (pkgKey, pkgId, name, arch, version, epoch, release, filename)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    pkg_key,
                    pkg.pkgid,
                    pkg.name,
                    pkg.arch,
                    pkg.version,
                    pkg.epoch,
                    pkg.release,
                    pkg.location_href,
                ],
            )?;
            Ok(())
        })();
        self.conn.execute_batch("COMMIT;")?;
        result
    }

    pub fn finish(self) -> Result<(), DbError> {
        self.conn.execute_batch("ANALYZE;")?;
        Ok(())
    }
}
