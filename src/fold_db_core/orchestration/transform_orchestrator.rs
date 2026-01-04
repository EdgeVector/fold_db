//! Refactored Transform Orchestrator using component delegation
//!
//! This orchestrator now coordinates between specialized components rather than
//! handling all operations directly, resulting in better separation of concerns
//! and improved maintainability.

use log::{error, info};
use sled::Tree;
use std::sync::Arc;

use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::schema::SchemaError;
use crate::storage::traits::KvStore;
use crate::transform::manager::{types::TransformResult, TransformManager};

// Import the new specialized components
use super::execution_coordinator::ExecutionCoordinator;
use super::persistence_manager::PersistenceManager;
use super::queue_manager::QueueManager;
use super::transform_event_monitor::TransformEventMonitor;

use async_trait::async_trait;

/// Trait for adding transforms to a queue
#[async_trait]
pub trait TransformQueue {
    async fn add_task(
        &self,
        schema_name: &str,
        field_name: &str,
        mutation_hash: &str,
    ) -> Result<(), SchemaError>;
    async fn add_transform(
        &self,
        transform_id: &str,
        mutation_hash: &str,
    ) -> Result<(), SchemaError>;
}

/// Orchestrates execution of transforms sequentially using specialized components.
///
/// This refactored version delegates operations to focused components:
/// - QueueManager: Thread-safe queue operations
/// - PersistenceManager: State persistence
/// - TransformEventMonitor: Field value event monitoring
/// - ExecutionCoordinator: Transform execution and result publishing
pub struct TransformOrchestrator {
    queue_manager: QueueManager,
    persistence_manager: PersistenceManager,
    execution_coordinator: ExecutionCoordinator,
    _event_monitor: TransformEventMonitor, // Kept alive for background monitoring
}

impl TransformOrchestrator {
    /// Create a new TransformOrchestrator with component delegation (Sled backend)
    pub fn new(
        manager: Arc<TransformManager>,
        tree: Tree,
        message_bus: Arc<MessageBus>,
        db_ops: Arc<DbOperations>,
    ) -> Self {
        info!("🏗️ Creating TransformOrchestrator with component delegation (Sled)");

        // Initialize persistence manager
        let persistence_manager = PersistenceManager::new(tree.clone());

        // Load initial state or create empty state
        let initial_state = persistence_manager.load_state().unwrap_or_else(|e| {
            error!("❌ Failed to load initial state, using empty state: {}", e);
            super::queue_manager::QueueState::default()
        });

        info!(
            "📋 Loaded initial state - queue length: {}, queued count: {}, processed count: {}",
            initial_state.queue.len(),
            initial_state.queued.len(),
            initial_state.processed.len()
        );

        // Initialize queue manager with loaded state
        let queue_manager = QueueManager::new(initial_state);

        // Initialize execution coordinator
        let execution_coordinator = ExecutionCoordinator::new(
            Arc::clone(&manager),
            Arc::clone(&message_bus),
            Arc::clone(&db_ops),
        );

        // Initialize event monitor (starts background monitoring)
        let event_monitor = TransformEventMonitor::new(
            Arc::clone(&message_bus),
            Arc::clone(&manager),
            PersistenceManager::new(tree.clone()),
        );

        info!("✅ TransformOrchestrator initialized with all components");

        Self {
            queue_manager,
            persistence_manager,
            execution_coordinator,
            _event_monitor: event_monitor,
        }
    }

