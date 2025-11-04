//! Backfill tracking for transform execution monitoring
//!
//! Tracks the progress and status of backfill operations when transforms are registered

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Get current Unix timestamp in seconds
#[inline]
fn current_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before Unix epoch")
        .as_secs()
}

/// Get current Unix timestamp in nanoseconds
#[inline]
fn current_timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before Unix epoch")
        .as_nanos()
}

/// Status of a backfill operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "bindings/src/datafold_node/static-react/src/types/generated.ts")
)]
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
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "bindings/src/datafold_node/static-react/src/types/generated.ts")
)]
pub struct BackfillInfo {
    /// Unique hash identifying this specific backfill operation
    pub backfill_hash: String,
    /// Transform ID being backfilled
    pub transform_id: String,
    /// Schema name
    pub schema_name: String,
    /// Current status
    pub status: BackfillStatus,
    /// When the backfill started (Unix timestamp in seconds)
    #[cfg_attr(feature = "ts-bindings", ts(type = "number"))]
    pub start_time: u64,
    /// When the backfill completed (if finished)
    #[cfg_attr(feature = "ts-bindings", ts(type = "number"))]
    pub end_time: Option<u64>,
    /// Error message if failed
    pub error: Option<String>,
    /// Records produced by the backfill
    #[cfg_attr(feature = "ts-bindings", ts(type = "number"))]
    pub records_produced: u64,
    /// Expected number of mutations to be created (for completion tracking)
    #[cfg_attr(feature = "ts-bindings", ts(type = "number"))]
    pub mutations_expected: u64,
    /// Number of mutations completed so far
    #[cfg_attr(feature = "ts-bindings", ts(type = "number"))]
    pub mutations_completed: u64,
    /// Number of mutations that failed
    #[cfg_attr(feature = "ts-bindings", ts(type = "number"))]
    pub mutations_failed: u64,
}

/// Aggregate statistics from all backfills
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "bindings/src/datafold_node/static-react/src/types/generated.ts")
)]
pub struct BackfillStatistics {
    /// Total number of backfills
    pub total_backfills: usize,
    /// Number of backfills currently in progress
    pub active_backfills: usize,
    /// Number of completed backfills
    pub completed_backfills: usize,
    /// Number of failed backfills
    pub failed_backfills: usize,
    /// Total mutations expected across all backfills
    #[cfg_attr(feature = "ts-bindings", ts(type = "number"))]
    pub total_mutations_expected: u64,
    /// Total mutations completed across all backfills
    #[cfg_attr(feature = "ts-bindings", ts(type = "number"))]
    pub total_mutations_completed: u64,
    /// Total mutations failed across all backfills
    #[cfg_attr(feature = "ts-bindings", ts(type = "number"))]
    pub total_mutations_failed: u64,
    /// Total records produced across all backfills
    #[cfg_attr(feature = "ts-bindings", ts(type = "number"))]
    pub total_records_produced: u64,
}

impl BackfillInfo {
    /// Generate a unique hash for this backfill operation
    /// Uses transform_id, schema_name, and timestamp to ensure uniqueness
    /// Uses seahash for stable, high-quality hashing across Rust versions
    fn generate_backfill_hash(transform_id: &str, schema_name: &str) -> String {
        let timestamp = current_timestamp_nanos();
        
        // Concatenate inputs for hashing
        let input = format!("{}:{}:{}", transform_id, schema_name, timestamp);
        let hash = seahash::hash(input.as_bytes());
        
        format!("backfill_{:016x}", hash)
    }

    /// Create a new backfill info with a specific hash (used when hash is pre-generated)
    pub fn new_with_hash(backfill_hash: String, transform_id: String, schema_name: String) -> Self {
        Self {
            backfill_hash,
            transform_id,
            schema_name,
            status: BackfillStatus::InProgress,
            start_time: current_timestamp_secs(),
            end_time: None,
            error: None,
            records_produced: 0,
            mutations_expected: 0,
            mutations_completed: 0,
            mutations_failed: 0,
        }
    }

    /// Mark backfill as failed
    pub fn mark_failed(&mut self, error: String) {
        self.status = BackfillStatus::Failed;
        self.error = Some(error);
        self.end_time = Some(current_timestamp_secs());
    }

    /// Calculate duration in seconds
    pub fn duration_seconds(&self) -> u64 {
        let end = self.end_time.unwrap_or_else(current_timestamp_secs);
        end.saturating_sub(self.start_time)
    }
}

