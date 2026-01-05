use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::fold_db_core::infrastructure::backfill_tracker::BackfillTracker;
use crate::fold_db_core::infrastructure::message_bus::{
    query_events::MutationExecuted,
    request_events::{BackfillExpectedMutations, BackfillMutationFailed},
    MessageBus,
};
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
    pub fn start_event_listener(&self, message_bus: Arc<MessageBus>) {
        info!("⏳ BackfillManager: Starting event listener threads");

        // Helper to spawn event monitor threads
        fn spawn_event_handler<T, F>(
            mut consumer: crate::fold_db_core::infrastructure::message_bus::Consumer<T>,
            mut handler: F,
        ) -> thread::JoinHandle<()>
        where
            T: crate::fold_db_core::infrastructure::message_bus::EventType,
            F: FnMut(T) + Send + 'static,
        {
            thread::spawn(move || loop {
                match consumer.recv_timeout(Duration::from_millis(100)) {
                    Ok(event) => handler(event),
                    Err(_) => continue,
                }
            })
        }

        // 1. BackfillExpectedMutations
        let tracker_clone = Arc::clone(&self.backfill_tracker);
        let _expected_thread = spawn_event_handler(
            message_bus.subscribe::<BackfillExpectedMutations>(),
            move |event: BackfillExpectedMutations| {
                tracker_clone.set_mutations_expected(&event.backfill_hash, event.count);
            },
        );

        // 2. BackfillMutationFailed
        let tracker_clone = Arc::clone(&self.backfill_tracker);
        let _failed_thread = spawn_event_handler(
            message_bus.subscribe::<BackfillMutationFailed>(),
            move |event: BackfillMutationFailed| {
                tracker_clone.increment_mutation_failed(&event.backfill_hash, event.error);
            },
        );

        // 3. MutationExecuted (for progress tracking)
        let tracker_clone = Arc::clone(&self.backfill_tracker);
        let _mutation_thread = spawn_event_handler(
            message_bus.subscribe::<MutationExecuted>(),
            move |event: MutationExecuted| {
                if let Some(context) = &event.mutation_context {
                    if let Some(backfill_hash) = &context.backfill_hash {
                        let _is_complete =
                            tracker_clone.increment_mutation_completed(backfill_hash);
                    }
                }
            },
        );

        // 4. Periodic Cleanup (self-managed)
        let tracker_clone = Arc::clone(&self.backfill_tracker);
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(3600)); // Run every hour
                tracker_clone.cleanup_old_backfills(100); // Keep last 100 completed
            }
        });
    }
}
