//! # Event Monitor: System-wide Observability
//!
//! Provides centralized event monitoring and logging for the entire FoldDB system.
//! Demonstrates how event-driven architecture enables comprehensive observability
//! with a single component that can see all system activity.

use super::backfill_tracker::{BackfillTracker, BackfillInfo};
use super::message_bus::{
    atom_events::{AtomCreated, AtomUpdated, FieldValueSet, MoleculeCreated, MoleculeUpdated},
    query_events::{MutationExecuted, QueryExecuted},
    schema_events::{SchemaChanged, SchemaLoaded, TransformExecuted, TransformTriggered, TransformRegistered, TransformRegistrationRequest, SchemaApproved},
    Consumer, MessageBus,
};
use crate::transform::manager::TransformManager;
use log::info;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Statistics about system activity tracked by the event monitor
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EventStatistics {
    pub field_value_sets: u64,
    pub atom_creations: u64,
    pub atom_updates: u64,
    pub molecule_creations: u64,
    pub molecule_updates: u64,
    pub schema_loads: u64,
    pub schema_changes: u64,
    pub transform_triggers: u64,
    pub transform_executions: u64,
    pub transform_successes: u64,
    pub transform_failures: u64,
    pub transform_registrations: u64,
    pub query_executions: u64,
    pub mutation_executions: u64,
    pub total_events: u64,
    pub monitoring_start_time: u64,
    /// Track execution times for performance monitoring
    pub transform_execution_times: Vec<(String, u64)>, // (transform_id, execution_time_ms)
    /// Track success/failure rates per transform
    pub transform_stats: std::collections::HashMap<String, TransformStats>,
    /// Track query performance by schema and type
    pub query_stats: std::collections::HashMap<String, QueryStats>,
    /// Track mutation performance by schema and operation
    pub mutation_stats: std::collections::HashMap<String, MutationStats>,
}

/// Statistics for individual transforms
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TransformStats {
    pub executions: u64,
    pub successes: u64,
    pub failures: u64,
    pub total_execution_time_ms: u64,
    pub avg_execution_time_ms: f64,
    pub last_execution_time: u64,
}

/// Statistics for query operations
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct QueryStats {
    pub executions: u64,
    pub total_execution_time_ms: u64,
    pub avg_execution_time_ms: f64,
    pub total_results: usize,
    pub avg_result_count: f64,
    pub last_execution_time: u64,
}

/// Statistics for mutation operations
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MutationStats {
    pub executions: u64,
    pub total_execution_time_ms: u64,
    pub avg_execution_time_ms: f64,
    pub total_fields_affected: usize,
    pub avg_fields_affected: f64,
    pub last_execution_time: u64,
}

impl EventStatistics {
    fn increment_field_value_sets(&mut self) {
        self.field_value_sets += 1;
        self.total_events += 1;
    }

    fn increment_atom_creations(&mut self) {
        self.atom_creations += 1;
        self.total_events += 1;
    }

    fn increment_atom_updates(&mut self) {
        self.atom_updates += 1;
        self.total_events += 1;
    }

    fn increment_molecule_creations(&mut self) {
        self.molecule_creations += 1;
        self.total_events += 1;
    }

    fn increment_molecule_updates(&mut self) {
        self.molecule_updates += 1;
        self.total_events += 1;
    }

    fn increment_schema_loads(&mut self) {
        self.schema_loads += 1;
        self.total_events += 1;
    }

    fn increment_schema_changes(&mut self) {
        self.schema_changes += 1;
        self.total_events += 1;
    }

    fn increment_transform_triggers(&mut self) {
        self.transform_triggers += 1;
        self.total_events += 1;
    }

    fn increment_transform_registrations(&mut self) {
        self.transform_registrations += 1;
        self.total_events += 1;
    }