/// Tracker for all backfill operations
#[derive(Debug, Clone)]
pub struct BackfillTracker {
    /// Current and historical backfill operations indexed by backfill_hash
    backfills: Arc<Mutex<HashMap<String, BackfillInfo>>>,
    /// Index from transform_id to latest backfill_hash for quick lookup
    transform_to_hash: Arc<Mutex<HashMap<String, String>>>,
}

impl BackfillTracker {
    /// Create a new backfill tracker
    pub fn new() -> Self {
        Self {
            backfills: Arc::new(Mutex::new(HashMap::new())),
            transform_to_hash: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start tracking a backfill with a pre-generated hash
    pub fn start_backfill_with_hash(&self, backfill_hash: String, transform_id: String, schema_name: String) {
        let info = BackfillInfo::new_with_hash(backfill_hash.clone(), transform_id.clone(), schema_name);
        
        // Store by hash
        self.backfills.lock().unwrap().insert(backfill_hash.clone(), info);
        
        // Update transform_id -> hash mapping
        self.transform_to_hash.lock().unwrap().insert(transform_id, backfill_hash);
    }

    /// Generate a backfill hash without starting a backfill (for pre-generation)
    pub fn generate_hash(transform_id: &str, schema_name: &str) -> String {
        BackfillInfo::generate_backfill_hash(transform_id, schema_name)
    }

    /// Set the expected number of mutations for this backfill
    /// If count is 0, immediately mark the backfill as completed (no data to process)
    pub fn set_mutations_expected(&self, backfill_hash: &str, count: u64) {
        if let Some(info) = self.backfills.lock().unwrap().get_mut(backfill_hash) {
            info.mutations_expected = count;
            info.records_produced = count; // Also set records_produced to match
            
            // If no mutations are expected, immediately mark as completed
            if count == 0 && info.status == BackfillStatus::InProgress {
                info.status = BackfillStatus::Completed;
                info.end_time = Some(current_timestamp_secs());
            }
        }
    }

    /// Increment completed mutation count for a backfill
    /// Returns true if all mutations are now complete
    pub fn increment_mutation_completed(&self, backfill_hash: &str) -> bool {
        let mut backfills = self.backfills.lock().unwrap();
        if let Some(info) = backfills.get_mut(backfill_hash) {
            info.mutations_completed += 1;
            
            // Check if all mutations are complete
            if info.mutations_completed >= info.mutations_expected && info.mutations_expected > 0 && info.status == BackfillStatus::InProgress {
                info.status = BackfillStatus::Completed;
                info.end_time = Some(current_timestamp_secs());
                return true;
            }
        }
        false
    }

    /// Increment failed mutation count for a backfill
    /// If failure rate exceeds threshold, mark the backfill as failed
    pub fn increment_mutation_failed(&self, backfill_hash: &str, error: String) {
        let mut backfills = self.backfills.lock().unwrap();
        if let Some(info) = backfills.get_mut(backfill_hash) {
            info.mutations_failed += 1;
            
            // If more than 10% of mutations fail, mark the backfill as failed
            let total_processed = info.mutations_completed + info.mutations_failed;
            let failure_rate = if total_processed > 0 {
                info.mutations_failed as f64 / total_processed as f64
            } else {
                0.0
            };
            
            if failure_rate > 0.1 && total_processed > 10 {
                info.status = BackfillStatus::Failed;
                info.error = Some(format!("Backfill failed: {} mutations failed ({:.1}% failure rate). Last error: {}", 
                    info.mutations_failed, failure_rate * 100.0, error));
                info.end_time = Some(current_timestamp_secs());
            }
        }
    }

    /// Mark backfill as failed by transform_id (uses latest backfill for that transform)
    pub fn fail_backfill(&self, transform_id: &str, error: String) {
        if let Some(hash) = self.transform_to_hash.lock().unwrap().get(transform_id) {
            if let Some(info) = self.backfills.lock().unwrap().get_mut(hash) {
                info.mark_failed(error);
            }
        }
    }

    /// Get info for a specific backfill by transform_id (returns latest backfill)
    pub fn get_backfill(&self, transform_id: &str) -> Option<BackfillInfo> {
        self.transform_to_hash.lock().unwrap().get(transform_id)
            .and_then(|hash| self.backfills.lock().unwrap().get(hash).cloned())
    }

    /// Get info for a specific backfill by backfill_hash
    pub fn get_backfill_by_hash(&self, backfill_hash: &str) -> Option<BackfillInfo> {
        self.backfills.lock().unwrap().get(backfill_hash).cloned()
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


