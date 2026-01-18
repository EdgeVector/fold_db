//! Persistence management component for the Transform Orchestrator
//!
//! Handles state persistence logic using storage abstraction,
//! extracted from the main TransformOrchestrator for better separation of concerns.

use super::queue_manager::QueueState;
use crate::schema::SchemaError;
use crate::storage::traits::KvStore;
use log::{error, info};
use sled::Tree;
use std::sync::Arc;

/// Manages persistence operations for queue state
pub struct PersistenceManager {
    tree: Option<Tree>,
    store: Option<Arc<dyn KvStore>>,
}

impl Clone for PersistenceManager {
    fn clone(&self) -> Self {
        Self {
            tree: None, // Tree is not cloneable, but we only need store for async operations
            store: self.store.clone(),
        }
    }
}

impl PersistenceManager {
    /// Create a new PersistenceManager with the given sled tree (backward compatible)
    pub fn new(tree: Tree) -> Self {
        Self {
            tree: Some(tree),
            store: None,
        }
    }

    /// Create a new PersistenceManager with KvStore (for DynamoDB and other backends)
    pub fn new_with_store(store: Arc<dyn KvStore>) -> Self {
        Self {
            tree: None,
            store: Some(store),
        }
    }

    /// Check if this manager uses async storage (DynamoDB) vs sync (Sled)
    pub fn is_async(&self) -> bool {
        self.store.is_some()
    }

    /// Save the current queue state to persistent storage (sync version for Sled)
    pub fn save_state(&self, state: &QueueState) -> Result<(), SchemaError> {
        if let Some(ref tree) = self.tree {
            info!("💾 SAVE_STATE START - saving orchestrator state to disk");

            info!(
                "📋 Current state to persist - queue length: {}, queued count: {}, processed count: {}",
                state.queue.len(),
                state.queued.len(),
                state.processed.len()
            );
            info!("📋 Queue items: {:?}", state.queue);
            info!("📋 Queued set: {:?}", state.queued);
            info!("📋 Processed set: {:?}", state.processed);

            // Use consistent serialization pattern from SerializationHelper
            let state_bytes = serde_json::to_vec(state).map_err(|e| {
                let error_msg = format!("Failed to serialize orchestrator state: {}", e);
                error!("❌ {}", error_msg);
                SchemaError::InvalidData(error_msg)
            })?;

            info!(
                "💾 Inserting state into tree (size: {} bytes)",
                state_bytes.len()
            );
            tree.insert("state", state_bytes).map_err(|e| {
                error!("❌ Failed to insert orchestrator state into tree: {}", e);
                SchemaError::InvalidData(format!("Failed to persist orchestrator state: {}", e))
            })?;

            info!("✅ SAVE_STATE COMPLETE - state saved successfully");
            Ok(())
        } else {
            Err(SchemaError::InvalidData("Synchronous save_state only available with Sled backend. Use save_state_async instead.".to_string()))
        }
    }

    /// Save the current queue state to persistent storage (async version for DynamoDB)
    pub async fn save_state_async(&self, state: &QueueState) -> Result<(), SchemaError> {
        if let Some(ref store) = self.store {
            info!("💾 SAVE_STATE START - saving orchestrator state to storage");

            info!(
                "📋 Current state to persist - queue length: {}, queued count: {}, processed count: {}",
                state.queue.len(),
                state.queued.len(),
                state.processed.len()
            );
            info!("📋 Queue items: {:?}", state.queue);
            info!("📋 Queued set: {:?}", state.queued);
            info!("📋 Processed set: {:?}", state.processed);

            // Use consistent serialization pattern from SerializationHelper
            let state_bytes = serde_json::to_vec(state).map_err(|e| {
                let error_msg = format!("Failed to serialize orchestrator state: {}", e);
                error!("❌ {}", error_msg);
                SchemaError::InvalidData(error_msg)
            })?;

            info!(
                "💾 Inserting state into store (size: {} bytes)",
                state_bytes.len()
            );
            store
                .put("state".as_bytes(), state_bytes)
                .await
                .map_err(|e| {
                    error!("❌ Failed to insert orchestrator state into store: {}", e);
                    SchemaError::InvalidData(format!("Failed to persist orchestrator state: {}", e))
                })?;

            info!("✅ SAVE_STATE COMPLETE - state saved successfully");
            Ok(())
        } else {
            Err(SchemaError::InvalidData(
                "Async save_state only available with KvStore backend".to_string(),
            ))
        }
    }

