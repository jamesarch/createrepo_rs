//! Thread pool for parallel package processing.
//!
//! This module provides a worker pool for processing jobs in parallel using
//! std::thread and crossbeam-channel.

use crate::rpm::RpmReader;
use crate::types::{ChecksumType, Package as TypesPackage, PackageFile as TypesPackageFile};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

/// Job types that can be submitted to the worker pool.
#[derive(Debug, Clone)]
pub enum Job {
    /// Process a package at the given path.
    ProcessPackage(PathBuf),
    /// Placeholder for other job types.
    Other(String),
}

impl Job {
    /// Returns the path associated with this job, if any.
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            Job::ProcessPackage(p) => Some(p),
            Job::Other(_) => None,
        }
    }
}

/// Result of processing a job.
#[derive(Debug, Clone)]
pub enum ProcessingResult {
    /// Successful processing of a package with parsed metadata.
    Success(PathBuf, TypesPackage),
    /// Error during processing.
    Error(PathBuf, String),
}

/// Internal message passed between pool and workers.
enum WorkerMessage {
    Job(Job),
    Stop,
}

/// Worker pool for parallel job processing.
///
/// Submits jobs to worker threads that process them in parallel.
pub struct WorkerPool {
    workers: Vec<thread::JoinHandle<()>>,
    job_sender: Sender<WorkerMessage>,
    shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    _result_sender: Sender<ProcessingResult>,
}

impl WorkerPool {
    /// Creates a new worker pool with the specified number of workers.
    pub fn new(num_workers: usize) -> (Self, Receiver<ProcessingResult>) {
        let (job_sender, job_receiver) = bounded::<WorkerMessage>(num_workers * 2);
        let (result_sender, result_receiver) = bounded::<ProcessingResult>(num_workers * 2);
        let shutdown_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut workers = Vec::with_capacity(num_workers);

        for id in 0..num_workers {
            let receiver = job_receiver.clone();
            let result_sender = result_sender.clone();
            let shutdown_flag = shutdown_flag.clone();

            let worker = thread::Builder::new()
                .name(format!("worker-{}", id))
                .spawn(move || {
                    Self::worker_loop(id, receiver, result_sender, shutdown_flag);
                })
                .expect("failed to spawn worker thread");

            workers.push(worker);
        }

        let pool = WorkerPool {
            workers,
            job_sender,
            shutdown_flag,
            _result_sender: result_sender,
        };

        (pool, result_receiver)
    }

    /// Returns a receiver for processing results.
    pub fn results(&self) -> &Receiver<ProcessingResult> {
        panic!("Use the Receiver returned by WorkerPool::new()")
    }

    /// Submits a job to the pool for processing.
    ///
    /// Returns true if the job was submitted successfully, false if the pool
    /// has been shut down.
    pub fn submit(&self, job: Job) -> bool {
        self.job_sender
            .send(WorkerMessage::Job(job))
            .map(|_| true)
            .is_ok()
    }

    /// Signals all workers to stop processing.
    pub fn shutdown(&self) {
        self.shutdown_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        // Send stop signals to all workers
        for _ in &self.workers {
            let _ = self.job_sender.send(WorkerMessage::Stop);
        }
    }

    /// Waits for all workers to complete and consumes the pool.
    pub fn join(self) {
        // First signal shutdown
        self.shutdown();

        // Send stop signals for each worker
        for _ in &self.workers {
            let _ = self.job_sender.send(WorkerMessage::Stop);
        }

        // Wait for all workers to finish
        for worker in self.workers {
            let _ = worker.join();
        }
    }

