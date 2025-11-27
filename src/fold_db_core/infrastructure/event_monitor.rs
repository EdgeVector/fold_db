//! # Event Monitor: System-wide Observability
//!
//! Provides centralized event monitoring and logging for the entire FoldDB system.
//! Demonstrates how event-driven architecture enables comprehensive observability
//! with a single component that can see all system activity.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use log::info;

use crate::transform::manager::TransformManager;

use super::backfill_tracker::{BackfillInfo, BackfillTracker};
// Re-export for public API
pub use super::event_statistics::{EventStatistics, MutationStats, QueryStats, TransformStats};
use super::message_bus::request_events::{BackfillExpectedMutations, BackfillMutationFailed};
use super::message_bus::{
    atom_events::{AtomCreated, AtomUpdated, FieldValueSet, MoleculeCreated, MoleculeUpdated},
    query_events::{MutationExecuted, QueryExecuted},
    schema_events::{SchemaApproved, SchemaChanged, SchemaLoaded, TransformExecuted, 
                     TransformRegistered, TransformRegistrationRequest, TransformTriggered},
    MessageBus,
};
use super::schema_approval_handler::handle_schema_approved;

/// Centralized event monitor that provides system-wide observability
pub struct EventMonitor {
    statistics: Arc<Mutex<EventStatistics>>,
    backfill_tracker: Arc<BackfillTracker>,
    _field_value_thread: thread::JoinHandle<()>,
    _atom_created_thread: thread::JoinHandle<()>,
    _atom_updated_thread: thread::JoinHandle<()>,
    _molecule_created_thread: thread::JoinHandle<()>,
    _molecule_updated_thread: thread::JoinHandle<()>,
    _schema_loaded_thread: thread::JoinHandle<()>,
    _schema_changed_thread: thread::JoinHandle<()>,
    _transform_triggered_thread: thread::JoinHandle<()>,
    _transform_executed_thread: thread::JoinHandle<()>,
    _transform_registered_thread: thread::JoinHandle<()>,
    _transform_registration_thread: thread::JoinHandle<()>,
    _schema_approved_thread: thread::JoinHandle<()>,
    _query_executed_thread: thread::JoinHandle<()>,
    _mutation_executed_thread: thread::JoinHandle<()>,
    _backfill_expected_thread: thread::JoinHandle<()>,
    _backfill_cleanup_thread: thread::JoinHandle<()>,
    _backfill_failed_thread: thread::JoinHandle<()>,
}

