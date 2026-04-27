//! # Event Monitor: System-wide Observability
//!
//! Provides centralized event monitoring and logging for the entire FoldDB system.
//! Demonstrates how event-driven architecture enables comprehensive observability
//! with a single component that can see all system activity.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::info;

pub use super::event_statistics::{EventStatistics, MutationStats, QueryStats};
use crate::messaging::{AsyncMessageBus, Event};

/// Centralized event monitor that provides system-wide observability
pub struct EventMonitor {
    statistics: Arc<Mutex<EventStatistics>>,
}

impl EventMonitor {
    /// Create a new EventMonitor that subscribes to all event types
    pub async fn new(message_bus: Arc<AsyncMessageBus>) -> Self {
        let statistics = Arc::new(Mutex::new(EventStatistics {
            monitoring_start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ..Default::default()
        }));

        info!("🔍 EventMonitor: Starting system-wide event monitoring");

        // FieldValueSet
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("FieldValueSet").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::FieldValueSet(_) = event {
                    stats
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .increment_field_value_sets();
                }
            }
        });

        // AtomCreated
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("AtomCreated").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::AtomCreated(_) = event {
                    stats
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .increment_atom_creations();
                }
            }
        });

        // MoleculeCreated
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("MoleculeCreated").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::MoleculeCreated(_) = event {
                    stats
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .increment_molecule_creations();
                }
            }
        });

        // QueryExecuted
        let stats = statistics.clone();
        let mut rx = message_bus.subscribe("QueryExecuted").await;
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Event::QueryExecuted(e) = event {
                    stats
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .increment_query_executions(
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
                    stats
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .increment_mutation_executions(&e);
                }
            }
        });

        Self { statistics }
    }

    /// Get current event statistics
    pub fn get_statistics(&self) -> EventStatistics {
        self.statistics
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
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
        info!("  🎯 Molecule Creations: {}", stats.molecule_creations);
        info!("  🔍 Query Executions: {}", stats.query_executions);
        info!("  🔧 Mutation Executions: {}", stats.mutation_executions);
        info!("  📈 Total Events: {}", stats.total_events);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::atom_events::{AtomCreated, FieldValueSet, MoleculeCreated};
    use crate::messaging::AsyncMessageBus;
    use serde_json::json;
    use std::time::Duration;

    #[tokio::test]
    async fn test_event_monitor_observability() {
        let bus = AsyncMessageBus::new();
        let bus_arc = Arc::new(bus);

        let monitor = EventMonitor::new(Arc::clone(&bus_arc)).await;

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

        // Allow time for event processing
        tokio::time::sleep(Duration::from_millis(200)).await;

        let stats = monitor.get_statistics();

        assert!(stats.field_value_sets >= 1);
        assert!(stats.atom_creations >= 1);
        assert!(stats.molecule_creations >= 1);

        monitor.log_summary();
    }
}