    /// The main loop for each worker thread.
    fn worker_loop(
        _id: usize,
        receiver: Receiver<WorkerMessage>,
        result_sender: Sender<ProcessingResult>,
        shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        while !shutdown_flag.load(std::sync::atomic::Ordering::SeqCst) {
            // Try to receive a job with a short timeout to check shutdown flag
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(WorkerMessage::Job(job)) => {
                    let result = Self::process_job(job);
                    let _ = result_sender.send(result);
                }
                Ok(WorkerMessage::Stop) => {
                    // Received stop signal
                    break;
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // No message available, check shutdown flag and continue
                    continue;
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    // Channel disconnected, worker should exit
                    break;
                }
            }
        }
    }

    /// Processes a single job and returns the result.
    fn process_job(job: Job) -> ProcessingResult {
        match job {
            Job::ProcessPackage(path) => {
                let mut reader = match RpmReader::open(&path) {
                    Ok(r) => r,
                    Err(e) => {
                        return ProcessingResult::Error(path, format!("Failed to open RPM: {}", e));
                    }
                };

                let rpm_pkg = match reader.read_package() {
                    Ok(p) => p,
                    Err(e) => {
                        return ProcessingResult::Error(path, format!("Failed to read package: {}", e));
                    }
                };

                let pkg = convert_package(rpm_pkg);
                ProcessingResult::Success(path, pkg)
            }
            Job::Other(_msg) => {
                ProcessingResult::Success(PathBuf::new(), TypesPackage::default())
            }
        }
    }
}

/// Converts an rpm::Package to a types::Package.
fn convert_package(rpm_pkg: crate::rpm::Package) -> TypesPackage {
    let location = rpm_pkg.location.clone();
    TypesPackage {
        pkgid: rpm_pkg.sha256.clone(),
        name: rpm_pkg.name,
        arch: rpm_pkg.arch,
        version: rpm_pkg.version,
        epoch: rpm_pkg.epoch.and_then(|e| e.parse().ok()),
        release: rpm_pkg.release,
        filename: location.clone(),
        location: location.clone(),
        checksum_type: ChecksumType::Sha256,
        checksum: rpm_pkg.sha256,
        source_pkg: None,
        size_archive: rpm_pkg.file_size,
        size_installed: 0,
        size_package: rpm_pkg.size,
        time_file: rpm_pkg.time_file,
        time_build: rpm_pkg.time_build,
        summary: None,
        description: None,
        url: None,
        license: None,
        vendor: None,
        buildhost: None,
        sourcerpm: None,
        requires: Vec::new(),
        provides: Vec::new(),
        conflicts: Vec::new(),
        obsoletes: Vec::new(),
        suggests: Vec::new(),
        enhances: Vec::new(),
        recommends: Vec::new(),
        files: rpm_pkg.files.into_iter().map(|f| {
            TypesPackageFile {
                path: f.path,
                file_type: f.file_type.unwrap_or_default(),
                digest: f.digest,
                size: 0,
            }
        }).collect(),
        changelogs: Vec::new(),
        location_href: Some(location),
        header_start: None,
        header_end: None,
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Instant;

    #[test]
    fn test_worker_pool_basic() {
        let (pool, results) = WorkerPool::new(2);

        // Submit 4 jobs
        for i in 0..4 {
            let path = PathBuf::from(format!("/tmp/test_package_{}.rpm", i));
            std::fs::write(&path, "test content").ok();
            pool.submit(Job::ProcessPackage(path));
        }

        // Wait for all results
        let mut completed = 0;
        let start = Instant::now();
        while completed < 4 && start.elapsed().as_secs() < 5 {
            if let Ok(result) = results.recv_timeout(Duration::from_millis(100)) {
                completed += 1;
                match result {
                    ProcessingResult::Success(path, _pkg) => {
                        assert!(path.to_string_lossy().contains("test_package"));
                    }
                    ProcessingResult::Error(_, err) => {
                        panic!("Unexpected error: {}", err);
                    }
                }
            }
        }

        assert_eq!(completed, 4, "All 4 jobs should complete");

        pool.join();

        for i in 0..4 {
            let path = PathBuf::from(format!("/tmp/test_package_{}.rpm", i));
            std::fs::remove_file(path).ok();
        }
    }

    #[test]
    fn test_worker_pool_shutdown() {
        let (pool, _results) = WorkerPool::new(2);
        pool.submit(Job::Other("test".to_string()));
        pool.shutdown();
        pool.join();
    }

    #[test]
    fn test_worker_pool_submit_after_shutdown() {
        let (pool, _results) = WorkerPool::new(2);
        pool.shutdown();
        pool.join();

        let result = pool.submit(Job::Other("test".to_string()));
        assert!(!result, "Submit should return false after shutdown");
    }
}