    /// Load queue state from persistent storage (sync version for Sled)
    pub fn load_state(&self) -> Result<QueueState, SchemaError> {
        if let Some(ref tree) = self.tree {
            info!("📖 LOAD_STATE START - loading orchestrator state from disk");

            let state = tree
                .get("state")
                .map_err(|e| {
                    error!("❌ Failed to get state from tree: {}", e);
                    SchemaError::InvalidData(format!("Failed to load state: {}", e))
                })?
                .map(|v| serde_json::from_slice::<QueueState>(&v))
                .transpose()
                .map_err(|e| {
                    let error_msg = format!("Failed to deserialize orchestrator state: {}", e);
                    error!("❌ {}", error_msg);
                    SchemaError::InvalidData(error_msg)
                })?
                .unwrap_or_else(|| {
                    info!("📋 No existing state found, creating new empty state");
                    QueueState::default()
                });

            info!("📖 LOAD_STATE COMPLETE - loaded state with queue length: {}, queued count: {}, processed count: {}",
                state.queue.len(), state.queued.len(), state.processed.len());
            info!("📋 Loaded queue items: {:?}", state.queue);
            info!("📋 Loaded queued set: {:?}", state.queued);
            info!("📋 Loaded processed set: {:?}", state.processed);

            Ok(state)
        } else {
            Err(SchemaError::InvalidData("Synchronous load_state only available with Sled backend. Use load_state_async instead.".to_string()))
        }
    }

    /// Load queue state from persistent storage (async version for DynamoDB)
    pub async fn load_state_async(&self) -> Result<QueueState, SchemaError> {
        if let Some(ref store) = self.store {
            info!("📖 LOAD_STATE START - loading orchestrator state from storage");

            let bytes = store.get("state".as_bytes()).await.map_err(|e| {
                error!("❌ Failed to get state from store: {}", e);
                SchemaError::InvalidData(format!("Failed to load state: {}", e))
            })?;

            let state = bytes
                .map(|v| serde_json::from_slice::<QueueState>(&v))
                .transpose()
                .map_err(|e| {
                    let error_msg = format!("Failed to deserialize orchestrator state: {}", e);
                    error!("❌ {}", error_msg);
                    SchemaError::InvalidData(error_msg)
                })?
                .unwrap_or_else(|| {
                    info!("📋 No existing state found, creating new empty state");
                    QueueState::default()
                });

            info!("📖 LOAD_STATE COMPLETE - loaded state with queue length: {}, queued count: {}, processed count: {}",
                state.queue.len(), state.queued.len(), state.processed.len());
            info!("📋 Loaded queue items: {:?}", state.queue);
            info!("📋 Loaded queued set: {:?}", state.queued);
            info!("📋 Loaded processed set: {:?}", state.processed);

            Ok(state)
        } else {
            Err(SchemaError::InvalidData(
                "Async load_state only available with KvStore backend".to_string(),
            ))
        }
    }

    /// Flush changes to disk to ensure persistence (sync version for Sled)
    pub fn flush(&self) -> Result<(), SchemaError> {
        if let Some(ref tree) = self.tree {
            info!("💾 Flushing tree to disk");
            tree.flush().map_err(|e| {
                error!("❌ Failed to flush orchestrator state to disk: {}", e);
                SchemaError::InvalidData(format!("Failed to flush orchestrator state: {}", e))
            })?;

            info!("✅ Tree flushed successfully");
            Ok(())
        } else {
            // For async stores (DynamoDB), flush is a no-op
            Ok(())
        }
    }

    /// Flush changes to storage (async version for DynamoDB)
    pub async fn flush_async(&self) -> Result<(), SchemaError> {
        if let Some(ref store) = self.store {
            store.flush().await.map_err(|e| {
                error!("❌ Failed to flush orchestrator state: {}", e);
                SchemaError::InvalidData(format!("Failed to flush orchestrator state: {}", e))
            })?;
            Ok(())
        } else {
            // For sync stores (Sled), use sync flush
            if let Some(ref tree) = self.tree {
                tree.flush().map_err(|e| {
                    error!("❌ Failed to flush orchestrator state to disk: {}", e);
                    SchemaError::InvalidData(format!("Failed to flush orchestrator state: {}", e))
                })?;
            }
            Ok(())
        }
    }

    /// Save state and immediately flush to disk for guaranteed persistence (sync version)
    pub fn save_and_flush(&self, state: &QueueState) -> Result<(), SchemaError> {
        self.save_state(state)?;
        self.flush()?;
        Ok(())
    }

    /// Save state and immediately flush (async version)
    pub async fn save_and_flush_async(&self, state: &QueueState) -> Result<(), SchemaError> {
        self.save_state_async(state).await?;
        self.flush_async().await?;
        Ok(())
    }

