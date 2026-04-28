//! Index Status Tracker - Tracks ongoing indexing operations
//!
//! This module provides real-time status information about background indexing
//! operations for UI display and monitoring.

use crate::progress::{InMemoryProgressStore, Job, JobStatus, JobType, ProgressStore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum IndexingState {
    Idle,
    Indexing,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexingStatus {
    /// Current state of the indexing system
    pub state: IndexingState,
    /// Number of operations currently being processed
    pub operations_in_progress: usize,
    /// Total operations processed since startup
    pub total_operations_processed: u64,
    /// Total operations queued (waiting to be processed)
    pub operations_queued: usize,
    /// Timestamp of last indexing operation (Unix timestamp in seconds)
    pub last_operation_time: Option<u64>,
    /// Average processing time per operation (milliseconds)
    pub avg_processing_time_ms: f64,
    /// Current throughput in operations per second
    pub operations_per_second: f64,
    /// Current batch being processed (if any)
    pub current_batch_size: Option<usize>,
    /// Timestamp when current batch started (Unix timestamp in seconds)
    pub current_batch_start_time: Option<u64>,
}

impl Default for IndexingStatus {
    fn default() -> Self {
        Self {
            state: IndexingState::Idle,
            operations_in_progress: 0,
            total_operations_processed: 0,
            operations_queued: 0,
            last_operation_time: None,
            avg_processing_time_ms: 0.0,
            operations_per_second: 0.0,
            current_batch_size: None,
            current_batch_start_time: None,
        }
    }
}

#[derive(Clone)]
pub struct IndexStatusTracker {
    store: Arc<dyn ProgressStore>,
}

impl IndexStatusTracker {
    pub fn new(store: Option<Arc<dyn ProgressStore>>) -> Self {
        Self {
            store: store.unwrap_or_else(|| Arc::new(InMemoryProgressStore::new())),
        }
    }

    /// Get the user_id from task-local context - returns error if not available
    fn get_user_id(&self) -> Result<String, String> {
        crate::user_context::get_current_user_id()
            .ok_or_else(|| "User context required for index status tracking".to_string())
    }

    /// Helper to save/load status from generic store
    async fn save_status(&self, status: &IndexingStatus) -> Result<(), String> {
        let user_id = self.get_user_id()?;
        let mut job = Job::new("indexing_status".to_string(), JobType::Indexing)
            .with_user(user_id)
            .with_metadata(serde_json::to_value(status).unwrap());

        // Map status
        if status.state == IndexingState::Indexing {
            job.status = JobStatus::Running;
            job.message = format!("Indexing... {} queued", status.operations_queued);
        } else {
            job.status = JobStatus::Completed;
            job.message = "Idle".to_string();
        }

        self.store.save(&job).await
    }

    async fn load_status(&self) -> Result<IndexingStatus, String> {
        let user_id = self.get_user_id()?;
        let jobs = self.store.list_by_user(&user_id).await?;
        if let Some(job) = jobs.iter().find(|j| j.id == "indexing_status") {
            Ok(serde_json::from_value(job.metadata.clone()).unwrap_or_default())
        } else {
            Ok(IndexingStatus::default())
        }
    }

    /// Mark the start of a batch indexing operation
    pub async fn start_batch(&self, batch_size: usize) {
        tracing::debug!("IndexStatusTracker: Starting batch of size {}", batch_size);
        let mut status = self.load_status().await.unwrap_or_default();

        status.state = IndexingState::Indexing;
        status.operations_in_progress = batch_size;
        status.current_batch_size = Some(batch_size);
        status.current_batch_start_time = Some(Self::current_timestamp());

        if let Err(e) = self.save_status(&status).await {
            tracing::error!(
                "IndexStatusTracker: Failed to save status in start_batch: {}",
                e
            );
        } else {
            tracing::debug!("IndexStatusTracker: Batch started, status saved");
        }
    }

    /// Mark the completion of a batch indexing operation
    pub async fn complete_batch(&self, batch_size: usize, duration_ms: u128) {
        tracing::debug!(
            "IndexStatusTracker: Completing batch of size {}, duration {}ms",
            batch_size,
            duration_ms
        );
        let mut status = self.load_status().await.unwrap_or_default();

        // Ensure the "Indexing" state is visible for at least 500ms
        if let Some(start_time) = status.current_batch_start_time {
            let elapsed = Self::current_timestamp() - start_time;
            if elapsed < 500 {
                tracing::debug!(
                    "IndexStatusTracker: Sleeping for {}ms to ensure visibility",
                    500 - elapsed
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(500 - elapsed)).await;
            }
        }

        status.state = IndexingState::Idle;
        status.operations_in_progress = 0;
        status.total_operations_processed += batch_size as u64;
        status.last_operation_time = Some(Self::current_timestamp());
        status.current_batch_size = None;
        status.current_batch_start_time = None;

        // Update statistics
        let current_avg = status.avg_processing_time_ms;
        let total_ops = status.total_operations_processed;

        // Moving average
        if total_ops > batch_size as u64 {
            status.avg_processing_time_ms = (current_avg * 0.9) + (duration_ms as f64 * 0.1);
        } else {
            status.avg_processing_time_ms = duration_ms as f64;
        }

        if duration_ms > 0 {
            status.operations_per_second = (batch_size as f64 / duration_ms as f64) * 1000.0;
        }

        if let Err(e) = self.save_status(&status).await {
            tracing::error!(
                "IndexStatusTracker: Failed to save status in complete_batch: {}",
                e
            );
        } else {
            tracing::debug!(
                "IndexStatusTracker: Batch completed, status saved. Total ops: {}",
                status.total_operations_processed
            );
        }
    }

    /// Update the number of operations queued
    pub async fn set_queued(&self, count: usize) {
        let mut status = self.load_status().await.unwrap_or_default();
        status.operations_queued = count;
        if let Err(e) = self.save_status(&status).await {
            tracing::error!(
                "IndexStatusTracker: Failed to save status in set_queued: {}",
                e
            );
        }
    }

    /// Get the current indexing status
    pub async fn get_status(&self) -> IndexingStatus {
        match self.load_status().await {
            Ok(status) => {
                tracing::debug!("IndexStatusTracker: get_status loaded: {:?}", status);
                status
            }
            Err(e) => {
                // If it fails (e.g. timeout), log and return default
                tracing::error!("IndexStatusTracker: Failed to load status: {}", e);
                IndexingStatus::default()
            }
        }
    }

    /// Check if indexing is currently in progress
    pub async fn is_indexing(&self) -> bool {
        self.get_status().await.state == IndexingState::Indexing
    }

    /// Get current timestamp in seconds since Unix epoch
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}
