//! Statistics tracking for event monitoring
//!
//! This module contains all statistics-related types and their implementations.

use std::time::{SystemTime, UNIX_EPOCH};

/// Statistics about system activity tracked by the event monitor
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EventStatistics {
    pub field_value_sets: u64,
    pub atom_creations: u64,
    pub molecule_creations: u64,
    pub query_executions: u64,
    pub mutation_executions: u64,
    pub total_events: u64,
    pub monitoring_start_time: u64,
    /// Track query performance by schema and type
    pub query_stats: std::collections::HashMap<String, QueryStats>,
    /// Track mutation performance by schema and operation
    pub mutation_stats: std::collections::HashMap<String, MutationStats>,
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
    pub fn increment_field_value_sets(&mut self) {
        self.field_value_sets += 1;
        self.total_events += 1;
    }

    pub fn increment_atom_creations(&mut self) {
        self.atom_creations += 1;
        self.total_events += 1;
    }

    pub fn increment_molecule_creations(&mut self) {
        self.molecule_creations += 1;
        self.total_events += 1;
    }

    pub fn increment_query_executions(
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

    pub fn increment_mutation_executions(
        &mut self,
        event: &crate::messaging::query_events::MutationExecuted,
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
}