    /// Create a new TransformOrchestrator with KvStore (for DynamoDB and other backends)
    pub async fn new_with_store(
        manager: Arc<TransformManager>,
        store: Arc<dyn KvStore>,
        message_bus: Arc<MessageBus>,
        db_ops: Arc<DbOperations>,
    ) -> Result<Self, SchemaError> {
        info!("🏗️ Creating TransformOrchestrator with component delegation (KvStore)");

        // Initialize persistence manager
        let persistence_manager = PersistenceManager::new_with_store(store.clone());

        // Load initial state or create empty state
        let initial_state = persistence_manager
            .load_state_async()
            .await
            .unwrap_or_else(|e| {
                error!("❌ Failed to load initial state, using empty state: {}", e);
                super::queue_manager::QueueState::default()
            });

        info!(
            "📋 Loaded initial state - queue length: {}, queued count: {}, processed count: {}",
            initial_state.queue.len(),
            initial_state.queued.len(),
            initial_state.processed.len()
        );

        // Initialize queue manager with loaded state
        let queue_manager = QueueManager::new(initial_state);

        // Initialize execution coordinator
        let execution_coordinator = ExecutionCoordinator::new(
            Arc::clone(&manager),
            Arc::clone(&message_bus),
            Arc::clone(&db_ops),
        );

        // Initialize event monitor (starts background monitoring)
        let event_monitor = TransformEventMonitor::new(
            Arc::clone(&message_bus),
            Arc::clone(&manager),
            PersistenceManager::new_with_store(store.clone()),
        );

        info!("✅ TransformOrchestrator initialized with all components");

        Ok(Self {
            queue_manager,
            persistence_manager,
            execution_coordinator,
            _event_monitor: event_monitor,
        })
    }

    /// Add a task for the given schema and field using the execution coordinator
    pub async fn add_task(
        &self,
        schema_name: &str,
        field_name: &str,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        info!(
            "📋 ADD_TASK - Adding task for {}.{}",
            schema_name, field_name
        );

        // Use execution coordinator to get transforms for the field
        let manager = self.execution_coordinator.get_manager();
        let transform_ids = manager.get_transforms_for_field(schema_name, field_name)?;

        info!(
            "🔍 Found {} transforms for {}.{}: {:?}",
            transform_ids.len(),
            schema_name,
            field_name,
            transform_ids
        );

        if transform_ids.is_empty() {
            info!("ℹ️ No transforms found for {}.{}", schema_name, field_name);
            return Ok(());
        }

        // Add each transform to the queue
        for transform_id in transform_ids {
            self.queue_manager.add_item(&transform_id, mutation_hash)?;
        }

        // Persist the updated state (non-blocking for async stores)
        if let Err(e) = self.persist_current_state_async().await {
            error!("⚠️ Failed to persist state: {}", e);
        }

        info!("✅ ADD_TASK completed for {}.{}", schema_name, field_name);
        Ok(())
    }

    /// Add a transform directly to the queue by ID
    pub async fn add_transform(
        &self,
        transform_id: &str,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        info!("🚀 ADD_TRANSFORM - Adding transform: {}", transform_id);

        // Add to queue
        let added = self.queue_manager.add_item(transform_id, mutation_hash)?;

        if added {
            info!("✅ Transform {} added to queue", transform_id);
        } else {
            info!("ℹ️ Transform {} already in queue", transform_id);
        }

        // Persist state (non-blocking for async stores)
        if let Err(e) = self.persist_current_state_async().await {
            error!("⚠️ Failed to persist state: {}", e);
        }

        // Process queue immediately after adding
        info!(
            "🔄 Triggering automatic queue processing for: {}",
            transform_id
        );
        self.process_queue().await;

        info!("🏁 ADD_TRANSFORM completed for: {}", transform_id);
        Ok(())
    }

    /// Process a single task from the queue
    pub async fn process_one(&self) -> Option<Result<TransformResult, SchemaError>> {
        info!("🔄 PROCESS_ONE - Checking queue for items");

        // Pop item from queue
        let item = match self.queue_manager.pop_item() {
            Ok(Some(item)) => item,
            Ok(None) => {
                info!("📭 Queue is empty");
                return None;
            }
            Err(e) => {
                error!("❌ Failed to pop item from queue: {}", e);
                return Some(Err(e));
            }
        };

        // Check if already processed
        let already_processed = match self
            .queue_manager
            .is_processed(&item.id, &item.mutation_hash)
        {
            Ok(processed) => processed,
            Err(e) => {
                error!("❌ Failed to check processed status: {}", e);
                return Some(Err(e));
            }
        };

        // Persist state before execution
        if let Err(e) = self.persist_current_state_async().await {
            error!("❌ Failed to persist state before execution: {}", e);
            return Some(Err(e));
        }

        // Execute transform using execution coordinator
        let result = self
            .execution_coordinator
            .execute_transform(&item, already_processed)
            .await;

        // Mark as processed if execution succeeded
        if result.is_ok() {
            if let Err(e) = self
                .queue_manager
                .mark_processed(&item.id, &item.mutation_hash)
            {
                error!("❌ Failed to mark transform as processed: {}", e);
                return Some(Err(e));
            }

            // Persist state after successful processing
            if let Err(e) = self.persist_current_state_async().await {
                error!("❌ Failed to persist state after processing: {}", e);
                return Some(Err(e));
            }
        }

        info!("🏁 PROCESS_ONE completed for: {}", item.id);
        Some(result)
    }

