use std::sync::Arc;
use std::time::Duration;

use crate::fold_db_core::infrastructure::backfill_tracker::BackfillTracker;
use crate::fold_db_core::infrastructure::message_bus::{AsyncMessageBus, Event};
use log::info;

/// Orchestrator for Backfill operations
pub struct BackfillManager {
    backfill_tracker: Arc<BackfillTracker>,
}

impl BackfillManager {
    /// Create a new BackfillManager
    pub fn new(backfill_tracker: Arc<BackfillTracker>) -> Self {
        Self { backfill_tracker }
    }

    /// Start listening for backfill-related events
    pub async fn start_event_listener(&self, message_bus: Arc<AsyncMessageBus>) {
        info!("⏳ BackfillManager: Starting event listener tasks");

        // 1. BackfillExpectedMutations
        let tracker = Arc::clone(&self.backfill_tracker);
        let mut consumer = message_bus.subscribe("BackfillExpectedMutations").await;

        tokio::spawn(async move {
            loop {
                match consumer.recv().await {
                    Some(Event::BackfillExpectedMutations(event)) => {
                        tracker.set_mutations_expected(&event.backfill_hash, event.count).await;
                    }
                    Some(_) => {}
                    None => break,
                }
            }
        });

        // 2. BackfillMutationFailed
        let tracker = Arc::clone(&self.backfill_tracker);
        let mut consumer = message_bus.subscribe("BackfillMutationFailed").await;

        tokio::spawn(async move {
            loop {
                match consumer.recv().await {
                    Some(Event::BackfillMutationFailed(event)) => {
                        tracker.increment_mutation_failed(&event.backfill_hash, event.error).await;
                    }
                    Some(_) => {}
                    None => break,
                }
            }
        });

        // 3. MutationExecuted (for progress tracking)
        let tracker = Arc::clone(&self.backfill_tracker);
        let mut consumer = message_bus.subscribe("MutationExecuted").await;

        tokio::spawn(async move {
            loop {
                match consumer.recv().await {
                    Some(Event::MutationExecuted(event)) => {
                        if let Some(context) = &event.mutation_context {
                            if let Some(backfill_hash) = &context.backfill_hash {
                                let _is_complete =
                                    tracker.increment_mutation_completed(backfill_hash).await;
                            }
                        }
                    }
                    Some(_) => {}
                    None => break,
                }
            }
        });

        // 4. Periodic Cleanup (self-managed)
        let tracker = Arc::clone(&self.backfill_tracker);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await; // Run every hour
                tracker.cleanup_old_backfills(100); // Keep last 100 completed
            }
        });
    }
}
