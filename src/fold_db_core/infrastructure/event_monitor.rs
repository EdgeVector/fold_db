//! # Event Monitor: System-wide Observability
//!
//! Provides centralized event monitoring and logging for the entire FoldDB system.
//! Demonstrates how event-driven architecture enables comprehensive observability
//! with a single component that can see all system activity.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use log::info;

use crate::transform::manager::TransformManager;

use super::backfill_tracker::{BackfillInfo, BackfillTracker};
use crate::progress::ProgressStore;
// Re-export for public API
pub use super::event_statistics::{EventStatistics, MutationStats, QueryStats, TransformStats};
use super::message_bus::{AsyncMessageBus, Event};
use super::schema_approval_handler::handle_schema_approved;

/// Centralized event monitor that provides system-wide observability
pub struct EventMonitor {
    statistics: Arc<Mutex<EventStatistics>>,
    backfill_tracker: Arc<BackfillTracker>,
}

impl EventMonitor {
    /// Create a new EventMonitor that subscribes to all event types
    pub async fn new(
        message_bus: Arc<AsyncMessageBus>,
        transform_manager: Arc<TransformManager>,
        progress_store: Option<Arc<dyn ProgressStore>>,
        user_id: String,
    ) -> Self {
        let statistics = Arc::new(Mutex::new(EventStatistics {
            monitoring_start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ..Default::default()
        }));

        let backfill_tracker = Arc::new(BackfillTracker::new(progress_store, user_id));

        info!("🔍 EventMonitor: Starting system-wide event monitoring");

        // Helper to spawn monitoring tasks
        // We can't easily genericize the extraction of event variant, so we'll do it per event or use a macro
        // For clarity, we'll write them out or use a simple closure pattern where the closure checks the variant.

        // FieldValueSet
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("FieldValueSet").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::FieldValueSet(_) = event {
                    stats.lock().unwrap().increment_field_value_sets();
                }
            }
        });

        // AtomCreated
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("AtomCreated").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::AtomCreated(_) = event {
                    stats.lock().unwrap().increment_atom_creations();
                }
            }
        });

        // AtomUpdated
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("AtomUpdated").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::AtomUpdated(_) = event {
                    stats.lock().unwrap().increment_atom_updates();
                }
            }
        });

        // MoleculeCreated
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("MoleculeCreated").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::MoleculeCreated(_) = event {
                    stats.lock().unwrap().increment_molecule_creations();
                }
            }
        });

        // MoleculeUpdated
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("MoleculeUpdated").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::MoleculeUpdated(_) = event {
                    stats.lock().unwrap().increment_molecule_updates();
                }
            }
        });

        // SchemaLoaded
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("SchemaLoaded").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::SchemaLoaded(_) = event {
                    stats.lock().unwrap().increment_schema_loads();
                }
            }
        });

        // SchemaChanged
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("SchemaChanged").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::SchemaChanged(_) = event {
                    stats.lock().unwrap().increment_schema_changes();
                }
            }
        });

        // TransformTriggered
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("TransformTriggered").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::TransformTriggered(_) = event {
                    stats.lock().unwrap().increment_transform_triggers();
                }
            }
        });

        // TransformExecuted
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("TransformExecuted").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::TransformExecuted(e) = event {
                    let is_error =
                        e.result.contains("error:") || e.result.contains("execution_error:");
                    let success = !is_error;
                    stats.lock().unwrap().increment_transform_executions(
                        &e.transform_id,
                        success,
                        0,
                    );
                }
            }
        });

        // TransformRegistered
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("TransformRegistered").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::TransformRegistered(_) = event {
                    stats.lock().unwrap().increment_transform_registrations();
                }
            }
        });

        // TransformRegistrationRequest
        let mut rx = message_bus.subscribe("TransformRegistrationRequest").await;
        tokio::spawn(async move {
            while (rx.recv().await).is_some() {
                // Nothing to do but consume
            }
        });

        // SchemaApproved
        let backfill_tracker_clone = Arc::clone(&backfill_tracker);
        let transform_manager_clone = Arc::clone(&transform_manager);
        let mut rx = message_bus.subscribe("SchemaApproved").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::SchemaApproved(e) = event {
                    if let Err(err) =
                        handle_schema_approved(e, &backfill_tracker_clone, &transform_manager_clone)
                            .await
                    {
                        log::error!("Failed to handle schema approval: {}", err);
                    }
                }
            }
        });

        // QueryExecuted
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("QueryExecuted").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::QueryExecuted(e) = event {
                    stats.lock().unwrap().increment_query_executions(
                        &e.schema,
                        &e.query_type,
                        e.execution_time_ms,
                        e.result_count,
                    );
                }
            }
        });

        // MutationExecuted
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("MutationExecuted").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::MutationExecuted(e) = event {
                    stats.lock().unwrap().increment_mutation_executions(&e);
                }
            }
        });

        Self {
            statistics,
            backfill_tracker,
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
    use crate::fold_db_core::infrastructure::message_bus::atom_events::{
        AtomCreated, FieldValueSet, MoleculeCreated,
    };
    use crate::fold_db_core::infrastructure::message_bus::schema_events::SchemaLoaded;
    use crate::fold_db_core::infrastructure::message_bus::AsyncMessageBus;
    use serde_json::json;
    use std::time::Duration;

    #[tokio::test]
    async fn test_event_monitor_observability() {
        let bus = AsyncMessageBus::new();
        let bus_arc = Arc::new(bus);

        // Create a dummy TransformManager for testing
        let db = sled::Config::new().temporary(true).open().unwrap();
        let db_ops = Arc::new(
            crate::db_operations::DbOperations::from_sled(db)
                .await
                .unwrap(),
        );
        let transform_manager = Arc::new(
            crate::transform::manager::TransformManager::new(db_ops, Arc::clone(&bus_arc))
                .await
                .unwrap(),
        );
        let monitor = EventMonitor::new(
            Arc::clone(&bus_arc),
            transform_manager,
            None,
            "test_user".to_string(),
        )
        .await;

        // Publish various events
        bus_arc
            .publish_event(Event::FieldValueSet(FieldValueSet::new(
                "test.field",
                json!("value"),
                "test",
            )))
            .await
            .unwrap();
        bus_arc
            .publish_event(Event::AtomCreated(AtomCreated::new(
                "atom-123",
                json!({"test": "data"}),
            )))
            .await
            .unwrap();
        bus_arc
            .publish_event(Event::MoleculeCreated(MoleculeCreated::new(
                "molecule-456",
                "Collection",
                "schema.field",
            )))
            .await
            .unwrap();
        bus_arc
            .publish_event(Event::SchemaLoaded(SchemaLoaded::new(
                "TestSchema",
                "success",
            )))
            .await
            .unwrap();

        // Allow time for event processing
        tokio::time::sleep(Duration::from_millis(200)).await;

        let stats = monitor.get_statistics();
        // Since we check stats async, they should be updated
        // Note: FieldValueSet might trigger AtomCreated depending on flags, but here we publish directly.
        // The stats increment logic is direct.

        // Wait, why total_events >= 4? Because we published 4.
        // Let's check individual.
        assert!(stats.field_value_sets >= 1);
        assert!(stats.atom_creations >= 1);
        assert!(stats.molecule_creations >= 1);
        assert!(stats.schema_loads >= 1);

        monitor.log_summary();
    }
}
