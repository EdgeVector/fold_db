//! Type definitions for chain parser
//!
//! Contains all data structures, enums, and result types used in chain parsing
//! and compatibility analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single operation in a chain expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChainOperation {
    /// Access a field (e.g., `content`, `tags`)
    FieldAccess(String),
    /// Map operation that creates an iterator scope
    Map,
    /// Split array operation
    SplitArray,
    /// Split by word operation
    SplitByWord,
    /// Apply a reducer function
    Reducer(String),
    /// Access special field like `$atom_uuid`
    SpecialField(String),
}

/// Represents a parsed chain expression with depth and branch information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedChain {
    /// Original expression string
    pub expression: String,
    /// Sequence of operations in the chain
    pub operations: Vec<ChainOperation>,
    /// Iterator depth (number of .map() calls)
    pub depth: usize,
    /// Branch identifier for fan-out detection
    pub branch: String,
    /// Iterator scopes at each depth
    pub scopes: Vec<IteratorScope>,
}

/// Represents an iterator scope at a specific depth
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IteratorScope {
    /// Depth level (0 = root)
    pub depth: usize,
    /// Operations that led to this scope
    pub operations: Vec<ChainOperation>,
    /// Branch path up to this scope
    pub branch_path: String,
}

/// Result of analyzing multiple chains for compatibility
#[derive(Debug, Clone)]
pub struct CompatibilityAnalysis {
    /// Maximum depth among all chains
    pub max_depth: usize,
    /// Whether the chains are compatible
    pub compatible: bool,
    /// Chains grouped by branch
    pub branches: HashMap<String, Vec<ParsedChain>>,
    /// Field alignment requirements
    pub alignment_requirements: Vec<FieldAlignmentRequirement>,
}

/// Field alignment types based on depth relative to maximum depth
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldAlignment {
    /// 1:1 aligned - uses maximum depth D
    OneToOne,
    /// Broadcast - uses shallower depth, duplicated across all rows at depth D
    Broadcast,
    /// Reduced - would exceed depth D, must be reduced
    Reduced,
}

/// Field alignment requirement for a specific field
#[derive(Debug, Clone)]
pub struct FieldAlignmentRequirement {
    /// Original field expression
    pub field_expression: String,
    /// Iterator depth of this field
    pub depth: usize,
    /// Required alignment type
    pub alignment: FieldAlignment,
    /// Branch identifier
    pub branch: String,
}
