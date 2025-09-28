//! Type definitions for execution engine core
//!
//! Contains all data structures, result types, and enums used in the
//! execution engine for processing field expressions and managing execution context.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::field::FieldValue;

/// Context for executing a set of field expressions
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Input data to process
    pub input_data: HashMap<String, HashMap<KeyValue, FieldValue>>,
    /// Maximum depth for emission
    pub emission_depth: usize,
    /// Additional context variables
    pub variables: HashMap<String, Value>,
}

/// Result of executing field expressions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Generated index entries
    pub index_entries: HashMap<String, Vec<IndexEntry>>,
    /// Any warnings generated during execution
    pub warnings: HashMap<String, Vec<ExecutionWarning>>,
}

/// A single row value entry produced by the execution engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Deterministic identifier for the logical row
    pub row_id: String,
    /// Evaluated value for the current field
    pub value: Value,
    /// Unique identifier for the atom (kept for compatibility/traceability)
    pub atom_uuid: String,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
    /// Field expression that generated this entry
    pub expression: String,
}

/// Statistics about execution performance
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionStatistics {
    /// Total number of index entries generated
    pub total_entries: usize,
    /// Number of items per depth level
    pub items_per_depth: HashMap<usize, usize>,
    /// Estimated memory usage in bytes
    pub memory_usage_bytes: usize,
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of cache misses
    pub cache_misses: usize,
}

/// Warning generated during execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionWarning {
    /// Type of warning
    pub warning_type: ExecutionWarningType,
    /// Warning message
    pub message: String,
    /// Field that generated the warning (if applicable)
    pub field: Option<String>,
}

/// Types of execution warnings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionWarningType {
    /// Performance degradation warning
    PerformanceDegradation,
    /// Memory usage warning
    MemoryUsage,
    /// Data quality warning
    DataQuality,
    /// Configuration warning
    Configuration,
}
