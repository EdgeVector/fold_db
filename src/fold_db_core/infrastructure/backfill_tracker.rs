//! Backfill tracking for transform execution monitoring
//!
//! Tracks the progress and status of backfill operations when transforms are registered

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Status of a backfill operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackfillStatus {
    /// Backfill is currently in progress
    InProgress,
    /// Backfill completed successfully
    Completed,
    /// Backfill failed with error
    Failed,
}

/// Information about a backfill operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillInfo {
    /// Transform ID being backfilled
    pub transform_id: String,
    /// Source schema name
    pub source_schema: String,
    /// Current status
    pub status: BackfillStatus,
    /// Items processed so far
    pub items_processed: u64,
    /// Total items to process (if known)
    pub items_total: Option<u64>,
    /// When the backfill started
    pub start_time: u64,
    /// When the backfill completed (if finished)
    pub end_time: Option<u64>,
    /// Error message if failed
    pub error: Option<String>,
    /// Records produced by the backfill
    pub records_produced: u64,
}

impl BackfillInfo {
    /// Create a new backfill info in progress state
    pub fn new(transform_id: String, source_schema: String) -> Self {
        Self {
            transform_id,
            source_schema,
            status: BackfillStatus::InProgress,
            items_processed: 0,
            items_total: None,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            end_time: None,
            error: None,
            records_produced: 0,
        }
    }

    /// Mark backfill as completed
    pub fn mark_completed(&mut self, records_produced: u64) {
        self.status = BackfillStatus::Completed;
        self.records_produced = records_produced;
        self.end_time = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
    }

    /// Mark backfill as failed
    pub fn mark_failed(&mut self, error: String) {
        self.status = BackfillStatus::Failed;
        self.error = Some(error);
        self.end_time = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
    }

    /// Update progress
    pub fn update_progress(&mut self, items_processed: u64, items_total: Option<u64>) {
        self.items_processed = items_processed;
        self.items_total = items_total;
    }

    /// Calculate duration in seconds
    pub fn duration_seconds(&self) -> u64 {
        let end = self.end_time.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });
        end.saturating_sub(self.start_time)
    }

    /// Calculate progress percentage (0-100)
    pub fn progress_percentage(&self) -> Option<f64> {
        self.items_total.map(|total| {
            if total == 0 {
                100.0
            } else {
                (self.items_processed as f64 / total as f64) * 100.0
            }
        })
    }
}

/// Tracker for all backfill operations
#[derive(Debug, Clone)]
pub struct BackfillTracker {
    /// Current and historical backfill operations
    backfills: Arc<Mutex<HashMap<String, BackfillInfo>>>,
}

impl BackfillTracker {
    /// Create a new backfill tracker
    pub fn new() -> Self {
        Self {
            backfills: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start tracking a new backfill
    pub fn start_backfill(&self, transform_id: String, source_schema: String) {
        let info = BackfillInfo::new(transform_id.clone(), source_schema);
        self.backfills.lock().unwrap().insert(transform_id, info);
    }

    /// Update backfill progress
    pub fn update_progress(
        &self,
        transform_id: &str,
        items_processed: u64,
        items_total: Option<u64>,
    ) {
        if let Some(info) = self.backfills.lock().unwrap().get_mut(transform_id) {
            info.update_progress(items_processed, items_total);
        }
    }

    /// Mark backfill as completed
    pub fn complete_backfill(&self, transform_id: &str, records_produced: u64) {
        if let Some(info) = self.backfills.lock().unwrap().get_mut(transform_id) {
            info.mark_completed(records_produced);
        }
    }

    /// Mark backfill as failed
    pub fn fail_backfill(&self, transform_id: &str, error: String) {
        if let Some(info) = self.backfills.lock().unwrap().get_mut(transform_id) {
            info.mark_failed(error);
        }
    }

    /// Get info for a specific backfill
    pub fn get_backfill(&self, transform_id: &str) -> Option<BackfillInfo> {
        self.backfills.lock().unwrap().get(transform_id).cloned()
    }

    /// Get all backfill info
    pub fn get_all_backfills(&self) -> Vec<BackfillInfo> {
        self.backfills
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    /// Get only active (in-progress) backfills
    pub fn get_active_backfills(&self) -> Vec<BackfillInfo> {
        self.backfills
            .lock()
            .unwrap()
            .values()
            .filter(|info| info.status == BackfillStatus::InProgress)
            .cloned()
            .collect()
    }

    /// Get completed backfills
    pub fn get_completed_backfills(&self) -> Vec<BackfillInfo> {
        self.backfills
            .lock()
            .unwrap()
            .values()
            .filter(|info| info.status == BackfillStatus::Completed)
            .cloned()
            .collect()
    }

    /// Get failed backfills
    pub fn get_failed_backfills(&self) -> Vec<BackfillInfo> {
        self.backfills
            .lock()
            .unwrap()
            .values()
            .filter(|info| info.status == BackfillStatus::Failed)
            .cloned()
            .collect()
    }

    /// Clear old completed backfills (keep only recent ones)
    pub fn cleanup_old_backfills(&self, keep_count: usize) {
        let mut backfills = self.backfills.lock().unwrap();
        
        let mut completed: Vec<_> = backfills
            .iter()
            .filter(|(_, info)| info.status == BackfillStatus::Completed)
            .map(|(id, info)| (id.clone(), info.start_time))
            .collect();
        
        completed.sort_by_key(|(_, time)| *time);
        
        if completed.len() > keep_count {
            let to_remove = &completed[..completed.len() - keep_count];
            for (id, _) in to_remove {
                backfills.remove(id);
            }
        }
    }
}

impl Default for BackfillTracker {
    fn default() -> Self {
        Self::new()
    }
}


