//! Index Status Tracker - Tracks ongoing indexing operations
//!
//! This module provides real-time status information about background indexing
//! operations for UI display and monitoring.

use super::progress_store::{InMemoryProgressStore, ProgressStore};
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

    /// Mark the start of a batch indexing operation
    pub async fn start_batch(&self, batch_size: usize) {
        log::debug!("IndexStatusTracker: Starting batch of size {}", batch_size);
        let mut status = self.store.load_status().await.unwrap_or_default();

        status.state = IndexingState::Indexing;
        status.operations_in_progress = batch_size;
        status.current_batch_size = Some(batch_size);
        status.current_batch_start_time = Some(Self::current_timestamp());

        if let Err(e) = self.store.save_status(&status).await {
            log::error!(
                "IndexStatusTracker: Failed to save status in start_batch: {}",
                e
            );
        } else {
            log::debug!("IndexStatusTracker: Batch started, status saved");
        }
    }

    /// Mark the completion of a batch indexing operation
    pub async fn complete_batch(&self, batch_size: usize, duration_ms: u128) {
        log::debug!(
            "IndexStatusTracker: Completing batch of size {}, duration {}ms",
            batch_size,
            duration_ms
        );
        let mut status = self.store.load_status().await.unwrap_or_default();

        // Ensure the "Indexing" state is visible for at least 500ms
        if let Some(start_time) = status.current_batch_start_time {
            let elapsed = Self::current_timestamp() - start_time;
            if elapsed < 500 {
                log::debug!(
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

        if let Err(e) = self.store.save_status(&status).await {
            log::error!(
                "IndexStatusTracker: Failed to save status in complete_batch: {}",
                e
            );
        } else {
            log::debug!(
                "IndexStatusTracker: Batch completed, status saved. Total ops: {}",
                status.total_operations_processed
            );
        }
    }

    /// Update the number of operations queued
    pub async fn set_queued(&self, count: usize) {
        let mut status = self.store.load_status().await.unwrap_or_default();
        status.operations_queued = count;
        if let Err(e) = self.store.save_status(&status).await {
            log::error!(
                "IndexStatusTracker: Failed to save status in set_queued: {}",
                e
            );
        }
    }

    /// Get the current indexing status
    pub async fn get_status(&self) -> IndexingStatus {
        match self.store.load_status().await {
            Ok(status) => {
                log::debug!("IndexStatusTracker: get_status loaded: {:?}", status);
                status
            }
            Err(e) => {
                log::error!("IndexStatusTracker: Failed to load status: {}", e);
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
