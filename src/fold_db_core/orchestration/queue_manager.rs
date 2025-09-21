//! Queue management component for the Transform Orchestrator
//!
//! Handles thread-safe queue operations with proper locking, extracted from
//! the main TransformOrchestrator to improve separation of concerns.

use crate::schema::SchemaError;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

/// Represents a single item in the transform queue
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueueItem {
    pub id: String,
    pub mutation_hash: String,
}

/// Internal queue state with deduplication tracking
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct QueueState {
    pub queue: VecDeque<QueueItem>,
    pub queued: HashSet<String>,
    pub processed: HashSet<String>,
}

/// Thread-safe queue manager for transform orchestration
pub struct QueueManager {
    state: Arc<Mutex<QueueState>>,
}

impl QueueManager {
    /// Create a new QueueManager with the given initial state
    pub fn new(initial_state: QueueState) -> Self {
        Self {
            state: Arc::new(Mutex::new(initial_state)),
        }
    }

    /// Create a new QueueManager with empty state
    pub fn new_empty() -> Self {
        Self::new(QueueState::default())
    }

    /// Add an item to the queue if not already queued
    pub fn add_item(&self, transform_id: &str, mutation_hash: &str) -> Result<bool, SchemaError> {
        info!("🔒 Acquiring queue lock to add item: {}", transform_id);
        let mut state = self.state.lock().map_err(|e| {
            error!(
                "❌ Failed to acquire queue lock for adding {}: {}",
                transform_id, e
            );
            SchemaError::InvalidData("Failed to acquire queue lock".to_string())
        })?;

        let key = format!("{}|{}", transform_id, mutation_hash);
        info!("🔑 Generated key for item: {}", key);

        if state.queued.insert(key.clone()) {
            state.queue.push_back(QueueItem {
                id: transform_id.to_string(),
                mutation_hash: mutation_hash.to_string(),
            });
            info!(
                "✅ Successfully added item {} to queue with key: {}",
                transform_id, key
            );
            Ok(true)
        } else {
            info!(
                "ℹ️ Item {} with key {} already in queue, skipping",
                transform_id, key
            );
            Ok(false)
        }
    }

    /// Pop the next item from the queue
    pub fn pop_item(&self) -> Result<Option<QueueItem>, SchemaError> {
        info!("🔒 Acquiring queue lock for pop_item");
        let mut state = self.state.lock().map_err(|e| {
            error!("❌ Failed to acquire queue lock in pop_item: {}", e);
            SchemaError::InvalidData("Failed to acquire queue lock".to_string())
        })?;

        info!(
            "📋 Queue state before popping - length: {}, items: {:?}",
            state.queue.len(),
            state.queue
        );

        match state.queue.pop_front() {
            Some(item) => {
                let key = format!("{}|{}", item.id, item.mutation_hash);
                let was_in_queued = state.queued.remove(&key);

                info!(
                    "📤 Popped item from queue: {} (mutation_hash: {}, was_in_queued_set: {})",
                    item.id, item.mutation_hash, was_in_queued
                );
                info!("🔑 Processing key: {}", key);

                Ok(Some(item))
            }
            None => {
                info!("📭 Queue is empty, nothing to pop");
                Ok(None)
            }
        }
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> Result<bool, SchemaError> {
        let state = self.state.lock().map_err(|e| {
            error!("❌ Failed to acquire queue lock for empty check: {}", e);
            SchemaError::InvalidData("Failed to acquire queue lock".to_string())
        })?;
        Ok(state.queue.is_empty())
    }

    /// Get the current queue length
    pub fn len(&self) -> Result<usize, SchemaError> {
        let state = self.state.lock().map_err(|e| {
            error!("❌ Failed to acquire queue lock for length check: {}", e);
            SchemaError::InvalidData("Failed to acquire queue lock".to_string())
        })?;
        let length = state.queue.len();
        info!("📊 Current queue length: {}", length);
        Ok(length)
    }

    /// Mark an item as processed
    pub fn mark_processed(
        &self,
        transform_id: &str,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        info!("📝 Marking item as processed: {}", transform_id);
        let mut state = self.state.lock().map_err(|e| {
            error!(
                "❌ Failed to acquire queue lock for marking processed: {}",
                e
            );
            SchemaError::InvalidData("Failed to acquire queue lock".to_string())
        })?;

        let key = format!("{}|{}", transform_id, mutation_hash);
        state.processed.insert(key.clone());
        info!(
            "✅ Item {} marked as processed with key: {}",
            transform_id, key
        );
        info!("📋 Updated processed set: {:?}", state.processed);
        Ok(())
    }

    /// Check if an item has been processed
    pub fn is_processed(
        &self,
        transform_id: &str,
        mutation_hash: &str,
    ) -> Result<bool, SchemaError> {
        let state = self.state.lock().map_err(|e| {
            error!("❌ Failed to acquire queue lock for processed check: {}", e);
            SchemaError::InvalidData("Failed to acquire queue lock".to_string())
        })?;

        let key = format!("{}|{}", transform_id, mutation_hash);
        let processed = state.processed.contains(&key);
        info!("🔍 Item {} processed status: {}", transform_id, processed);
        Ok(processed)
    }

    /// List all queued transform IDs without dequeuing
    pub fn list_queued_transforms(&self) -> Result<Vec<String>, SchemaError> {
        let state = self.state.lock().map_err(|e| {
            error!(
                "❌ Failed to acquire queue lock for listing transforms: {}",
                e
            );
            SchemaError::InvalidData("Failed to acquire queue lock".to_string())
        })?;

        let transform_ids: Vec<String> = state.queue.iter().map(|item| item.id.clone()).collect();
        Ok(transform_ids)
    }

    /// Get a copy of the current queue state for persistence
    pub fn get_state(&self) -> Result<QueueState, SchemaError> {
        let state = self.state.lock().map_err(|e| {
            error!("❌ Failed to acquire queue lock for state access: {}", e);
            SchemaError::InvalidData("Failed to acquire queue lock".to_string())
        })?;
        Ok(state.clone())
    }

    /// Update the queue state (used when loading from persistence)
    pub fn set_state(&self, new_state: QueueState) -> Result<(), SchemaError> {
        let mut state = self.state.lock().map_err(|e| {
            error!("❌ Failed to acquire queue lock for state update: {}", e);
            SchemaError::InvalidData("Failed to acquire queue lock".to_string())
        })?;
        *state = new_state;
        info!("✅ Queue state updated successfully");
        Ok(())
    }

    /// Remove items from queue that match the criteria
    pub fn remove_items<F>(&self, predicate: F) -> Result<usize, SchemaError>
    where
        F: Fn(&QueueItem) -> bool,
    {
        let mut state = self.state.lock().map_err(|e| {
            error!("❌ Failed to acquire queue lock for item removal: {}", e);
            SchemaError::InvalidData("Failed to acquire queue lock".to_string())
        })?;

        let items_before = state.queue.len();
        state.queue.retain(|item| !predicate(item));
        let items_after = state.queue.len();
        let removed_count = items_before - items_after;

        info!(
            "📋 Removed {} items from queue ({} -> {})",
            removed_count, items_before, items_after
        );

        Ok(removed_count)
    }
}
