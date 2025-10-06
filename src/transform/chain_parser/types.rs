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
    /// Apply a registered function (iterator or reducer)
    Function { name: String, params: Vec<String> },
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
}