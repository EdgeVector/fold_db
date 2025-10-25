//! Index Status Tracker - Tracks ongoing indexing operations
//!
//! This module provides real-time status information about background indexing
//! operations for UI display and monitoring.

use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
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
    status: Arc<RwLock<IndexingStatus>>,
}

impl IndexStatusTracker {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(IndexingStatus::default())),
        }
    }

    /// Mark the start of a batch indexing operation
    pub fn start_batch(&self, batch_size: usize) {
        if let Ok(mut status) = self.status.write() {
            status.state = IndexingState::Indexing;
            status.operations_in_progress = batch_size;
            status.current_batch_size = Some(batch_size);
            status.current_batch_start_time = Some(Self::current_timestamp());
        }
    }

    /// Mark the completion of a batch indexing operation
    pub fn complete_batch(&self, batch_size: usize, duration_ms: u128) {
        if let Ok(mut status) = self.status.write() {
            status.state = IndexingState::Idle;
            status.operations_in_progress = 0;
            status.total_operations_processed += batch_size as u64;
            status.last_operation_time = Some(Self::current_timestamp());
            status.current_batch_size = None;
            status.current_batch_start_time = None;
            
            // Update average processing time (exponential moving average)
            let new_avg = duration_ms as f64 / batch_size as f64;
            if status.avg_processing_time_ms == 0.0 {
                status.avg_processing_time_ms = new_avg;
            } else {
                // EMA with alpha = 0.3 (weight recent operations more)
                status.avg_processing_time_ms = 
                    0.3 * new_avg + 0.7 * status.avg_processing_time_ms;
            }
            
            // Calculate operations per second (ops/sec)
            // duration_ms is milliseconds, so we convert to seconds
            if duration_ms > 0 {
                status.operations_per_second = (batch_size as f64 * 1000.0) / duration_ms as f64;
            } else {
                status.operations_per_second = 0.0;
            }
        }
    }

    /// Update the number of operations queued
    pub fn set_queued(&self, count: usize) {
        if let Ok(mut status) = self.status.write() {
            status.operations_queued = count;
        }
    }

    /// Get the current indexing status
    pub fn get_status(&self) -> IndexingStatus {
        self.status.read()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// Check if indexing is currently in progress
    pub fn is_indexing(&self) -> bool {
        self.status.read()
            .map(|s| s.state == IndexingState::Indexing)
            .unwrap_or(false)
    }

    /// Get current timestamp in seconds since Unix epoch
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

impl Default for IndexStatusTracker {
    fn default() -> Self {
        Self::new()
    }
}