impl EventMonitor {
    /// Create a new EventMonitor that subscribes to all event types
    pub fn new(message_bus: Arc<MessageBus>, transform_manager: Arc<TransformManager>) -> Self {
        let statistics = Arc::new(Mutex::new(EventStatistics {
            monitoring_start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ..Default::default()
        }));

        let backfill_tracker = Arc::new(BackfillTracker::new());

        info!("🔍 EventMonitor: Starting system-wide event monitoring");

        // Helper function to create event monitoring threads
        fn spawn_event_monitor<T, F>(mut consumer: super::message_bus::Consumer<T>, mut handler: F) -> thread::JoinHandle<()>
        where
            T: super::message_bus::EventType,
            F: FnMut(T) + Send + 'static,
        {
            thread::spawn(move || {
                loop {
                    match consumer.recv_timeout(Duration::from_millis(100)) {
                        Ok(event) => handler(event),
                        Err(_) => continue,
                    }
                }
            })
        }

        // Start monitoring threads for each event type
        let stats_clone = statistics.clone();
        let field_value_thread = spawn_event_monitor(
            message_bus.subscribe::<FieldValueSet>(),
            move |_| stats_clone.lock().unwrap().increment_field_value_sets(),
        );

        let stats_clone = statistics.clone();
        let atom_created_thread = spawn_event_monitor(
            message_bus.subscribe::<AtomCreated>(),
            move |_| stats_clone.lock().unwrap().increment_atom_creations(),
        );

        let stats_clone = statistics.clone();
        let atom_updated_thread = spawn_event_monitor(
            message_bus.subscribe::<AtomUpdated>(),
            move |_| stats_clone.lock().unwrap().increment_atom_updates(),
        );

        let stats_clone = statistics.clone();
        let molecule_created_thread = spawn_event_monitor(
            message_bus.subscribe::<MoleculeCreated>(),
            move |_| stats_clone.lock().unwrap().increment_molecule_creations(),
        );

        let stats_clone = statistics.clone();
        let molecule_updated_thread = spawn_event_monitor(
            message_bus.subscribe::<MoleculeUpdated>(),
            move |_| stats_clone.lock().unwrap().increment_molecule_updates(),
        );

        let stats_clone = statistics.clone();
        let schema_loaded_thread = spawn_event_monitor(
            message_bus.subscribe::<SchemaLoaded>(),
            move |_| stats_clone.lock().unwrap().increment_schema_loads(),
        );

        let stats_clone = statistics.clone();
        let schema_changed_thread = spawn_event_monitor(
            message_bus.subscribe::<SchemaChanged>(),
            move |_| stats_clone.lock().unwrap().increment_schema_changes(),
        );

        let stats_clone = statistics.clone();
        let transform_triggered_thread = spawn_event_monitor(
            message_bus.subscribe::<TransformTriggered>(),
            move |_| stats_clone.lock().unwrap().increment_transform_triggers(),
        );

        let stats_clone = statistics.clone();
        let transform_executed_thread = spawn_event_monitor(
            message_bus.subscribe::<TransformExecuted>(),
            move |event: TransformExecuted| {
                let is_error = event.result.contains("error:") || 
                              event.result.contains("execution_error:");
                let success = !is_error;
                stats_clone.lock().unwrap().increment_transform_executions(
                    &event.transform_id,
                    success,
                    0,
                );
            },
        );

        let stats_clone = statistics.clone();
        let transform_registered_thread = spawn_event_monitor(
            message_bus.subscribe::<TransformRegistered>(),
            move |_| stats_clone.lock().unwrap().increment_transform_registrations(),
        );

        let transform_manager_clone = Arc::clone(&transform_manager);
        let transform_registration_thread = spawn_event_monitor(
            message_bus.subscribe::<TransformRegistrationRequest>(),
            move |event: TransformRegistrationRequest| {
                if let Err(e) = transform_manager_clone.handle_transform_registration(&event.registration) {
                    log::error!("Failed to handle transform registration: {}", e);
                }
            },
        );

        let backfill_tracker_clone = Arc::clone(&backfill_tracker);
        let transform_manager_clone = Arc::clone(&transform_manager);
        let schema_approved_thread = spawn_event_monitor(
            message_bus.subscribe::<SchemaApproved>(),
            move |event: SchemaApproved| {
                if let Err(e) = handle_schema_approved(event, &backfill_tracker_clone, &transform_manager_clone) {
                    log::error!("Failed to handle schema approval: {}", e);
                }
            },
        );

        let stats_clone = statistics.clone();
        let query_executed_thread = spawn_event_monitor(
            message_bus.subscribe::<QueryExecuted>(),
            move |event: QueryExecuted| {
                stats_clone.lock().unwrap().increment_query_executions(
                    &event.schema,
                    &event.query_type,
                    event.execution_time_ms,
                    event.result_count,
                );
            },
        );

        let stats_clone = statistics.clone();
        let backfill_tracker_clone = Arc::clone(&backfill_tracker);
        let mutation_executed_thread = spawn_event_monitor(
            message_bus.subscribe::<MutationExecuted>(),
            move |event: MutationExecuted| {
                stats_clone.lock().unwrap().increment_mutation_executions(&event);
                
                if let Some(context) = &event.mutation_context {
                    if let Some(backfill_hash) = &context.backfill_hash {
                        let _is_complete = backfill_tracker_clone.increment_mutation_completed(backfill_hash);
                    }
                }
            },
        );

        // Monitor BackfillExpectedMutations to set expected counts per backfill hash
        let backfill_tracker_clone = Arc::clone(&backfill_tracker);
        let backfill_expected_thread = spawn_event_monitor(
            message_bus.subscribe::<BackfillExpectedMutations>(),
            move |event: BackfillExpectedMutations| {
                backfill_tracker_clone.set_mutations_expected(&event.backfill_hash, event.count);
            },
        );

        // Periodic cleanup of old completed backfills
        let backfill_tracker_clone = Arc::clone(&backfill_tracker);
        let backfill_cleanup_thread = thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(3600)); // Run every hour
                backfill_tracker_clone.cleanup_old_backfills(100); // Keep last 100 completed backfills
            }
        });

        // Monitor BackfillMutationFailed to track failures
        let backfill_tracker_clone = Arc::clone(&backfill_tracker);
        let backfill_failed_thread = spawn_event_monitor(
            message_bus.subscribe::<BackfillMutationFailed>(),
            move |event: BackfillMutationFailed| {
                backfill_tracker_clone.increment_mutation_failed(&event.backfill_hash, event.error);
            },
        );

        Self {
            statistics,
            backfill_tracker,
            _field_value_thread: field_value_thread,
            _atom_created_thread: atom_created_thread,
            _atom_updated_thread: atom_updated_thread,
            _molecule_created_thread: molecule_created_thread,
            _molecule_updated_thread: molecule_updated_thread,
            _schema_loaded_thread: schema_loaded_thread,
            _schema_changed_thread: schema_changed_thread,
            _transform_triggered_thread: transform_triggered_thread,
            _transform_executed_thread: transform_executed_thread,
            _transform_registered_thread: transform_registered_thread,
            _transform_registration_thread: transform_registration_thread,
            _schema_approved_thread: schema_approved_thread,
            _query_executed_thread: query_executed_thread,
            _mutation_executed_thread: mutation_executed_thread,
            _backfill_expected_thread: backfill_expected_thread,
            _backfill_cleanup_thread: backfill_cleanup_thread,
            _backfill_failed_thread: backfill_failed_thread,
        }
    }

    /// Get current event statistics
    pub fn get_statistics(&self) -> EventStatistics {
        self.statistics.lock().unwrap().clone()
    }

    /// Get the backfill tracker
    pub fn get_backfill_tracker(&self) -> Arc<BackfillTracker> {
        Arc::clone(&self.backfill_tracker)
    }

    /// Get all backfill information
    pub fn get_all_backfills(&self) -> Vec<BackfillInfo> {
        self.backfill_tracker.get_all_backfills()
    }

    /// Get active (in-progress) backfills
    pub fn get_active_backfills(&self) -> Vec<BackfillInfo> {
        self.backfill_tracker.get_active_backfills()
    }

    /// Get specific backfill info
    pub fn get_backfill(&self, transform_id: &str) -> Option<BackfillInfo> {
        self.backfill_tracker.get_backfill(transform_id)
    }

    /// Log a summary of all activity since monitoring started
    pub fn log_summary(&self) {
        let stats = self.get_statistics();
        let runtime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - stats.monitoring_start_time;

        info!("📊 EventMonitor Summary ({}s runtime):", runtime);
        info!("  📝 Field Value Sets: {}", stats.field_value_sets);
        info!("  🆕 Atom Creations: {}", stats.atom_creations);
        info!("  🔄 Atom Updates: {}", stats.atom_updates);
        info!("  🎯 Molecule Creations: {}", stats.molecule_creations);
        info!("  ⚡ Molecule Updates: {}", stats.molecule_updates);
        info!("  📋 Schema Loads: {}", stats.schema_loads);
        info!("  🔧 Schema Changes: {}", stats.schema_changes);
        info!("  🚀 Transform Triggers: {}", stats.transform_triggers);
        info!("  ✅ Transform Executions: {}", stats.transform_executions);
        info!("  🔍 Query Executions: {}", stats.query_executions);
        info!("  🔧 Mutation Executions: {}", stats.mutation_executions);
        info!("  📈 Total Events: {}", stats.total_events);

        // Transform Performance Metrics
        if stats.transform_executions > 0 {
            let (success_rate, avg_time, successes, failures) =
                stats.get_transform_performance_summary();
            info!("  🎯 Transform Performance:");
            info!(
                "    ✅ Successes: {} ({:.1}%)",
                successes,
                success_rate * 100.0
            );
            info!("    ❌ Failures: {}", failures);
            info!("    ⏱️  Avg Execution Time: {:.2}ms", avg_time);

            // Individual transform statistics
            if !stats.transform_stats.is_empty() {
                info!("  📊 Per-Transform Statistics:");
                for (transform_id, transform_stats) in &stats.transform_stats {
                    let success_rate = if transform_stats.executions > 0 {
                        transform_stats.successes as f64 / transform_stats.executions as f64 * 100.0
                    } else {
                        0.0
                    };
                    info!(
                        "    🔧 {}: {} executions, {:.1}% success, {:.2}ms avg",
                        transform_id,
                        transform_stats.executions,
                        success_rate,
                        transform_stats.avg_execution_time_ms
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_db_core::MessageBus;
    use serde_json::json;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_event_monitor_observability() {
        let bus = MessageBus::new();
        // Create a dummy TransformManager for testing
        let db = sled::Config::new().temporary(true).open().unwrap();
        let db_ops = Arc::new(tokio::runtime::Runtime::new().unwrap().block_on(
            crate::db_operations::DbOperationsV2::from_sled(db)
        ).unwrap());
        let bus_arc = Arc::new(bus);
        let transform_manager = Arc::new(tokio::runtime::Runtime::new().unwrap().block_on(
            crate::transform::manager::TransformManager::new(db_ops, Arc::clone(&bus_arc))
        ).unwrap());
        let monitor = EventMonitor::new(Arc::clone(&bus_arc), transform_manager);

        // Publish various events
        bus_arc.publish(FieldValueSet::new("test.field", json!("value"), "test"))
            .unwrap();
        bus_arc.publish(AtomCreated::new("atom-123", json!({"test": "data"})))
            .unwrap();
        bus_arc.publish(MoleculeCreated::new(
            "molecule-456",
            "Collection",
            "schema.field",
        ))
        .unwrap();
        bus_arc.publish(SchemaLoaded::new("TestSchema", "success"))
            .unwrap();

        // Allow time for event processing
        thread::sleep(Duration::from_millis(200));

        let stats = monitor.get_statistics();
        assert!(stats.total_events >= 4);
        assert!(stats.field_value_sets >= 1);
        assert!(stats.atom_creations >= 1);
        assert!(stats.molecule_creations >= 1);
        assert!(stats.schema_loads >= 1);

        monitor.log_summary();
    }
}
