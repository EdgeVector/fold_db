use std::collections::HashMap;
use std::sync::Arc;

use log::{debug, error, info, warn};
use serde_json::Value;

use crate::db_operations::{native_index::NativeIndexManager, DbOperations};
use crate::fold_db_core::infrastructure::message_bus::{
    query_events::MutationExecuted, AsyncMessageBus, Event,
};
use crate::fold_db_core::orchestration::index_status::IndexStatusTracker;
use crate::fold_db_core::orchestration::keyword_extractor::KeywordExtractor;

/// Orchestrator for Native Indexing operations
pub struct IndexOrchestrator {
    db_ops: Arc<DbOperations>,
    index_status_tracker: Option<IndexStatusTracker>,
    pending_tasks:
        Arc<crate::fold_db_core::infrastructure::pending_task_tracker::PendingTaskTracker>,
    keyword_extractor: Option<Arc<KeywordExtractor>>,
}

impl IndexOrchestrator {
    /// Create a new IndexOrchestrator
    pub fn new(
        db_ops: Arc<DbOperations>,
        index_status_tracker: Option<IndexStatusTracker>,
        pending_tasks: Arc<
            crate::fold_db_core::infrastructure::pending_task_tracker::PendingTaskTracker,
        >,
        keyword_extractor: Option<Arc<KeywordExtractor>>,
    ) -> Self {
        Self {
            db_ops,
            index_status_tracker,
            pending_tasks,
            keyword_extractor,
        }
    }

    /// Start listening for mutation events to trigger indexing
    pub async fn start_event_listener(&self, message_bus: Arc<AsyncMessageBus>) {
        info!("IndexOrchestrator: Starting event listener task");

        let db_ops = Arc::clone(&self.db_ops);
        let tracker = self.index_status_tracker.clone();
        let mut consumer = message_bus.subscribe("MutationExecuted").await;
        let keyword_extractor = self.keyword_extractor.clone();

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
                                    Self::process_indexing(&db_ops, &tracker, &event, &keyword_extractor).await;
                                })
                                .await;
                            } else {
                                // No user context - process anyway (will use default user_id)
                                warn!("IndexOrchestrator: No user_id in event, using default");
                                Self::process_indexing(&db_ops, &tracker, &event, &keyword_extractor).await;
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

    /// Process indexing for a batch of data
    async fn process_indexing(
        db_ops: &Arc<DbOperations>,
        tracker: &Option<IndexStatusTracker>,
        event: &MutationExecuted,
        keyword_extractor: &Option<Arc<KeywordExtractor>>,
    ) {
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

        // If keyword extractor is available, use LLM-powered extraction
        if let Some(extractor) = keyword_extractor {
            // Merge all rows into a single HashMap for one LLM call
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

            // Update tracker
            if let Some(idx_tracker) = tracker {
                idx_tracker.start_batch(merged_fields.len()).await;
            }

            let start = std::time::Instant::now();

            match extractor.extract_keywords(&merged_fields).await {
                Ok(keywords) => {
                    if !keywords.is_empty() {
                        if let Err(e) = native_index_mgr
                            .batch_index_from_keywords(
                                schema_name,
                                &key_value,
                                keywords,
                            )
                            .await
                        {
                            error!("IndexOrchestrator: Keyword indexing failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("IndexOrchestrator: LLM keyword extraction failed: {}", e);
                }
            }

            // Complete tracker
            if let Some(idx_tracker) = tracker {
                idx_tracker
                    .complete_batch(merged_fields.len(), start.elapsed().as_millis())
                    .await;
            }
        } else {
            // No keyword extractor - skip indexing (LLM not configured)
            debug!("IndexOrchestrator: No keyword extractor available, skipping indexing");
            return;
        }

        // Flush for sync backends (Sled)
        if !native_index_mgr.is_async() {
            let _ = native_index_mgr.flush();
        }
    }
}
