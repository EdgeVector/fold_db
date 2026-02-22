use std::sync::Arc;

use log::{debug, error, info, warn};

use crate::db_operations::{native_index::BatchIndexOperation, DbOperations};
use crate::fold_db_core::infrastructure::message_bus::{
    query_events::MutationExecuted, AsyncMessageBus, Event,
};
use crate::fold_db_core::orchestration::index_status::IndexStatusTracker;

/// Orchestrator for Native Indexing operations
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
        info!("🔎 IndexOrchestrator: Starting event listener task");

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
                                "🔎 IndexOrchestrator: Received mutation event with {} rows, user_id: {:?}",
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
                                warn!("🔎 IndexOrchestrator: No user_id in event, using default");
                                Self::process_indexing(&db_ops, &tracker, &event).await;
                            }

                            // Task completed
                            pending_tasks.decrement();
                        }
                    }
                    Some(_) => {} // Ignore other events
                    None => {
                        error!("❌ IndexOrchestrator: Message bus disconnected");
                        break;
                    }
                }
            }
        });
    }

    /// Process indexing for a batch of data
    async fn process_indexing(
        db_ops: &Arc<DbOperations>,
        tracker: &Option<IndexStatusTracker>,
        event: &MutationExecuted,
    ) {
        let Some(native_index_mgr) = db_ops.native_index_manager() else {
            return;
        };

        let schema_name = &event.schema;
        let data = event.data.as_ref().unwrap();

        // extract key_value from mutation_context
        // We assume 1:1 mapping between data[0] and mutation_context.key_value
        // If batching evolves, this logic needs update to match data rows to keys.
        let key_value = if let Some(ctx) = &event.mutation_context {
            if let Some(kv) = &ctx.key_value {
                kv.clone()
            } else {
                // Fallback if no key value (should not happen for mutations usually)
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

        // Construct operations
        let mut index_operations: Vec<BatchIndexOperation> = Vec::new();

        // For now, only process the first row in data, as we have 1 context per event
        if let Some(row) = data.first() {
            for (field_name, value) in row {
                // Filter excluded fields (uuid, id, password, token)
                if should_index_field(field_name) {
                    // We pass None for classifications to let NativeIndexManager default it (usually "word")
                    index_operations.push((
                        schema_name.clone(),
                        field_name.clone(),
                        key_value.clone(),
                        value.clone(),
                        None,
                    ));
                }
            }
        }

        if index_operations.is_empty() {
            return;
        }

        // Update tracker
        if let Some(idx_tracker) = tracker {
            idx_tracker.start_batch(index_operations.len()).await;
        }

        let start = std::time::Instant::now();

        // Execute Batch Indexing using append-only method (optimized: no read-modify-write)
        let result = native_index_mgr
            .batch_index_append_only(&index_operations)
            .await;

        if let Err(e) = result {
            error!("❌ IndexOrchestrator: Batch indexing failed: {}", e);
        }

        // Complete tracker
        if let Some(idx_tracker) = tracker {
            idx_tracker
                .complete_batch(index_operations.len(), start.elapsed().as_millis())
                .await;
        }

        // Flush for sync backends (Sled)
        if !native_index_mgr.is_async() {
            let _ = native_index_mgr.flush();
        }
    }
}

fn should_index_field(field_name: &str) -> bool {
    let excluded = ["uuid", "id", "password", "token"];
    !excluded.iter().any(|e| e.eq_ignore_ascii_case(field_name))
}
