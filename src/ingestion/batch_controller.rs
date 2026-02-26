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

/// A file currently being processed (in-flight).
#[derive(Debug, Clone)]
pub struct InFlightFile {
    pub name: String,
    pub progress_id: String,
}

/// A file that failed during batch processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedFile {
    pub name: String,
    pub error: String,
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
    pub failed_files: Vec<FailedFile>,
    pub pending_files: Vec<PendingFile>,
    /// Files currently being processed concurrently.
    pub in_flight_files: Vec<InFlightFile>,
    /// Whether the AI provider is local (e.g. Ollama) and therefore free.
    pub is_local_provider: bool,
    resume_notify: Arc<Notify>,
}

impl BatchController {
    pub fn new(
        batch_id: String,
        spend_limit: Option<f64>,
        pending_files: Vec<PendingFile>,
        is_local_provider: bool,
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
            failed_files: Vec::new(),
            pending_files,
            in_flight_files: Vec::new(),
            is_local_provider,
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

    /// Track a file that has started processing.
    pub fn add_in_flight(&mut self, name: String, progress_id: String) {
        self.in_flight_files.push(InFlightFile { name, progress_id });
    }

    /// Remove a file from the in-flight list by progress_id.
    fn remove_in_flight(&mut self, progress_id: &str) {
        self.in_flight_files.retain(|f| f.progress_id != progress_id);
    }

    /// Number of files currently being processed.
    pub fn in_flight_count(&self) -> usize {
        self.in_flight_files.len()
    }

    /// Record that a file finished processing with the given actual cost.
    pub fn record_completed(&mut self, progress_id: &str, cost: f64) {
        self.accumulated_cost += cost;
        self.files_completed += 1;
        self.remove_in_flight(progress_id);
    }

    /// Record that a file failed.
    pub fn record_failed(&mut self, progress_id: &str, name: String, error: String) {
        self.files_failed += 1;
        self.failed_files.push(FailedFile { name, error });
        self.remove_in_flight(progress_id);
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
    pub failed_files: Vec<FailedFile>,
    pub files_remaining: usize,
    pub estimated_remaining_cost: f64,
    /// Number of files currently being processed concurrently.
    pub in_flight_count: usize,
    /// Name of a file currently being processed (first in-flight, for backward compat).
    pub current_file_name: Option<String>,
    /// Current processing step message for the active file.
    pub current_file_step: Option<String>,
    /// Progress percentage (0-100) for the active file.
    pub current_file_progress: Option<u8>,
    /// Whether the AI provider is local (e.g. Ollama) and therefore free.
    pub is_local_provider: bool,
}

impl BatchStatusResponse {
    pub fn from_controller(ctrl: &BatchController) -> Self {
        let current_file_name = ctrl.in_flight_files.first().map(|f| f.name.clone());
        Self {
            batch_id: ctrl.batch_id.clone(),
            status: ctrl.status,
            spend_limit: ctrl.spend_limit,
            accumulated_cost: ctrl.accumulated_cost,
            files_total: ctrl.files_total,
            files_completed: ctrl.files_completed,
            files_failed: ctrl.files_failed,
            failed_files: ctrl.failed_files.clone(),
            files_remaining: ctrl.files_remaining(),
            estimated_remaining_cost: ctrl.estimated_remaining_cost(),
            in_flight_count: ctrl.in_flight_count(),
            current_file_name,
            // Filled in by the route handler from ProgressTracker
            current_file_step: None,
            current_file_progress: None,
            is_local_provider: ctrl.is_local_provider,
        }
    }
}
