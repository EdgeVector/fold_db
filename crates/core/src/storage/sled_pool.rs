//! SledPool — lazy Sled connection with idle auto-release.
//!
//! Instead of holding an exclusive file lock on the Sled database forever,
//! the pool opens Sled on demand and releases it after a period of inactivity.
//! This allows multiple processes (Tauri app, dev server, CLI) to share the
//! same database directory without permanent lock contention.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::error::{StorageError, StorageResult};

/// A pool managing a single Sled database connection with lazy open and idle release.
///
/// All Sled consumers (SledKvStore, NodeConfigStore, org operations) acquire
/// a `SledGuard` from the pool. The guard keeps the database open via an
/// active-operation counter. When all guards are dropped and the idle timeout
/// elapses, the database is closed and the file lock is released.
pub struct SledPool {
    path: PathBuf,
    inner: Mutex<PoolInner>,
}

struct PoolInner {
    db: Option<sled::Db>,
    active_ops: usize,
    last_activity: Instant,
}

/// RAII guard that keeps a Sled database open for the duration of an operation.
/// Dropping the guard decrements the active operation counter.
pub struct SledGuard {
    pool: Arc<SledPool>,
    db: sled::Db,
}

impl SledGuard {
    /// Access the Sled database handle.
    pub fn db(&self) -> &sled::Db {
        &self.db
    }
}

impl Drop for SledGuard {
    fn drop(&mut self) {
        let mut inner = self.pool.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.active_ops = inner.active_ops.saturating_sub(1);
        inner.last_activity = Instant::now();
    }
}

impl SledPool {
    /// Create a new pool for the given database path. Does NOT open Sled yet.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            inner: Mutex::new(PoolInner {
                db: None,
                active_ops: 0,
                last_activity: Instant::now(),
            }),
        }
    }

    /// Get the database path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Acquire a Sled handle. Opens the database if not already open.
    /// If another process holds the lock, retries with exponential backoff
    /// (up to ~5s total) to wait for the other process's idle release.
    pub fn acquire_arc(self: &Arc<Self>) -> StorageResult<SledGuard> {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());

        let db = if let Some(ref db) = inner.db {
            db.clone()
        } else {
            let mut last_err = String::new();
            let mut db_result = None;
            for attempt in 0..10 {
                match sled::open(&self.path) {
                    Ok(db) => {
                        db_result = Some(db);
                        break;
                    }
                    Err(e) => {
                        last_err = e.to_string();
                        if last_err.contains("WouldBlock")
                            || last_err.contains("acquire lock")
                            || last_err.contains("Resource temporarily unavailable")
                        {
                            // Lock held by another process — wait and retry
                            drop(inner);
                            let delay = Duration::from_millis(100 * (1 << attempt.min(4)));
                            std::thread::sleep(delay);
                            inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
                            // Check if another thread opened it while we waited
                            if let Some(ref db) = inner.db {
                                db_result = Some(db.clone());
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
            }

            match db_result {
                Some(db) => {
                    inner.db = Some(db.clone());
                    db
                }
                None => {
                    return Err(StorageError::ConfigurationError(format!(
                        "Failed to open sled database after retries: {}",
                        last_err
                    )));
                }
            }
        };

        inner.active_ops += 1;
        inner.last_activity = Instant::now();
        drop(inner);

        Ok(SledGuard {
            pool: Arc::clone(self),
            db,
        })
    }

    /// Start a background task that releases the Sled handle after idle timeout.
    /// Call this once after creating the pool.
    pub fn start_idle_reaper(self: &Arc<Self>, idle_timeout: Duration) {
        let pool = Arc::clone(self);
        // lint:spawn-bare-ok boot-time idle reaper — perpetual worker, no per-request parent span.
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                let mut inner = pool.inner.lock().unwrap_or_else(|p| p.into_inner());
                if inner.db.is_some()
                    && inner.active_ops == 0
                    && inner.last_activity.elapsed() >= idle_timeout
                {
                    tracing::debug!(
                        "SledPool: releasing idle database at {}",
                        pool.path.display()
                    );
                    inner.db = None;
                }
            }
        });
    }

    /// Manually release the database handle (for testing or explicit cleanup).
    pub fn release(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if inner.active_ops == 0 {
            inner.db = None;
        }
    }

    /// Check if the database is currently open.
    pub fn is_open(&self) -> bool {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.db.is_some()
    }

    /// Get the number of active operations.
    pub fn active_ops(&self) -> usize {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.active_ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_pool_lazy_open() {
        let tmp = TempDir::new().unwrap();
        let pool = Arc::new(SledPool::new(tmp.path().join("db")));

        assert!(!pool.is_open());

        let guard = pool.acquire_arc().unwrap();
        assert!(pool.is_open());
        assert_eq!(pool.active_ops(), 1);

        // Can use the database
        let tree = guard.db().open_tree("test").unwrap();
        tree.insert("key", "value").unwrap();

        drop(guard);
        assert_eq!(pool.active_ops(), 0);
    }

    #[test]
    fn test_pool_release() {
        let tmp = TempDir::new().unwrap();
        let pool = Arc::new(SledPool::new(tmp.path().join("db")));

        let guard = pool.acquire_arc().unwrap();
        drop(guard);

        assert!(pool.is_open()); // Still open (cached)
        pool.release();
        assert!(!pool.is_open()); // Now closed
    }

    #[test]
    fn test_pool_no_release_while_active() {
        let tmp = TempDir::new().unwrap();
        let pool = Arc::new(SledPool::new(tmp.path().join("db")));

        let guard = pool.acquire_arc().unwrap();
        pool.release(); // Should NOT close — guard is active
        assert!(pool.is_open());

        drop(guard);
        pool.release(); // Now it can close
        assert!(!pool.is_open());
    }

    #[test]
    fn test_pool_multiple_guards() {
        let tmp = TempDir::new().unwrap();
        let pool = Arc::new(SledPool::new(tmp.path().join("db")));

        let g1 = pool.acquire_arc().unwrap();
        let g2 = pool.acquire_arc().unwrap();
        assert_eq!(pool.active_ops(), 2);

        drop(g1);
        assert_eq!(pool.active_ops(), 1);
        pool.release(); // Should NOT close — g2 active
        assert!(pool.is_open());

        drop(g2);
        assert_eq!(pool.active_ops(), 0);
        pool.release();
        assert!(!pool.is_open());
    }

    #[test]
    fn test_pool_reopen_after_release() {
        let tmp = TempDir::new().unwrap();
        let pool = Arc::new(SledPool::new(tmp.path().join("db")));

        // Write data
        {
            let guard = pool.acquire_arc().unwrap();
            let tree = guard.db().open_tree("test").unwrap();
            tree.insert("key", "value").unwrap();
            tree.flush().unwrap();
        }
        pool.release();
        assert!(!pool.is_open());

        // Reopen and read data
        let guard = pool.acquire_arc().unwrap();
        let tree = guard.db().open_tree("test").unwrap();
        let val = tree.get("key").unwrap().unwrap();
        assert_eq!(val.as_ref(), b"value");
    }
}
