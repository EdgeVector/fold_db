//! Iterator stack management for schema indexing
//!
//! Manages iterator depths, scope contexts, and provides proper nesting support
//! for complex field expressions with multiple iterator levels.

use crate::schema::indexing::chain_parser::{ParsedChain, IteratorScope};
use crate::schema::indexing::errors::{IteratorStackError, IteratorStackResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manages a stack of iterator scopes for field evaluation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IteratorStack {
    /// Stack of active iterator scopes
    scopes: Vec<ActiveScope>,
    /// Current depth in the stack
    current_depth: usize,
    /// Maximum allowed depth
    max_depth: usize,
    /// Context data for each scope level
    scope_contexts: HashMap<usize, ScopeContext>,
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
    Custom { name: String, config: IteratorConfig },
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

impl Default for IteratorStack {
    fn default() -> Self {
        Self::new()
    }
}

impl IteratorStack {
    /// Creates a new empty iterator stack
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            current_depth: 0,
            max_depth: 10,
            scope_contexts: HashMap::new(),
        }
    }

    /// Creates a new iterator stack with custom max depth
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            scopes: Vec::new(),
            current_depth: 0,
            max_depth,
            scope_contexts: HashMap::new(),
        }
    }

    /// Builds an iterator stack from a parsed chain
    pub fn from_chain(chain: &ParsedChain) -> IteratorStackResult<Self> {
        let mut stack = Self::new();
        stack.build_from_chain(chain)?;
        Ok(stack)
    }

    /// Builds the iterator stack from a parsed chain expression
    fn build_from_chain(&mut self, chain: &ParsedChain) -> IteratorStackResult<()> {
        if chain.depth > self.max_depth {
            return Err(IteratorStackError::MaxDepthExceeded {
                current_depth: chain.depth,
                max_depth: self.max_depth,
            });
        }

        // Build scopes from the chain's iterator scopes
        for scope in &chain.scopes {
            self.push_scope_from_iterator_scope(scope)?;
        }

        Ok(())
    }

    /// Creates and pushes a scope from an IteratorScope
    fn push_scope_from_iterator_scope(&mut self, scope: &IteratorScope) -> IteratorStackResult<()> {
        let iterator_type = self.determine_iterator_type_from_scope(scope)?;
        
        let active_scope = ActiveScope {
            depth: scope.depth,
            iterator_type,
            position: 0,
            total_items: 0, // Will be set during execution
            branch_path: scope.branch_path.clone(),
            parent_depth: if scope.depth > 0 { Some(scope.depth - 1) } else { None },
        };

        self.push_scope(active_scope)?;
        Ok(())
    }

    /// Determines the iterator type from an iterator scope
    fn determine_iterator_type_from_scope(&self, scope: &IteratorScope) -> IteratorStackResult<IteratorType> {
        // Analyze the operations to determine the iterator type
        let operations = &scope.operations;
        
        if operations.is_empty() {
            return Err(IteratorStackError::InvalidIteratorChain {
                chain: scope.branch_path.clone(),
                reason: "Empty operations in scope".to_string(),
            });
        }

        // Look for the field that creates the iterator
        for (i, operation) in operations.iter().enumerate() {
            match operation {
                crate::schema::indexing::chain_parser::ChainOperation::FieldAccess(field_name) => {
                    // Check if this is followed by a split operation
                    if i + 1 < operations.len() {
                        match &operations[i + 1] {
                            crate::schema::indexing::chain_parser::ChainOperation::SplitArray => {
                                return Ok(IteratorType::ArraySplit {
                                    field_name: field_name.clone(),
                                });
                            }
                            crate::schema::indexing::chain_parser::ChainOperation::SplitByWord => {
                                return Ok(IteratorType::WordSplit {
                                    field_name: field_name.clone(),
                                });
                            }
                            crate::schema::indexing::chain_parser::ChainOperation::Map => {
                                return Ok(IteratorType::Schema {
                                    field_name: field_name.clone(),
                                });
                            }
                            _ => continue,
                        }
                    }
                }
                _ => continue,
            }
        }

        // Default to schema iterator if we can't determine the type
        let field_name = scope.branch_path.split('.').next().unwrap_or("unknown").to_string();
        Ok(IteratorType::Schema { field_name })
    }

    /// Pushes a new scope onto the stack
    pub fn push_scope(&mut self, scope: ActiveScope) -> IteratorStackResult<()> {
        if scope.depth > self.max_depth {
            return Err(IteratorStackError::MaxDepthExceeded {
                current_depth: scope.depth,
                max_depth: self.max_depth,
            });
        }

        // Create scope context
        let context = ScopeContext {
            values: HashMap::new(),
            iterator_state: IteratorState {
                current_item: None,
                items: Vec::new(),
                completed: false,
                error: None,
            },
            parent_context: scope.parent_depth,
        };

        self.scope_contexts.insert(scope.depth, context);
        self.scopes.push(scope);
        self.current_depth = self.scopes.len();

        Ok(())
    }

    /// Pops the top scope from the stack
    pub fn pop_scope(&mut self) -> Option<ActiveScope> {
        if let Some(scope) = self.scopes.pop() {
            self.scope_contexts.remove(&scope.depth);
            self.current_depth = self.scopes.len();
            Some(scope)
        } else {
            None
        }
    }

    /// Gets the current scope (top of stack)
    pub fn current_scope(&self) -> Option<&ActiveScope> {
        self.scopes.last()
    }

    /// Gets a mutable reference to the current scope
    pub fn current_scope_mut(&mut self) -> Option<&mut ActiveScope> {
        self.scopes.last_mut()
    }

    /// Gets a scope at a specific depth
    pub fn scope_at_depth(&self, depth: usize) -> Option<&ActiveScope> {
        self.scopes.iter().find(|s| s.depth == depth)
    }

    /// Gets the scope context at a specific depth
    pub fn context_at_depth(&self, depth: usize) -> Option<&ScopeContext> {
        self.scope_contexts.get(&depth)
    }

    /// Gets a mutable reference to the scope context at a specific depth
    pub fn context_at_depth_mut(&mut self, depth: usize) -> Option<&mut ScopeContext> {
        self.scope_contexts.get_mut(&depth)
    }

    /// Gets the current depth
    pub fn current_depth(&self) -> usize {
        self.current_depth
    }

    /// Gets the maximum depth
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    /// Checks if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }

    /// Gets the number of active scopes
    pub fn len(&self) -> usize {
        self.scopes.len()
    }

    /// Sets a value in the current scope context
    pub fn set_current_value(&mut self, key: String, value: serde_json::Value) -> IteratorStackResult<()> {
        if let Some(scope) = self.current_scope() {
            let depth = scope.depth;
            if let Some(context) = self.scope_contexts.get_mut(&depth) {
                context.values.insert(key, value);
                Ok(())
            } else {
                Err(IteratorStackError::ExecutionError {
                    message: format!("No context found for depth {}", depth),
                })
            }
        } else {
            Err(IteratorStackError::ExecutionError {
                message: "No current scope available".to_string(),
            })
        }
    }

    /// Gets a value from the scope context (searches up the stack)
    pub fn get_value(&self, key: &str) -> Option<&serde_json::Value> {
        // Search from current depth up to root
        for depth in (0..=self.current_depth).rev() {
            if let Some(context) = self.scope_contexts.get(&depth) {
                if let Some(value) = context.values.get(key) {
                    return Some(value);
                }
            }
        }
        None
    }

    /// Updates the iterator state for the current scope
    pub fn update_current_iterator_state(&mut self, state: IteratorState) -> IteratorStackResult<()> {
        if let Some(scope) = self.current_scope() {
            let depth = scope.depth;
            if let Some(context) = self.scope_contexts.get_mut(&depth) {
                context.iterator_state = state;
                Ok(())
            } else {
                Err(IteratorStackError::ExecutionError {
                    message: format!("No context found for depth {}", depth),
                })
            }
        } else {
            Err(IteratorStackError::ExecutionError {
                message: "No current scope available".to_string(),
            })
        }
    }

    /// Advances the iterator at the current scope
    pub fn advance_current_iterator(&mut self) -> IteratorStackResult<bool> {
        if let Some(scope) = self.current_scope_mut() {
            scope.position += 1;
            Ok(scope.position < scope.total_items)
        } else {
            Err(IteratorStackError::ExecutionError {
                message: "No current scope to advance".to_string(),
            })
        }
    }

    /// Resets the iterator position at the current scope
    pub fn reset_current_iterator(&mut self) -> IteratorStackResult<()> {
        if let Some(scope) = self.current_scope_mut() {
            scope.position = 0;
            Ok(())
        } else {
            Err(IteratorStackError::ExecutionError {
                message: "No current scope to reset".to_string(),
            })
        }
    }

    /// Checks if all iterators in the stack have completed
    pub fn all_completed(&self) -> bool {
        self.scope_contexts
            .values()
            .all(|context| context.iterator_state.completed)
    }

    /// Gets a summary of the iterator stack state
    pub fn get_summary(&self) -> IteratorStackSummary {
        IteratorStackSummary {
            total_scopes: self.scopes.len(),
            current_depth: self.current_depth,
            max_depth: self.max_depth,
            active_iterators: self.scopes.iter().map(|s| s.iterator_type.clone()).collect(),
            completion_status: self.scope_contexts
                .iter()
                .map(|(depth, context)| (*depth, context.iterator_state.completed))
                .collect(),
        }
    }
}