    /// Process all queued tasks sequentially
    pub async fn process_queue(&self) {
        info!("🔄 PROCESS_QUEUE - Starting to process all queued transforms");

        let initial_length = match self.len() {
            Ok(length) => {
                info!("📊 Initial queue length: {}", length);
                length
            }
            Err(e) => {
                error!("❌ Failed to get initial queue length: {}", e);
                return;
            }
        };

        if initial_length == 0 {
            info!("📭 Queue is empty, nothing to process");
            return;
        }

        let mut processed_count = 0;
        let mut iteration_count = 0;

        loop {
            iteration_count += 1;
            info!("🔄 Processing iteration #{}", iteration_count);

            match self.process_one().await {
                Some(result) => {
                    processed_count += 1;
                    match result {
                        Ok(value) => {
                            info!(
                                "✅ Successfully processed transform #{}: {:?}",
                                processed_count, value
                            );
                        }
                        Err(e) => {
                            error!(
                                "❌ Failed to process transform #{}: {:?}",
                                processed_count, e
                            );
                        }
                    }
                }
                None => {
                    info!(
                        "📭 No more items in queue after iteration #{}",
                        iteration_count
                    );
                    break;
                }
            }

            // Safety check to prevent infinite loops
            if iteration_count > 100 {
                error!(
                    "❌ Breaking out of process_queue loop after {} iterations",
                    iteration_count
                );
                break;
            }
        }

        let final_length = self.len().unwrap_or(0);
        info!(
            "🏁 PROCESS_QUEUE completed - processed {} transforms, final queue length: {}",
            processed_count, final_length
        );
    }

    /// Persist current state (async version for DynamoDB)
    pub async fn persist_current_state_async(&self) -> Result<(), SchemaError> {
        let current_state = self.queue_manager.get_state()?;
        if self.persistence_manager.is_async() {
            self.persistence_manager
                .save_and_flush_async(&current_state)
                .await
        } else {
            self.persistence_manager.save_and_flush(&current_state)
        }
    }

    /// List queued transform IDs without dequeuing or running them
    pub fn list_queued_transforms(&self) -> Result<Vec<String>, SchemaError> {
        self.queue_manager.list_queued_transforms()
    }

    /// Queue length, useful for tests
    pub fn len(&self) -> Result<usize, SchemaError> {
        self.queue_manager.len()
    }

    /// Returns true if the queue is empty
    pub fn is_empty(&self) -> Result<bool, SchemaError> {
        self.queue_manager.is_empty()
    }

    /// Get access to individual components for advanced operations
    pub fn get_queue_manager(&self) -> &QueueManager {
        &self.queue_manager
    }

    pub fn get_persistence_manager(&self) -> &PersistenceManager {
        &self.persistence_manager
    }

    pub fn get_execution_coordinator(&self) -> &ExecutionCoordinator {
        &self.execution_coordinator
    }
}

#[async_trait]
impl TransformQueue for TransformOrchestrator {
    async fn add_task(
        &self,
        schema_name: &str,
        field_name: &str,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        self.add_task(schema_name, field_name, mutation_hash).await
    }

    async fn add_transform(
        &self,
        transform_id: &str,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        self.add_transform(transform_id, mutation_hash).await
    }
}
