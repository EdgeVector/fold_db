use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
