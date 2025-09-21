//! Iterator stack types and data structures
//!
//! This module contains all the core data structures used by the iterator stack system,
//! including scopes, contexts, iterator types, and configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manages a stack of iterator scopes for field evaluation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IteratorStack {
    /// Stack of active iterator scopes
    pub scopes: Vec<ActiveScope>,
    /// Current depth in the stack
    pub current_depth: usize,
    /// Maximum allowed depth
    pub max_depth: usize,
    /// Context data for each scope level
    pub scope_contexts: HashMap<usize, ScopeContext>,
}

/// Represents an active iterator scope in the stack
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveScope {
    /// Depth level of this scope
    pub depth: usize,
    /// Iterator type and configuration
    pub iterator_type: IteratorType,
    /// Current position in iteration
    pub position: usize,
    /// Total number of items to iterate
    pub total_items: usize,
    /// Branch path that led to this scope
    pub branch_path: String,
    /// Parent scope depth (None for root)
    pub parent_depth: Option<usize>,
}

/// Types of iterators that can be created
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IteratorType {
    /// Schema-level iterator (e.g., blogpost.map())
    Schema { field_name: String },
    /// Array split iterator (e.g., tags.split_array())
    ArraySplit { field_name: String },
    /// Word split iterator (e.g., content.split_by_word())
    WordSplit { field_name: String },
    /// Custom iterator with specific logic
    Custom {
        name: String,
        config: IteratorConfig,
    },
}

/// Configuration for custom iterators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IteratorConfig {
    /// Iterator-specific parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Whether this iterator can be parallelized
    pub parallelizable: bool,
    /// Memory optimization hints
    pub memory_hint: MemoryHint,
}

/// Memory optimization hints for iterators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MemoryHint {
    /// Low memory usage, suitable for streaming
    Streaming,
    /// Moderate memory usage, can buffer some data
    Buffered,
    /// High memory usage, loads all data
    InMemory,
}

/// Context information for a specific scope
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopeContext {
    /// Values available at this scope level
    pub values: HashMap<String, serde_json::Value>,
    /// Iterator state for this scope
    pub iterator_state: IteratorState,
    /// Parent context reference
    pub parent_context: Option<usize>,
}

/// State information for an active iterator
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IteratorState {
    /// Current item being processed
    pub current_item: Option<serde_json::Value>,
    /// All items in the current iteration
    pub items: Vec<serde_json::Value>,
    /// Whether iteration has completed
    pub completed: bool,
    /// Error state if iteration failed
    pub error: Option<String>,
}

/// Summary information about the iterator stack state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IteratorStackSummary {
    /// Total number of scopes in the stack
    pub total_scopes: usize,
    /// Current depth
    pub current_depth: usize,
    /// Maximum allowed depth
    pub max_depth: usize,
    /// Active iterator types
    pub active_iterators: Vec<IteratorType>,
    /// Completion status for each depth
    pub completion_status: HashMap<usize, bool>,
}
