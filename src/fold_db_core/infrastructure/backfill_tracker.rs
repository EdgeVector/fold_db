//! Backfill tracking for transform execution monitoring
//!
//! Tracks the progress and status of backfill operations when transforms are registered

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use ts_rs::TS;

/// Status of a backfill operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "src/datafold_node/static-react/src/types/generated.ts")]
pub enum BackfillStatus {
    /// Backfill is currently in progress
    InProgress,
    /// Backfill completed successfully
    Completed,
    /// Backfill failed with error
    Failed,
}

/// Information about a backfill operation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "src/datafold_node/static-react/src/types/generated.ts")]
pub struct BackfillInfo {
    /// Unique hash identifying this specific backfill operation
    pub backfill_hash: String,
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
    /// Expected number of mutations to be created (for completion tracking)
    pub mutations_expected: u64,
    /// Number of mutations completed so far
    pub mutations_completed: u64,
    /// Number of mutations that failed
    pub mutations_failed: u64,
}

impl BackfillInfo {
    /// Create a new backfill info in progress state with a unique hash
    pub fn new(transform_id: String, source_schema: String) -> Self {
        let backfill_hash = Self::generate_backfill_hash(&transform_id, &source_schema);
        Self {
            backfill_hash,
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
            mutations_expected: 0,
            mutations_completed: 0,
            mutations_failed: 0,
        }
    }

    /// Generate a unique hash for this backfill operation
    /// Uses transform_id, source_schema, and timestamp to ensure uniqueness
    /// Uses seahash for stable, high-quality hashing across Rust versions
    fn generate_backfill_hash(transform_id: &str, source_schema: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time error")
            .as_nanos();
        
        // Concatenate inputs for hashing
        let input = format!("{}:{}:{}", transform_id, source_schema, timestamp);
        let hash = seahash::hash(input.as_bytes());
        
        format!("backfill_{:016x}", hash)
    }

    /// Create a new backfill info with a specific hash (used when hash is pre-generated)
    pub fn new_with_hash(backfill_hash: String, transform_id: String, source_schema: String) -> Self {
        Self {
            backfill_hash,
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
            mutations_expected: 0,
            mutations_completed: 0,
            mutations_failed: 0,
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

    /// Start tracking a new backfill and return the unique backfill hash
    pub fn start_backfill(&self, transform_id: String, source_schema: String) -> String {
        let info = BackfillInfo::new(transform_id.clone(), source_schema);
        let backfill_hash = info.backfill_hash.clone();
        
        // Store by hash
        self.backfills.lock().unwrap().insert(backfill_hash.clone(), info);
        
        // Update transform_id -> hash mapping
        self.transform_to_hash.lock().unwrap().insert(transform_id, backfill_hash.clone());
        
        backfill_hash
    }

    /// Start tracking a backfill with a pre-generated hash
    pub fn start_backfill_with_hash(&self, backfill_hash: String, transform_id: String, source_schema: String) {
        let info = BackfillInfo::new_with_hash(backfill_hash.clone(), transform_id.clone(), source_schema);
        
        // Store by hash
        self.backfills.lock().unwrap().insert(backfill_hash.clone(), info);
        
        // Update transform_id -> hash mapping
        self.transform_to_hash.lock().unwrap().insert(transform_id, backfill_hash);
    }

    /// Generate a backfill hash without starting a backfill (for pre-generation)
    pub fn generate_hash(transform_id: &str, source_schema: &str) -> String {
        BackfillInfo::generate_backfill_hash(transform_id, source_schema)
    }

    /// Update backfill progress by transform_id (uses latest backfill for that transform)
    pub fn update_progress(
        &self,
        transform_id: &str,
        items_processed: u64,
        items_total: Option<u64>,
    ) {
        if let Some(hash) = self.transform_to_hash.lock().unwrap().get(transform_id) {
            if let Some(info) = self.backfills.lock().unwrap().get_mut(hash) {
                info.update_progress(items_processed, items_total);
            }
        }
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
                info.end_time = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("System time error")
                        .as_secs(),
                );
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
                info.end_time = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );
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
                info.end_time = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );
            }
        }
    }

    /// Mark backfill as completed by transform_id (uses latest backfill for that transform)
    /// Note: This is the old method that doesn't wait for mutations
    pub fn complete_backfill(&self, transform_id: &str, records_produced: u64) {
        if let Some(hash) = self.transform_to_hash.lock().unwrap().get(transform_id) {
            if let Some(info) = self.backfills.lock().unwrap().get_mut(hash) {
                info.mark_completed(records_produced);
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


