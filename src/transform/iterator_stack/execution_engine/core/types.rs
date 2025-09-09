//! Type definitions for execution engine core
//!
//! Contains all data structures, result types, and enums used in the
//! execution engine for processing field expressions and managing execution context.

use crate::transform::iterator_stack::field_alignment::FieldAlignmentInfo;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Context for executing a set of field expressions
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Input data to process
    pub input_data: Value,
    /// Field alignment information
    pub field_alignments: HashMap<String, FieldAlignmentInfo>,
    /// Maximum depth for emission
    pub emission_depth: usize,
    /// Additional context variables
    pub variables: HashMap<String, Value>,
}

/// Result of executing field expressions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Generated index entries
    pub index_entries: Vec<IndexEntry>,
    /// Execution statistics
    pub statistics: ExecutionStatistics,
    /// Any warnings generated during execution
    pub warnings: Vec<ExecutionWarning>,
}

/// A single index entry produced by the execution engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Hash field value (used for indexing)
    pub hash_value: Value,
    /// Range field value (used for sorting/filtering)
    pub range_value: Value,
    /// Unique identifier for the atom
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
