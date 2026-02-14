use std::collections::HashMap;
use std::sync::Arc;

use log::{debug, error, info, warn};
use serde_json::Value;

use crate::db_operations::{native_index::NativeIndexManager, DbOperations};
use crate::fold_db_core::infrastructure::message_bus::{
    query_events::MutationExecuted, AsyncMessageBus, Event,
};
use crate::fold_db_core::orchestration::index_status::IndexStatusTracker;

/// Orchestrator for Native Indexing operations.
///
/// Handles field-name indexing only. Keyword extraction is performed inline
/// during ingestion (see `ingestion_service.rs`) and written atomically
/// with mutations (see `mutation_manager.rs`).
pub struct IndexOrchestrator {
    db_ops: Arc<DbOperations>,
    index_status_tracker: Option<IndexStatusTracker>,
    pending_tasks:
        Arc<crate::fold_db_core::infrastructure::pending_task_tracker::PendingTaskTracker>,
}

impl IndexOrchestrator {
    /// Create a new IndexOrchestrator
    pub fn new(
        db_ops: Arc<DbOperations>,
        index_status_tracker: Option<IndexStatusTracker>,
        pending_tasks: Arc<
            crate::fold_db_core::infrastructure::pending_task_tracker::PendingTaskTracker,
        >,
    ) -> Self {
        Self {
            db_ops,
            index_status_tracker,
            pending_tasks,
        }
    }

    /// Start listening for mutation events to trigger indexing
    pub async fn start_event_listener(&self, message_bus: Arc<AsyncMessageBus>) {
        info!("IndexOrchestrator: Starting event listener task");

        let db_ops = Arc::clone(&self.db_ops);
        let tracker = self.index_status_tracker.clone();
        let mut consumer = message_bus.subscribe("MutationExecuted").await;

        let pending_tasks = self.pending_tasks.clone();

        // Spawn a task to listen for MutationExecuted events
        tokio::spawn(async move {
            loop {
                match consumer.recv().await {
                    Some(Event::MutationExecuted(event)) => {
                        // Handle the event asynchronously
                        if let Some(data) = &event.data {
                            if data.is_empty() {
                                continue;
                            }

                            debug!(
                                "IndexOrchestrator: Received mutation event with {} rows, user_id: {:?}",
                                data.len(),
                                event.user_id
                            );

                            // Track the task
                            pending_tasks.increment();

                            // Process indexing within user context (critical for multi-tenant DynamoDB writes)
                            if let Some(ref user_id) = event.user_id {
                                crate::logging::core::run_with_user(user_id, async {
                                    Self::process_indexing(&db_ops, &tracker, &event).await;
                                })
                                .await;
                            } else {
                                // No user context - process anyway (will use default user_id)
                                warn!("IndexOrchestrator: No user_id in event, using default");
                                Self::process_indexing(&db_ops, &tracker, &event).await;
                            }

                            // Task completed
                            pending_tasks.decrement();
                        }
                    }
                    Some(_) => {} // Ignore other events
                    None => {
                        error!("IndexOrchestrator: Message bus disconnected");
                        break;
                    }
                }
            }
        });
    }

    /// Process field-name indexing for a batch of data.
    ///
    /// Keyword indexing is handled inline during ingestion — this method
    /// only indexes field names for mutations that weren't already indexed.
    async fn process_indexing(
        db_ops: &Arc<DbOperations>,
        _tracker: &Option<IndexStatusTracker>,
        event: &MutationExecuted,
    ) {
        if event.already_indexed {
            debug!(
                "IndexOrchestrator: Skipping '{}' — already indexed inline",
                event.schema
            );
            return;
        }

        let Some(native_index_mgr) = db_ops.native_index_manager() else {
            return;
        };

        let schema_name = &event.schema;
        let data = event.data.as_ref().unwrap();

        // Extract key_value from mutation_context
        let key_value = if let Some(ctx) = &event.mutation_context {
            if let Some(kv) = &ctx.key_value {
                kv.clone()
            } else {
                warn!(
                    "IndexOrchestrator: No key_value in mutation context for schema {}",
                    schema_name
                );
                return;
            }
        } else {
            warn!(
                "IndexOrchestrator: No mutation context for schema {}",
                schema_name
            );
            return;
        };

        // Merge all rows into a single HashMap
        let mut merged_fields: HashMap<String, Value> = HashMap::new();
        for row in data {
            for (field_name, value) in row {
                if NativeIndexManager::should_index_field(field_name) {
                    merged_fields.insert(field_name.clone(), value.clone());
                }
            }
        }

        if merged_fields.is_empty() {
            return;
        }

        // Extract molecule version numbers from the event
        let mol_versions = event.molecule_versions.as_ref();

        // Index field names only (no LLM — keyword extraction is handled inline during ingestion)
        let field_names: Vec<String> = merged_fields.keys().cloned().collect();
        if let Err(e) = native_index_mgr
            .batch_index_field_names(schema_name, &key_value, &field_names, mol_versions)
            .await
        {
            error!("IndexOrchestrator: Field-name indexing failed: {}", e);
        }

        let _ = native_index_mgr.flush().await;
    }
}