    fn increment_transform_executions(
        &mut self,
        transform_id: &str,
        success: bool,
        execution_time_ms: u64,
    ) {
        self.transform_executions += 1;
        self.total_events += 1;

        if success {
            self.transform_successes += 1;
        } else {
            self.transform_failures += 1;
        }

        // Track execution time
        self.transform_execution_times
            .push((transform_id.to_string(), execution_time_ms));

        // Update per-transform statistics
        let stats = self
            .transform_stats
            .entry(transform_id.to_string())
            .or_default();
        stats.executions += 1;
        if success {
            stats.successes += 1;
        } else {
            stats.failures += 1;
        }
        stats.total_execution_time_ms += execution_time_ms;
        stats.avg_execution_time_ms =
            stats.total_execution_time_ms as f64 / stats.executions as f64;
        stats.last_execution_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    fn increment_query_executions(
        &mut self,
        schema: &str,
        query_type: &str,
        execution_time_ms: u64,
        result_count: usize,
    ) {
        self.query_executions += 1;
        self.total_events += 1;

        // Update per-schema query statistics
        let key = format!("{}:{}", schema, query_type);
        let stats = self.query_stats.entry(key).or_default();
        stats.executions += 1;
        stats.total_execution_time_ms += execution_time_ms;
        stats.avg_execution_time_ms =
            stats.total_execution_time_ms as f64 / stats.executions as f64;
        stats.total_results += result_count;
        stats.avg_result_count = stats.total_results as f64 / stats.executions as f64;
        stats.last_execution_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    fn increment_mutation_executions(
        &mut self,
        event: &MutationExecuted,
    ) {
        self.mutation_executions += 1;
        self.total_events += 1;
        let schema = event.schema.clone();
        let operation = event.operation.clone();
        let execution_time_ms = event.execution_time_ms;
        let fields_affected = event.fields_affected.len();
        // Update per-schema mutation statistics
        let key = format!("{}:{}", schema, operation);
        let stats = self.mutation_stats.entry(key).or_default();
        stats.executions += 1;
        stats.total_execution_time_ms += execution_time_ms;
        stats.avg_execution_time_ms =
            stats.total_execution_time_ms as f64 / stats.executions as f64;
        stats.total_fields_affected += fields_affected;
        stats.avg_fields_affected = stats.total_fields_affected as f64 / stats.executions as f64;
        stats.last_execution_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Get overall transform performance metrics
    pub fn get_transform_performance_summary(&self) -> (f64, f64, u64, u64) {
        let overall_success_rate = if self.transform_executions > 0 {
            self.transform_successes as f64 / self.transform_executions as f64
        } else {
            0.0
        };

        let overall_avg_time = if !self.transform_execution_times.is_empty() {
            let total_time: u64 = self
                .transform_execution_times
                .iter()
                .map(|(_, time)| *time)
                .sum();
            total_time as f64 / self.transform_execution_times.len() as f64
        } else {
            0.0
        };

        (
            overall_success_rate,
            overall_avg_time,
            self.transform_successes,
            self.transform_failures,
        )
    }
}

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

        // Create consumers for all event types
        let mut field_value_consumer = message_bus.subscribe::<FieldValueSet>();
        let mut atom_created_consumer = message_bus.subscribe::<AtomCreated>();
        let mut atom_updated_consumer = message_bus.subscribe::<AtomUpdated>();
        let mut molecule_created_consumer = message_bus.subscribe::<MoleculeCreated>();
        let mut molecule_updated_consumer = message_bus.subscribe::<MoleculeUpdated>();
        let mut schema_loaded_consumer = message_bus.subscribe::<SchemaLoaded>();
        let mut schema_changed_consumer = message_bus.subscribe::<SchemaChanged>();
        let mut transform_triggered_consumer = message_bus.subscribe::<TransformTriggered>();
        let mut transform_executed_consumer = message_bus.subscribe::<TransformExecuted>();
        let mut transform_registered_consumer = message_bus.subscribe::<TransformRegistered>();
        let mut transform_registration_consumer = message_bus.subscribe::<TransformRegistrationRequest>();
        let mut schema_approved_consumer = message_bus.subscribe::<SchemaApproved>();
        let mut query_executed_consumer = message_bus.subscribe::<QueryExecuted>();
        let mut mutation_executed_consumer = message_bus.subscribe::<MutationExecuted>();
        let mut backfill_expected_consumer = message_bus.subscribe::<crate::fold_db_core::infrastructure::message_bus::events::request_events::BackfillExpectedMutations>();

        // Start monitoring threads for each event type
        let stats_clone = statistics.clone();
        let field_value_thread = thread::spawn(move || {
            Self::monitor_field_value_events(&mut field_value_consumer, stats_clone);
        });

        let stats_clone = statistics.clone();
        let atom_created_thread = thread::spawn(move || {
            Self::monitor_atom_created_events(&mut atom_created_consumer, stats_clone);
        });

        let stats_clone = statistics.clone();
        let atom_updated_thread = thread::spawn(move || {
            Self::monitor_atom_updated_events(&mut atom_updated_consumer, stats_clone);
        });

        let stats_clone = statistics.clone();
        let molecule_created_thread = thread::spawn(move || {
            Self::monitor_molecule_created_events(&mut molecule_created_consumer, stats_clone);
        });

        let stats_clone = statistics.clone();
        let molecule_updated_thread = thread::spawn(move || {
            Self::monitor_molecule_updated_events(&mut molecule_updated_consumer, stats_clone);
        });

        let stats_clone = statistics.clone();
        let schema_loaded_thread = thread::spawn(move || {
            Self::monitor_schema_loaded_events(&mut schema_loaded_consumer, stats_clone);
        });

        let stats_clone = statistics.clone();
        let schema_changed_thread = thread::spawn(move || {
            Self::monitor_schema_changed_events(&mut schema_changed_consumer, stats_clone);
        });

        let stats_clone = statistics.clone();
        let transform_triggered_thread = thread::spawn(move || {
            Self::monitor_transform_triggered_events(
                &mut transform_triggered_consumer,
                stats_clone,
            );
        });

        let stats_clone = statistics.clone();
        let transform_executed_thread = thread::spawn(move || {
            Self::monitor_transform_executed_events(&mut transform_executed_consumer, stats_clone);
        });

        let stats_clone = statistics.clone();
        let transform_registered_thread = thread::spawn(move || {
            Self::monitor_transform_registered_events(&mut transform_registered_consumer, stats_clone);
        });

        let stats_clone = statistics.clone();
        let transform_manager_clone = Arc::clone(&transform_manager);
        let transform_registration_thread = thread::spawn(move || {
            Self::monitor_transform_registration_events(&mut transform_registration_consumer, stats_clone, transform_manager_clone);
        });

        let stats_clone = statistics.clone();
        let query_executed_thread = thread::spawn(move || {
            Self::monitor_query_executed_events(&mut query_executed_consumer, stats_clone);
        });

        let stats_clone = statistics.clone();
        let backfill_tracker_clone = Arc::clone(&backfill_tracker);
        let transform_manager_clone = Arc::clone(&transform_manager);
        let schema_approved_thread = thread::spawn(move || {
            Self::monitor_schema_approved_events(&mut schema_approved_consumer, stats_clone, backfill_tracker_clone, transform_manager_clone);
        });

        let stats_clone = statistics.clone();
        let backfill_tracker_clone = Arc::clone(&backfill_tracker);
        let mutation_executed_thread = thread::spawn(move || {
            Self::monitor_mutation_executed_events(&mut mutation_executed_consumer, stats_clone, backfill_tracker_clone);
        });

        // Monitor BackfillExpectedMutations to set expected counts per backfill hash
        let backfill_tracker_clone = Arc::clone(&backfill_tracker);
        let backfill_expected_thread = thread::spawn(move || {
            loop {
                match backfill_expected_consumer.recv_timeout(Duration::from_millis(100)) {
                    Ok(event) => {
                        backfill_tracker_clone.set_mutations_expected(&event.backfill_hash, event.count);
                    }
                    Err(_) => continue,
                }
            }
        });

        // Periodic cleanup of old completed backfills
        let backfill_tracker_clone = Arc::clone(&backfill_tracker);
        let backfill_cleanup_thread = thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(3600)); // Run every hour
                backfill_tracker_clone.cleanup_old_backfills(100); // Keep last 100 completed backfills
            }
        });

        // Monitor BackfillMutationFailed to track failures
        let mut backfill_failed_consumer = message_bus.subscribe::<crate::fold_db_core::infrastructure::message_bus::request_events::BackfillMutationFailed>();
        let backfill_tracker_clone = Arc::clone(&backfill_tracker);
        let backfill_failed_thread = thread::spawn(move || {
            loop {
                match backfill_failed_consumer.recv_timeout(Duration::from_millis(100)) {
                    Ok(event) => {
                        backfill_tracker_clone.increment_mutation_failed(&event.backfill_hash, event.error);
                    }
                    Err(_) => continue,
                }
            }
        });

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

    fn monitor_field_value_events(
        consumer: &mut Consumer<FieldValueSet>,
        statistics: Arc<Mutex<EventStatistics>>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!(
                        "🔍 EventMonitor: FieldValueSet - field: {}, source: {}",
                        event.field, event.source
                    );
                    statistics.lock().unwrap().increment_field_value_sets();
                }
                Err(_) => continue, // Timeout or disconnect
            }
        }
    }

    fn monitor_atom_created_events(
        consumer: &mut Consumer<AtomCreated>,
        statistics: Arc<Mutex<EventStatistics>>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!("🔍 EventMonitor: AtomCreated - atom_id: {}", event.atom_id);
                    statistics.lock().unwrap().increment_atom_creations();
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_atom_updated_events(
        consumer: &mut Consumer<AtomUpdated>,
        statistics: Arc<Mutex<EventStatistics>>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!("🔍 EventMonitor: AtomUpdated - atom_id: {}", event.atom_id);
                    statistics.lock().unwrap().increment_atom_updates();
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_molecule_created_events(
        consumer: &mut Consumer<MoleculeCreated>,
        statistics: Arc<Mutex<EventStatistics>>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!(
                        "🔍 EventMonitor: MoleculeCreated - molecule_uuid: {}, type: {}, field_path: {}",
                        event.molecule_uuid, event.molecule_type, event.field_path
                    );
                    statistics.lock().unwrap().increment_molecule_creations();
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_molecule_updated_events(
        consumer: &mut Consumer<MoleculeUpdated>,
        statistics: Arc<Mutex<EventStatistics>>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!(
                        "🔍 EventMonitor: MoleculeUpdated - molecule_uuid: {}, operation: {}, field_path: {}",
                        event.molecule_uuid, event.operation, event.field_path
                    );
                    statistics.lock().unwrap().increment_molecule_updates();
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_schema_loaded_events(
        consumer: &mut Consumer<SchemaLoaded>,
        statistics: Arc<Mutex<EventStatistics>>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!(
                        "🔍 EventMonitor: SchemaLoaded - schema: {}, status: {}",
                        event.schema_name, event.status
                    );
                    statistics.lock().unwrap().increment_schema_loads();
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_schema_changed_events(
        consumer: &mut Consumer<SchemaChanged>,
        statistics: Arc<Mutex<EventStatistics>>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!("🔍 EventMonitor: SchemaChanged - schema: {}", event.schema);
                    statistics.lock().unwrap().increment_schema_changes();
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_transform_triggered_events(
        consumer: &mut Consumer<TransformTriggered>,
        statistics: Arc<Mutex<EventStatistics>>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!(
                        "🔍 EventMonitor: TransformTriggered - transform_id: {}",
                        event.transform_id
                    );
                    statistics.lock().unwrap().increment_transform_triggers();
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_transform_executed_events(
        consumer: &mut Consumer<TransformExecuted>,
        statistics: Arc<Mutex<EventStatistics>>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!(
                        "🔍 EventMonitor: TransformExecuted - transform_id: {}, result: {}",
                        event.transform_id, event.result
                    );

                    // Determine success from result field
                    let is_error = event.result.contains("error:") || 
                                  event.result.contains("execution_error:");
                    let success = !is_error;

                    // Note: Execution time tracking would require adding timing info to TransformExecuted event
                    statistics.lock().unwrap().increment_transform_executions(
                        &event.transform_id,
                        success,
                        0, // No timing info available in event
                    );
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_transform_registered_events(
        consumer: &mut Consumer<TransformRegistered>,
        statistics: Arc<Mutex<EventStatistics>>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!(
                        "🔍 EventMonitor: TransformRegistered - transform_id: {}, source_schema: {}",
                        event.transform_id, event.source_schema_name
                    );

                    // Update statistics
                    statistics.lock().unwrap().increment_transform_registrations();

                    // NOTE: Backfill is now triggered when schema is approved, not when transform is registered
                    info!(
                        "ℹ️  Transform '{}' registered. Backfill will run when schema is approved.",
                        event.transform_id
                    );
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_schema_approved_events(
        consumer: &mut Consumer<SchemaApproved>,
        _statistics: Arc<Mutex<EventStatistics>>,
        backfill_tracker: Arc<BackfillTracker>,
        transform_manager: Arc<TransformManager>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!(
                        "🔍 EventMonitor: SchemaApproved - schema_name: {}",
                        event.schema_name
                    );

                    // Check if this schema has a registered transform (i.e., it's a transform/derived schema)
                    match transform_manager.transform_exists(&event.schema_name) {
                        Ok(true) => {
                            info!(
                                "✅ Schema '{}' has a registered transform, triggering backfill",
                                event.schema_name
                            );

                            // Get the transform to find the source schema name
                            match transform_manager.list_transforms() {
                                Ok(transforms) => {
                                    if let Some(transform) = transforms.get(&event.schema_name) {
                                        // Extract source schema name from the transform's input fields
                                        match transform.get_declarative_schema() {
                                            Some(schema) => {
                                                let inputs = schema.get_inputs();
                                                if let Some(first_input) = inputs.first() {
                                                    if let Some(source_schema_name) = first_input.split('.').next() {
                                                        // Get backfill hash from event - it must be present for transform schemas
                                                        let backfill_hash = match event.backfill_hash.as_ref() {
                                                            Some(hash) => {
                                                                backfill_tracker.start_backfill_with_hash(
                                                                    hash.clone(),
                                                                    event.schema_name.clone(),
                                                                    source_schema_name.to_string(),
                                                                );
                                                                hash.clone()
                                                            }
                                                            None => {
                                                                log::error!("SchemaApproved event for transform '{}' missing required backfill_hash", event.schema_name);
                                                                return;
                                                            }
                                                        };

                                                        // Handle the transform backfill with the backfill_hash
                                                        if let Err(e) = Self::handle_transform_backfill(
                                                            &event.schema_name,
                                                            source_schema_name,
                                                            &transform_manager,
                                                            &backfill_tracker,
                                                            &backfill_hash,
                                                        ) {
                                                            log::error!("Failed to handle transform backfill for '{}': {}", event.schema_name, e);
                                                            backfill_tracker.fail_backfill(&event.schema_name, e.to_string());
                                                        }
                                                    } else {
                                                        log::error!("Failed to extract source schema from input field: {}", first_input);
                                                    }
                                                } else {
                                                    log::warn!("Transform '{}' has no input fields, skipping backfill", event.schema_name);
                                                }
                                            }
                                            None => {
                                                log::error!("Transform '{}' has no declarative schema", event.schema_name);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("Failed to list transforms: {}", e);
                                }
                            }
                        }
                        Ok(false) => {
                            info!(
                                "ℹ️  Schema '{}' has no registered transform, no backfill needed",
                                event.schema_name
                            );
                        }
                        Err(e) => {
                            log::error!("Failed to check if transform exists for '{}': {}", event.schema_name, e);
                        }
                    }
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_transform_registration_events(
        consumer: &mut Consumer<TransformRegistrationRequest>,
        _statistics: Arc<Mutex<EventStatistics>>,
        transform_manager: Arc<TransformManager>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    // Handle the transform registration
                    if let Err(e) = transform_manager.handle_transform_registration(&event.registration) {
                        log::error!("Failed to handle transform registration: {}", e);
                    }
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_query_executed_events(
        consumer: &mut Consumer<QueryExecuted>,
        statistics: Arc<Mutex<EventStatistics>>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    info!(
                        "🔍 EventMonitor: QueryExecuted - schema: {}, query_type: {}, execution_time: {}ms, results: {}",
                        event.schema, event.query_type, event.execution_time_ms, event.result_count
                    );
                    statistics.lock().unwrap().increment_query_executions(
                        &event.schema,
                        &event.query_type,
                        event.execution_time_ms,
                        event.result_count,
                    );
                }
                Err(_) => continue,
            }
        }
    }

    fn monitor_mutation_executed_events(
        consumer: &mut Consumer<MutationExecuted>,
        statistics: Arc<Mutex<EventStatistics>>,
        backfill_tracker: Arc<BackfillTracker>,
    ) {
        loop {
            match consumer.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    statistics.lock().unwrap().increment_mutation_executions(&event);
                    
                    // Check if this mutation is part of a backfill
                    if let Some(context) = &event.mutation_context {
                        if let Some(backfill_hash) = &context.backfill_hash {
                            // Increment completed mutation count for this backfill
                            let _is_complete = backfill_tracker.increment_mutation_completed(backfill_hash);
                        }
                    }
                }
                Err(_) => continue,
            }
        }
    }

    /// Handle transform backfill by fetching all entries from the source schema
    /// and executing the transform for each key-value pair
    fn handle_transform_backfill(
        transform_id: &str,
        _source_schema_name: &str,
        transform_manager: &Arc<TransformManager>,
        backfill_tracker: &Arc<BackfillTracker>,
        backfill_hash: &str,
    ) -> Result<(), crate::schema::SchemaError> {
        use crate::transform::manager::types::TransformRunner;

        // Create mutation context with backfill_hash for tracking
        let mutation_context = Some(crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
            key_value: None,
            mutation_hash: None,
            incremental: false, // Full backfill, not incremental
            backfill_hash: Some(backfill_hash.to_string()),
        });

        // Execute the transform with backfill context
        match transform_manager.execute_transform_with_context(transform_id, &mutation_context) {
            Ok(_result) => {
                Ok(())
            }
            Err(e) => {
                backfill_tracker.fail_backfill(transform_id, e.to_string());
                Err(e)
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
        let db_ops = Arc::new(crate::db_operations::DbOperations::new(db).unwrap());
        let bus_arc = Arc::new(bus);
        let transform_manager = Arc::new(crate::transform::manager::TransformManager::new(db_ops, Arc::clone(&bus_arc)).unwrap());
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

    #[test]
    fn test_event_monitor_statistics() {
        let mut stats = EventStatistics::default();

        stats.increment_field_value_sets();
        stats.increment_atom_creations();
        stats.increment_schema_loads();

        assert_eq!(stats.field_value_sets, 1);
        assert_eq!(stats.atom_creations, 1);
        assert_eq!(stats.schema_loads, 1);
        assert_eq!(stats.total_events, 3);
    }
}