    /// Check if state exists in persistent storage (sync version)
    pub fn state_exists(&self) -> Result<bool, SchemaError> {
        if let Some(ref tree) = self.tree {
            let exists = tree
                .get("state")
                .map_err(|e| {
                    error!("❌ Failed to check state existence: {}", e);
                    SchemaError::InvalidData(format!("Failed to check state existence: {}", e))
                })?
                .is_some();

            info!("🔍 State exists in storage: {}", exists);
            Ok(exists)
        } else {
            Err(SchemaError::InvalidData("Synchronous state_exists only available with Sled backend. Use state_exists_async instead.".to_string()))
        }
    }

    /// Check if state exists in persistent storage (async version)
    pub async fn state_exists_async(&self) -> Result<bool, SchemaError> {
        if let Some(ref store) = self.store {
            let exists = store.exists("state".as_bytes()).await.map_err(|e| {
                error!("❌ Failed to check state existence: {}", e);
                SchemaError::InvalidData(format!("Failed to check state existence: {}", e))
            })?;

            info!("🔍 State exists in storage: {}", exists);
            Ok(exists)
        } else {
            self.state_exists()
        }
    }

    /// Clear all persistent state (useful for testing or reset operations) - sync version
    pub fn clear_state(&self) -> Result<(), SchemaError> {
        if let Some(ref tree) = self.tree {
            info!("🗑️ Clearing persistent state");

            tree.remove("state").map_err(|e| {
                error!("❌ Failed to clear state: {}", e);
                SchemaError::InvalidData(format!("Failed to clear state: {}", e))
            })?;

            self.flush()?;
            info!("✅ State cleared successfully");
            Ok(())
        } else {
            Err(SchemaError::InvalidData("Synchronous clear_state only available with Sled backend. Use clear_state_async instead.".to_string()))
        }
    }

    /// Clear all persistent state (async version)
    pub async fn clear_state_async(&self) -> Result<(), SchemaError> {
        if let Some(ref store) = self.store {
            info!("🗑️ Clearing persistent state");

            store.delete("state".as_bytes()).await.map_err(|e| {
                error!("❌ Failed to clear state: {}", e);
                SchemaError::InvalidData(format!("Failed to clear state: {}", e))
            })?;

            self.flush_async().await?;
            info!("✅ State cleared successfully");
            Ok(())
        } else {
            self.clear_state()
        }
    }

    /// Get the underlying tree for advanced operations (use carefully) - only for Sled
    pub fn get_tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_db_core::orchestration::queue_manager::QueueItem;

    fn create_test_tree() -> Tree {
        crate::testing_utils::TestDatabaseFactory::create_named_test_tree("test_persistence")
    }

    #[test]
    fn test_save_and_load_state() {
        let tree = create_test_tree();
        let manager = PersistenceManager::new(tree);

        // Create test state
        let mut test_state = QueueState::default();
        test_state.queue.push_back(QueueItem {
            id: "test_transform".to_string(),
            mutation_hash: "test_hash".to_string(),
        });
        test_state
            .queued
            .insert("test_transform|test_hash".to_string());
        test_state
            .processed
            .insert("processed_transform|processed_hash".to_string());

        // Save state
        manager.save_state(&test_state).unwrap();
        manager.flush().unwrap();

        // Load state
        let loaded_state = manager.load_state().unwrap();

        // Verify state matches
        assert_eq!(loaded_state.queue.len(), 1);
        assert_eq!(loaded_state.queued.len(), 1);
        assert_eq!(loaded_state.processed.len(), 1);
        assert_eq!(loaded_state.queue[0].id, "test_transform");
        assert!(loaded_state.queued.contains("test_transform|test_hash"));
        assert!(loaded_state
            .processed
            .contains("processed_transform|processed_hash"));
    }

    #[test]
    fn test_state_exists() {
        let tree = create_test_tree();
        let manager = PersistenceManager::new(tree);

        // Initially no state
        assert!(!manager.state_exists().unwrap());

        // Save state
        let state = QueueState::default();
        manager.save_state(&state).unwrap();

        // Now state exists
        assert!(manager.state_exists().unwrap());
    }

    #[test]
    fn test_clear_state() {
        let tree = create_test_tree();
        let manager = PersistenceManager::new(tree);

        // Save state
        let state = QueueState::default();
        manager.save_and_flush(&state).unwrap();
        assert!(manager.state_exists().unwrap());

        // Clear state
        manager.clear_state().unwrap();
        assert!(!manager.state_exists().unwrap());
    }
}
