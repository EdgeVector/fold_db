//! Batch-level state management for smart folder ingestion.
//!
//! `BatchController` tracks spend limits, accumulated cost, file progress,
//! and supports pause/resume via `tokio::sync::Notify`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

/// Status of a batch ingestion run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchStatus {
    Running,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

impl std::fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchStatus::Running => write!(f, "Running"),
            BatchStatus::Paused => write!(f, "Paused"),
            BatchStatus::Completed => write!(f, "Completed"),
            BatchStatus::Cancelled => write!(f, "Cancelled"),
            BatchStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// A pending file in the batch queue.
#[derive(Debug, Clone)]
pub struct PendingFile {
    pub path: PathBuf,
    pub progress_id: String,
    pub estimated_cost: f64,
}

/// Controls a single batch ingestion: spend limit, cost tracking, pause/resume.
pub struct BatchController {
    pub batch_id: String,
    pub status: BatchStatus,
    pub spend_limit: Option<f64>,
    pub accumulated_cost: f64,
    pub files_total: usize,
    pub files_completed: usize,
    pub files_failed: usize,
    pub pending_files: Vec<PendingFile>,
    /// Name of the file currently being processed.
    pub current_file_name: Option<String>,
    /// Progress ID of the file currently being processed.
    pub current_file_progress_id: Option<String>,
    resume_notify: Arc<Notify>,
}

impl BatchController {
    pub fn new(
        batch_id: String,
        spend_limit: Option<f64>,
        pending_files: Vec<PendingFile>,
    ) -> Self {
        let files_total = pending_files.len();
        Self {
            batch_id,
            status: BatchStatus::Running,
            spend_limit,
            accumulated_cost: 0.0,
            files_total,
            files_completed: 0,
            files_failed: 0,
            pending_files,
            current_file_name: None,
            current_file_progress_id: None,
            resume_notify: Arc::new(Notify::new()),
        }
    }

    /// Check whether the next file (with `next_cost`) fits within the spend limit.
    /// Returns `true` if we can proceed, `false` if we'd exceed the cap.
    pub fn can_proceed(&self, next_cost: f64) -> bool {
        match self.spend_limit {
            None => true,
            Some(limit) => self.accumulated_cost + next_cost <= limit,
        }
    }

    /// Transition to Paused.
    pub fn pause(&mut self) {
        self.status = BatchStatus::Paused;
    }

    /// Transition to Running with an optional new spend limit, then wake the coordinator.
    pub fn resume(&mut self, new_limit: Option<f64>) {
        if let Some(limit) = new_limit {
            self.spend_limit = Some(limit);
        }
        self.status = BatchStatus::Running;
        self.resume_notify.notify_one();
    }

    /// Cancel the batch. The coordinator will stop picking up new files.
    pub fn cancel(&mut self) {
        self.status = BatchStatus::Cancelled;
        // Wake the coordinator so it can observe Cancelled and exit.
        self.resume_notify.notify_one();
    }

    /// Set the file currently being processed.
    pub fn set_current_file(&mut self, name: String, progress_id: String) {
        self.current_file_name = Some(name);
        self.current_file_progress_id = Some(progress_id);
    }

    /// Clear the current file tracking.
    fn clear_current_file(&mut self) {
        self.current_file_name = None;
        self.current_file_progress_id = None;
    }

    /// Record that a file finished processing with the given actual cost.
    pub fn record_completed(&mut self, cost: f64) {
        self.accumulated_cost += cost;
        self.files_completed += 1;
        self.clear_current_file();
    }

    /// Record that a file failed.
    pub fn record_failed(&mut self) {
        self.files_failed += 1;
        self.clear_current_file();
    }

    /// Pop the next pending file from the front of the queue.
    pub fn pop_next_file(&mut self) -> Option<PendingFile> {
        if self.pending_files.is_empty() {
            None
        } else {
            Some(self.pending_files.remove(0))
        }
    }

    /// Estimated cost of remaining (pending) files.
    pub fn estimated_remaining_cost(&self) -> f64 {
        self.pending_files.iter().map(|f| f.estimated_cost).sum()
    }

    /// Number of files still pending.
    pub fn files_remaining(&self) -> usize {
        self.pending_files.len()
    }

    /// Get a handle to the Notify for waiting on resume.
    pub fn resume_notifier(&self) -> Arc<Notify> {
        self.resume_notify.clone()
    }
}

/// Shared map of active batch controllers, keyed by batch_id.
pub type BatchControllerMap = Arc<Mutex<HashMap<String, Arc<Mutex<BatchController>>>>>;

/// Create an empty BatchControllerMap.
pub fn create_batch_controller_map() -> BatchControllerMap {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Serialisable snapshot of batch state for the status endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchStatusResponse {
    pub batch_id: String,
    pub status: BatchStatus,
    pub spend_limit: Option<f64>,
    pub accumulated_cost: f64,
    pub files_total: usize,
    pub files_completed: usize,
    pub files_failed: usize,
    pub files_remaining: usize,
    pub estimated_remaining_cost: f64,
    /// Name of the file currently being processed.
    pub current_file_name: Option<String>,
    /// Current processing step message for the active file.
    pub current_file_step: Option<String>,
    /// Progress percentage (0-100) for the active file.
    pub current_file_progress: Option<u8>,
}

impl BatchStatusResponse {
    pub fn from_controller(ctrl: &BatchController) -> Self {
        Self {
            batch_id: ctrl.batch_id.clone(),
            status: ctrl.status,
            spend_limit: ctrl.spend_limit,
            accumulated_cost: ctrl.accumulated_cost,
            files_total: ctrl.files_total,
            files_completed: ctrl.files_completed,
            files_failed: ctrl.files_failed,
            files_remaining: ctrl.files_remaining(),
            estimated_remaining_cost: ctrl.estimated_remaining_cost(),
            current_file_name: ctrl.current_file_name.clone(),
            // Filled in by the route handler from ProgressTracker
            current_file_step: None,
            current_file_progress: None,
        }
    }
}
