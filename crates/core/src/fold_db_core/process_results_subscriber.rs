//! Subscribes to MutationExecuted events and writes process results to a dedicated store.
//!
//! Each ingestion progress_id maps to a set of mutation outcomes (schema_name + actual key_value).
//! The frontend queries this store instead of relying on pre-calculated keys from the mutation.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::db_operations::DbOperations;
use crate::schema::types::KeyValue;

use crate::messaging::events::query_events::MutationExecuted;
use crate::messaging::{AsyncMessageBus, Event};

/// A single mutation outcome stored in the process_results namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMutationResult {
    pub schema_name: String,
    pub key_value: KeyValue,
}

/// Listens for MutationExecuted events and persists process results keyed by progress_id.
pub struct ProcessResultsSubscriber {
    db_ops: Arc<DbOperations>,
}

impl ProcessResultsSubscriber {
    pub fn new(db_ops: Arc<DbOperations>) -> Self {
        Self { db_ops }
    }

    /// Start listening for MutationExecuted events in a background task.
    pub async fn start_event_listener(&self, message_bus: Arc<AsyncMessageBus>, user_id: String) {
        let db_ops = Arc::clone(&self.db_ops);
        let mut consumer = message_bus.subscribe("MutationExecuted").await;

        // lint:spawn-bare-ok boot-time MutationExecuted listener — perpetual worker; per-event spans created downstream.
        tokio::spawn(async move {
            crate::user_context::run_with_user(&user_id, async move {
                while let Some(event) = consumer.recv().await {
                    if let Event::MutationExecuted(e) = event {
                        Self::handle_event(&e, &db_ops).await;
                    }
                }
                tracing::warn!("ProcessResultsSubscriber: consumer disconnected");
            })
            .await
        });
    }

    async fn handle_event(event: &MutationExecuted, db_ops: &Arc<DbOperations>) {
        // Extract progress_id from metadata
        let progress_id = match event.metadata.as_ref().and_then(|m| m.get("progress_id")) {
            Some(id) => id.clone(),
            None => return, // Not an ingestion mutation — nothing to record
        };

        // Extract mutation_id and key_value from mutation_context
        let ctx = match &event.mutation_context {
            Some(c) => c,
            None => return,
        };
        let mutation_id = match &ctx.mutation_hash {
            Some(id) => id.clone(),
            None => return,
        };
        let key_value = match ctx.key_value.clone() {
            Some(kv) => kv,
            None => return, // No key_value means we can't meaningfully record a result
        };

        let result = ProcessMutationResult {
            schema_name: event.schema.clone(),
            key_value,
        };

        // Write: key = "{progress_id}:mut:{mutation_id}"
        let key = format!("{}:mut:{}", progress_id, mutation_id);
        if let Err(e) = db_ops.metadata().put_process_result(&key, &result).await {
            tracing::error!(
                "ProcessResultsSubscriber: failed to write result for key '{}': {}",
                key,
                e
            );
        }
    }
}
