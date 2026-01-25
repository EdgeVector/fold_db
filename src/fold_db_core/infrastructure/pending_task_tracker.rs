use log::{debug, info, warn};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

/// Tracks pending background tasks to ensure clean shutdown in serverless environments.
///
/// This acts effectively as a WaitGroup that allows the main process to wait
/// for all side-effects (indexing, transforms, backfills) to complete.
#[derive(Debug)]
pub struct PendingTaskTracker {
    count: AtomicUsize,
    notify: Arc<Notify>,
}

impl PendingTaskTracker {
    /// Create a new tracker
    pub fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Increment the pending task count
    /// Call this BEFORE starting an async background operation
    pub fn increment(&self) {
        let prev = self.count.fetch_add(1, Ordering::SeqCst);
        debug!("Pending tasks incremented: {} -> {}", prev, prev + 1);
    }

    /// Decrement the pending task count
    /// Call this AFTER an async background operation completes
    pub fn decrement(&self) {
        let prev = self.count.fetch_sub(1, Ordering::SeqCst);
        let new_val = prev - 1;
        debug!("Pending tasks decremented: {} -> {}", prev, new_val);

        if new_val == 0 {
            self.notify.notify_waiters();
        }
    }

    /// Get current pending task count
    pub fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    /// Wait for all pending tasks to complete with a timeout
    pub async fn wait_for_completion(&self, timeout_duration: Duration) -> bool {
        let count = self.count();
        if count == 0 {
            return true;
        }

        info!(
            "Waiting for {} pending background tasks to complete...",
            count
        );

        // Simple loop with notification check
        // We use a timeout on the notification
        let start = std::time::Instant::now();

        loop {
            if self.count() == 0 {
                info!("All background tasks completed in {:?}", start.elapsed());
                return true;
            }

            if start.elapsed() >= timeout_duration {
                warn!(
                    "Timeout waiting for background tasks. Remaining: {}",
                    self.count()
                );
                return false;
            }

            // Wait for notification or timeout
            // We use a small internal timeout for the notify to re-check condition loop
            match tokio::time::timeout(Duration::from_millis(100), self.notify.notified()).await {
                Ok(_) => {
                    // Notified (count hit 0 usually), loop will check count
                }
                Err(_) => {
                    // Timeout on notify, just loop to check global timeout
                }
            }
        }
    }
}

impl Default for PendingTaskTracker {
    fn default() -> Self {
        Self::new()
    }
}