/// Summary of iterator stack state
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::indexing::chain_parser::ChainParser;

    #[test]
    fn test_iterator_stack_creation() {
        let stack = IteratorStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.current_depth(), 0);
        assert_eq!(stack.max_depth(), 10);
    }

    #[test]
    fn test_scope_management() {
        let mut stack = IteratorStack::new();
        
        let scope = ActiveScope {
            depth: 0,
            iterator_type: IteratorType::Schema {
                field_name: "blogpost".to_string(),
            },
            position: 0,
            total_items: 10,
            branch_path: "blogpost".to_string(),
            parent_depth: None,
        };

        stack.push_scope(scope).unwrap();
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.current_depth(), 1);

        let popped = stack.pop_scope().unwrap();
        assert_eq!(popped.depth, 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_value_storage_and_retrieval() {
        let mut stack = IteratorStack::new();
        
        let scope = ActiveScope {
            depth: 0,
            iterator_type: IteratorType::Schema {
                field_name: "blogpost".to_string(),
            },
            position: 0,
            total_items: 1,
            branch_path: "blogpost".to_string(),
            parent_depth: None,
        };

        stack.push_scope(scope).unwrap();
        
        let value = serde_json::json!({"title": "Test Post"});
        stack.set_current_value("current_post".to_string(), value.clone()).unwrap();
        
        let retrieved = stack.get_value("current_post").unwrap();
        assert_eq!(retrieved, &value);
    }

    #[test]
    fn test_iterator_stack_from_chain() {
        let parser = ChainParser::new();
        let chain = parser.parse("blogpost.map().content.split_by_word().map()").unwrap();
        
        let stack = IteratorStack::from_chain(&chain).unwrap();
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.current_depth(), 2);
    }

    #[test]
    fn test_max_depth_enforcement() {
        let mut stack = IteratorStack::with_max_depth(2);
        
        // Should succeed
        let scope1 = ActiveScope {
            depth: 0,
            iterator_type: IteratorType::Schema { field_name: "test".to_string() },
            position: 0,
            total_items: 1,
            branch_path: "test".to_string(),
            parent_depth: None,
        };
        stack.push_scope(scope1).unwrap();

        let scope2 = ActiveScope {
            depth: 1,
            iterator_type: IteratorType::ArraySplit { field_name: "tags".to_string() },
            position: 0,
            total_items: 1,
            branch_path: "test.tags".to_string(),
            parent_depth: Some(0),
        };
        stack.push_scope(scope2).unwrap();

        // Should fail - exceeds max depth
        let scope3 = ActiveScope {
            depth: 3,
            iterator_type: IteratorType::WordSplit { field_name: "content".to_string() },
            position: 0,
            total_items: 1,
            branch_path: "test.content".to_string(),
            parent_depth: Some(1),
        };
        
        let result = stack.push_scope(scope3);
        assert!(result.is_err());
        if let Err(IteratorStackError::MaxDepthExceeded { .. }) = result {
            // Expected error type
        } else {
            panic!("Expected MaxDepthExceeded error");
        }
    }
}