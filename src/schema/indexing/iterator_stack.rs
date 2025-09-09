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
    use crate::schema::indexing::chain_parser::{ChainParser, ChainOperation};
    use std::collections::HashMap;

    // Test constants
    const TEST_FIELD_NAME: &str = "blogpost";
    const TEST_BRANCH_PATH: &str = "blogpost";
    const TEST_MAX_DEPTH: usize = 5;

    fn create_test_scope(depth: usize, iterator_type: IteratorType, parent_depth: Option<usize>) -> ActiveScope {
        ActiveScope {
            depth,
            iterator_type,
            position: 0,
            total_items: 10,
            branch_path: format!("{}.depth_{}", TEST_BRANCH_PATH, depth),
            parent_depth,
        }
    }

    fn create_test_iterator_config() -> IteratorConfig {
        let mut parameters = HashMap::new();
        parameters.insert("batch_size".to_string(), serde_json::json!(100));
        parameters.insert("timeout".to_string(), serde_json::json!(5000));
        
        IteratorConfig {
            parameters,
            parallelizable: true,
            memory_hint: MemoryHint::Buffered,
        }
    }

    #[test]
    fn test_iterator_stack_creation() {
        let stack = IteratorStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.current_depth(), 0);
        assert_eq!(stack.max_depth(), 10);
    }

    #[test]
    fn test_iterator_stack_with_custom_max_depth() {
        let stack = IteratorStack::with_max_depth(TEST_MAX_DEPTH);
        assert!(stack.is_empty());
        assert_eq!(stack.current_depth(), 0);
        assert_eq!(stack.max_depth(), TEST_MAX_DEPTH);
    }

    #[test]
    fn test_default_implementation() {
        let stack = IteratorStack::default();
        assert!(stack.is_empty());
        assert_eq!(stack.current_depth(), 0);
        assert_eq!(stack.max_depth(), 10);
    }

    #[test]
    fn test_scope_management_basic() {
        let mut stack = IteratorStack::new();
        
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );

        stack.push_scope(scope).unwrap();
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.current_depth(), 1);
        assert!(!stack.is_empty());

        let popped = stack.pop_scope().unwrap();
        assert_eq!(popped.depth, 0);
        assert!(stack.is_empty());
        assert_eq!(stack.current_depth(), 0);
    }

    #[test]
    fn test_scope_management_multiple_scopes() {
        let mut stack = IteratorStack::new();
        
        // Push multiple scopes
        let scope1 = create_test_scope(
            0,
            IteratorType::Schema { field_name: "level1".to_string() },
            None,
        );
        let scope2 = create_test_scope(
            1,
            IteratorType::ArraySplit { field_name: "level2".to_string() },
            Some(0),
        );
        let scope3 = create_test_scope(
            2,
            IteratorType::WordSplit { field_name: "level3".to_string() },
            Some(1),
        );

        stack.push_scope(scope1).unwrap();
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.current_depth(), 1);

        stack.push_scope(scope2).unwrap();
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.current_depth(), 2);

        stack.push_scope(scope3).unwrap();
        assert_eq!(stack.len(), 3);
        assert_eq!(stack.current_depth(), 3);

        // Test popping in reverse order
        let popped3 = stack.pop_scope().unwrap();
        assert_eq!(popped3.depth, 2);
        assert_eq!(stack.len(), 2);

        let popped2 = stack.pop_scope().unwrap();
        assert_eq!(popped2.depth, 1);
        assert_eq!(stack.len(), 1);

        let popped1 = stack.pop_scope().unwrap();
        assert_eq!(popped1.depth, 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_current_scope_access() {
        let mut stack = IteratorStack::new();
        
        // Test empty stack
        assert!(stack.current_scope().is_none());
        assert!(stack.current_scope_mut().is_none());

        // Add scope and test access
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();

        let current_scope = stack.current_scope().unwrap();
        assert_eq!(current_scope.depth, 0);
        if let IteratorType::Schema { field_name } = &current_scope.iterator_type {
            assert_eq!(field_name, TEST_FIELD_NAME);
        } else {
            panic!("Expected Schema iterator type");
        }

        // Test mutable access
        let current_scope_mut = stack.current_scope_mut().unwrap();
        current_scope_mut.position = 5;
        assert_eq!(current_scope_mut.position, 5);
    }

    #[test]
    fn test_scope_at_depth_access() {
        let mut stack = IteratorStack::new();
        
        // Test empty stack
        assert!(stack.scope_at_depth(0).is_none());

        // Add scopes at different depths
        let scope1 = create_test_scope(
            0,
            IteratorType::Schema { field_name: "level1".to_string() },
            None,
        );
        let scope2 = create_test_scope(
            2, // Skip depth 1 to test non-sequential depths
            IteratorType::ArraySplit { field_name: "level2".to_string() },
            Some(0),
        );

        stack.push_scope(scope1).unwrap();
        stack.push_scope(scope2).unwrap();

        // Test access to existing depths
        let scope_at_0 = stack.scope_at_depth(0).unwrap();
        assert_eq!(scope_at_0.depth, 0);

        let scope_at_2 = stack.scope_at_depth(2).unwrap();
        assert_eq!(scope_at_2.depth, 2);

        // Test access to non-existing depth
        assert!(stack.scope_at_depth(1).is_none());
        assert!(stack.scope_at_depth(3).is_none());
    }

    #[test]
    fn test_iterator_type_variants() {
        let mut stack = IteratorStack::new();
        
        // Test Schema iterator
        let schema_scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: "blogpost".to_string() },
            None,
        );
        stack.push_scope(schema_scope).unwrap();
        let current = stack.current_scope().unwrap();
        assert!(matches!(current.iterator_type, IteratorType::Schema { .. }));

        // Test ArraySplit iterator
        let array_scope = create_test_scope(
            1,
            IteratorType::ArraySplit { field_name: "tags".to_string() },
            Some(0),
        );
        stack.push_scope(array_scope).unwrap();
        let current = stack.current_scope().unwrap();
        assert!(matches!(current.iterator_type, IteratorType::ArraySplit { .. }));

        // Test WordSplit iterator
        let word_scope = create_test_scope(
            2,
            IteratorType::WordSplit { field_name: "content".to_string() },
            Some(1),
        );
        stack.push_scope(word_scope).unwrap();
        let current = stack.current_scope().unwrap();
        assert!(matches!(current.iterator_type, IteratorType::WordSplit { .. }));

        // Test Custom iterator
        let custom_config = create_test_iterator_config();
        let custom_scope = create_test_scope(
            3,
            IteratorType::Custom { 
                name: "custom_iter".to_string(), 
                config: custom_config.clone() 
            },
            Some(2),
        );
        stack.push_scope(custom_scope).unwrap();
        let current = stack.current_scope().unwrap();
        assert!(matches!(current.iterator_type, IteratorType::Custom { .. }));
    }

    #[test]
    fn test_value_storage_and_retrieval_single_scope() {
        let mut stack = IteratorStack::new();
        
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();
        
        let value = serde_json::json!({"title": "Test Post", "content": "Test content"});
        stack.set_current_value("current_post".to_string(), value.clone()).unwrap();
        
        let retrieved = stack.get_value("current_post").unwrap();
        assert_eq!(retrieved, &value);
    }

    #[test]
    fn test_value_storage_and_retrieval_multiple_scopes() {
        let mut stack = IteratorStack::new();
        
        // Create nested scopes with different values
        let scope1 = create_test_scope(
            0,
            IteratorType::Schema { field_name: "level1".to_string() },
            None,
        );
        stack.push_scope(scope1).unwrap();
        
        let value1 = serde_json::json!({"level": 1, "data": "first"});
        stack.set_current_value("level_data".to_string(), value1.clone()).unwrap();

        let scope2 = create_test_scope(
            1,
            IteratorType::ArraySplit { field_name: "level2".to_string() },
            Some(0),
        );
        stack.push_scope(scope2).unwrap();
        
        let value2 = serde_json::json!({"level": 2, "data": "second"});
        stack.set_current_value("level_data".to_string(), value2.clone()).unwrap();

        // Test that we get the most recent value (from current scope)
        let retrieved = stack.get_value("level_data").unwrap();
        assert_eq!(retrieved, &value2);

        // Pop scope and test that we get the parent value
        stack.pop_scope().unwrap();
        let retrieved_parent = stack.get_value("level_data").unwrap();
        assert_eq!(retrieved_parent, &value1);
    }

    #[test]
    fn test_value_retrieval_nonexistent_key() {
        let mut stack = IteratorStack::new();
        
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();
        
        // Test retrieving non-existent key
        assert!(stack.get_value("nonexistent_key").is_none());
    }

    #[test]
    fn test_value_storage_without_current_scope() {
        let mut stack = IteratorStack::new();
        
        // Try to set value without any scope
        let value = serde_json::json!({"test": "value"});
        let result = stack.set_current_value("test_key".to_string(), value);
        assert!(result.is_err());
        
        if let Err(IteratorStackError::ExecutionError { message }) = result {
            assert!(message.contains("No current scope available"));
        } else {
            panic!("Expected ExecutionError");
        }
    }

    #[test]
    fn test_context_at_depth_access() {
        let mut stack = IteratorStack::new();
        
        // Test empty stack
        assert!(stack.context_at_depth(0).is_none());

        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();

        // Test context access
        let context = stack.context_at_depth(0).unwrap();
        assert!(context.values.is_empty());
        assert!(!context.iterator_state.completed);
        assert!(context.iterator_state.current_item.is_none());
        assert!(context.iterator_state.error.is_none());
        assert_eq!(context.parent_context, None);

        // Test mutable context access
        let context_mut = stack.context_at_depth_mut(0).unwrap();
        context_mut.values.insert("test_key".to_string(), serde_json::json!("test_value"));
        context_mut.iterator_state.completed = true;

        // Verify changes
        let context_updated = stack.context_at_depth(0).unwrap();
        assert_eq!(context_updated.values.len(), 1);
        assert!(context_updated.iterator_state.completed);
    }

    #[test]
    fn test_iterator_state_management() {
        let mut stack = IteratorStack::new();
        
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();

        // Create test iterator state
        let test_items = vec![
            serde_json::json!("item1"),
            serde_json::json!("item2"),
            serde_json::json!("item3"),
        ];
        
        let iterator_state = IteratorState {
            current_item: test_items.first().cloned(),
            items: test_items.clone(),
            completed: false,
            error: None,
        };

        // Update iterator state
        stack.update_current_iterator_state(iterator_state).unwrap();

        // Verify state was updated
        let context = stack.context_at_depth(0).unwrap();
        assert_eq!(context.iterator_state.items.len(), 3);
        assert_eq!(context.iterator_state.current_item, test_items.first().cloned());
        assert!(!context.iterator_state.completed);
        assert!(context.iterator_state.error.is_none());
    }

    #[test]
    fn test_iterator_state_update_without_scope() {
        let mut stack = IteratorStack::new();
        
        let iterator_state = IteratorState {
            current_item: None,
            items: vec![],
            completed: false,
            error: None,
        };

        let result = stack.update_current_iterator_state(iterator_state);
        assert!(result.is_err());
        
        if let Err(IteratorStackError::ExecutionError { message }) = result {
            assert!(message.contains("No current scope available"));
        } else {
            panic!("Expected ExecutionError");
        }
    }

    #[test]
    fn test_iterator_advancement() {
        let mut stack = IteratorStack::new();
        
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();

        // Test initial position
        let current_scope = stack.current_scope().unwrap();
        assert_eq!(current_scope.position, 0);

        // Advance iterator
        let has_more = stack.advance_current_iterator().unwrap();
        assert!(has_more); // Should be true since position (1) < total_items (10)

        let current_scope = stack.current_scope().unwrap();
        assert_eq!(current_scope.position, 1);

        // Advance to near the end
        for _ in 0..8 {
            stack.advance_current_iterator().unwrap();
        }

        let current_scope = stack.current_scope().unwrap();
        assert_eq!(current_scope.position, 9);

        // Advance past the end
        let has_more = stack.advance_current_iterator().unwrap();
        assert!(!has_more); // Should be false since position (10) >= total_items (10)

        let current_scope = stack.current_scope().unwrap();
        assert_eq!(current_scope.position, 10);
    }

    #[test]
    fn test_iterator_advancement_without_scope() {
        let mut stack = IteratorStack::new();
        
        let result = stack.advance_current_iterator();
        assert!(result.is_err());
        
        if let Err(IteratorStackError::ExecutionError { message }) = result {
            assert!(message.contains("No current scope to advance"));
        } else {
            panic!("Expected ExecutionError");
        }
    }

    #[test]
    fn test_iterator_reset() {
        let mut stack = IteratorStack::new();
        
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();

        // Advance iterator
        stack.advance_current_iterator().unwrap();
        stack.advance_current_iterator().unwrap();
        
        let current_scope = stack.current_scope().unwrap();
        assert_eq!(current_scope.position, 2);

        // Reset iterator
        stack.reset_current_iterator().unwrap();
        
        let current_scope = stack.current_scope().unwrap();
        assert_eq!(current_scope.position, 0);
    }

    #[test]
    fn test_iterator_reset_without_scope() {
        let mut stack = IteratorStack::new();
        
        let result = stack.reset_current_iterator();
        assert!(result.is_err());
        
        if let Err(IteratorStackError::ExecutionError { message }) = result {
            assert!(message.contains("No current scope to reset"));
        } else {
            panic!("Expected ExecutionError");
        }
    }

    #[test]
    fn test_all_completed_empty_stack() {
        let stack = IteratorStack::new();
        assert!(stack.all_completed());
    }

    #[test]
    fn test_all_completed_single_scope() {
        let mut stack = IteratorStack::new();
        
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();

        // Initially not completed
        assert!(!stack.all_completed());

        // Mark as completed
        let context = stack.context_at_depth_mut(0).unwrap();
        context.iterator_state.completed = true;
        
        assert!(stack.all_completed());
    }

    #[test]
    fn test_all_completed_multiple_scopes() {
        let mut stack = IteratorStack::new();
        
        let scope1 = create_test_scope(
            0,
            IteratorType::Schema { field_name: "level1".to_string() },
            None,
        );
        let scope2 = create_test_scope(
            1,
            IteratorType::ArraySplit { field_name: "level2".to_string() },
            Some(0),
        );
        
        stack.push_scope(scope1).unwrap();
        stack.push_scope(scope2).unwrap();

        // Initially not completed
        assert!(!stack.all_completed());

        // Mark first scope as completed
        let context1 = stack.context_at_depth_mut(0).unwrap();
        context1.iterator_state.completed = true;
        
        // Still not all completed
        assert!(!stack.all_completed());

        // Mark second scope as completed
        let context2 = stack.context_at_depth_mut(1).unwrap();
        context2.iterator_state.completed = true;
        
        // Now all completed
        assert!(stack.all_completed());
    }

    #[test]
    fn test_iterator_stack_summary() {
        let mut stack = IteratorStack::with_max_depth(TEST_MAX_DEPTH);
        
        // Empty stack summary
        let summary = stack.get_summary();
        assert_eq!(summary.total_scopes, 0);
        assert_eq!(summary.current_depth, 0);
        assert_eq!(summary.max_depth, TEST_MAX_DEPTH);
        assert!(summary.active_iterators.is_empty());
        assert!(summary.completion_status.is_empty());

        // Add scopes and test summary
        let scope1 = create_test_scope(
            0,
            IteratorType::Schema { field_name: "level1".to_string() },
            None,
        );
        let scope2 = create_test_scope(
            1,
            IteratorType::ArraySplit { field_name: "level2".to_string() },
            Some(0),
        );
        
        stack.push_scope(scope1).unwrap();
        stack.push_scope(scope2).unwrap();

        // Mark first scope as completed
        let context1 = stack.context_at_depth_mut(0).unwrap();
        context1.iterator_state.completed = true;

        let summary = stack.get_summary();
        assert_eq!(summary.total_scopes, 2);
        assert_eq!(summary.current_depth, 2);
        assert_eq!(summary.max_depth, TEST_MAX_DEPTH);
        assert_eq!(summary.active_iterators.len(), 2);
        assert_eq!(summary.completion_status.len(), 2);
        
        // Check completion status
        assert!(summary.completion_status.get(&0).unwrap());
        assert!(!summary.completion_status.get(&1).unwrap());
    }

    #[test]
    fn test_max_depth_enforcement() {
        let mut stack = IteratorStack::with_max_depth(2);
        
        // Should succeed - within max depth
        let scope1 = create_test_scope(
            0,
            IteratorType::Schema { field_name: "test".to_string() },
            None,
        );
        stack.push_scope(scope1).unwrap();

        let scope2 = create_test_scope(
            1,
            IteratorType::ArraySplit { field_name: "tags".to_string() },
            Some(0),
        );
        stack.push_scope(scope2).unwrap();

        // Should fail - exceeds max depth
        let scope3 = create_test_scope(
            3, // Depth 3 exceeds max depth of 2
            IteratorType::WordSplit { field_name: "content".to_string() },
            Some(1),
        );
        
        let result = stack.push_scope(scope3);
        assert!(result.is_err());
        
        if let Err(IteratorStackError::MaxDepthExceeded { current_depth, max_depth }) = result {
            assert_eq!(current_depth, 3);
            assert_eq!(max_depth, 2);
        } else {
            panic!("Expected MaxDepthExceeded error");
        }
    }

    #[test]
    fn test_iterator_stack_from_chain() {
        let parser = ChainParser::new();
        let chain = parser.parse("blogpost.map().content.split_by_word().map()").unwrap();
        
        let stack = IteratorStack::from_chain(&chain).unwrap();
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.current_depth(), 2);
        
        // Verify the iterator types were determined correctly
        let scope_at_0 = stack.scope_at_depth(0).unwrap();
        assert!(matches!(scope_at_0.iterator_type, IteratorType::Schema { .. }));
        
        let scope_at_1 = stack.scope_at_depth(1).unwrap();
        assert!(matches!(scope_at_1.iterator_type, IteratorType::WordSplit { .. }));
    }

    #[test]
    fn test_iterator_stack_from_chain_exceeds_max_depth() {
        let parser = ChainParser::new();
        let mut stack = IteratorStack::with_max_depth(1);
        
        let chain = parser.parse("blogpost.map().content.split_by_word().map()").unwrap();
        
        let result = stack.build_from_chain(&chain);
        assert!(result.is_err());
        
        if let Err(IteratorStackError::MaxDepthExceeded { current_depth, max_depth }) = result {
            assert!(current_depth > max_depth);
        } else {
            panic!("Expected MaxDepthExceeded error");
        }
    }

    #[test]
    fn test_determine_iterator_type_from_scope() {
        let stack = IteratorStack::new();
        
        // Test Schema iterator detection
        let mut scope = crate::schema::indexing::chain_parser::IteratorScope {
            depth: 0,
            branch_path: "blogpost".to_string(),
            operations: vec![
                ChainOperation::FieldAccess("blogpost".to_string()),
                ChainOperation::Map,
            ],
        };
        
        let iterator_type = stack.determine_iterator_type_from_scope(&scope).unwrap();
        assert!(matches!(iterator_type, IteratorType::Schema { field_name } if field_name == "blogpost"));

        // Test ArraySplit iterator detection
        scope.operations = vec![
            ChainOperation::FieldAccess("tags".to_string()),
            ChainOperation::SplitArray,
        ];
        
        let iterator_type = stack.determine_iterator_type_from_scope(&scope).unwrap();
        assert!(matches!(iterator_type, IteratorType::ArraySplit { field_name } if field_name == "tags"));

        // Test WordSplit iterator detection
        scope.operations = vec![
            ChainOperation::FieldAccess("content".to_string()),
            ChainOperation::SplitByWord,
        ];
        
        let iterator_type = stack.determine_iterator_type_from_scope(&scope).unwrap();
        assert!(matches!(iterator_type, IteratorType::WordSplit { field_name } if field_name == "content"));

        // Test empty operations error
        scope.operations = vec![];
        
        let result = stack.determine_iterator_type_from_scope(&scope);
        assert!(result.is_err());
        
        if let Err(IteratorStackError::InvalidIteratorChain { .. }) = result {
            // Expected error
        } else {
            panic!("Expected InvalidIteratorChain error");
        }
    }

    #[test]
    fn test_memory_hint_variants() {
        let streaming_hint = MemoryHint::Streaming;
        let buffered_hint = MemoryHint::Buffered;
        let in_memory_hint = MemoryHint::InMemory;

        // Test that all variants are different
        assert_ne!(streaming_hint, buffered_hint);
        assert_ne!(buffered_hint, in_memory_hint);
        assert_ne!(streaming_hint, in_memory_hint);
    }

    #[test]
    fn test_iterator_config() {
        let config = create_test_iterator_config();
        
        assert_eq!(config.parameters.len(), 2);
        assert!(config.parameters.contains_key("batch_size"));
        assert!(config.parameters.contains_key("timeout"));
        assert!(config.parallelizable);
        assert_eq!(config.memory_hint, MemoryHint::Buffered);
    }

    #[test]
    fn test_scope_context_parent_reference() {
        let mut stack = IteratorStack::new();
        
        let scope1 = create_test_scope(
            0,
            IteratorType::Schema { field_name: "parent".to_string() },
            None,
        );
        let scope2 = create_test_scope(
            1,
            IteratorType::ArraySplit { field_name: "child".to_string() },
            Some(0),
        );
        
        stack.push_scope(scope1).unwrap();
        stack.push_scope(scope2).unwrap();

        // Check parent context reference
        let context1 = stack.context_at_depth(0).unwrap();
        assert_eq!(context1.parent_context, None);

        let context2 = stack.context_at_depth(1).unwrap();
        assert_eq!(context2.parent_context, Some(0));
    }

    #[test]
    fn test_iterator_state_error_handling() {
        let mut stack = IteratorStack::new();
        
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();

        // Create iterator state with error
        let iterator_state = IteratorState {
            current_item: None,
            items: vec![],
            completed: false,
            error: Some("Test error message".to_string()),
        };

        stack.update_current_iterator_state(iterator_state).unwrap();

        let context = stack.context_at_depth(0).unwrap();
        assert!(context.iterator_state.error.is_some());
        assert_eq!(context.iterator_state.error.as_ref().unwrap(), "Test error message");
    }

    #[test]
    fn test_pop_scope_cleanup() {
        let mut stack = IteratorStack::new();
        
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();

        // Add some context data
        stack.set_current_value("test_key".to_string(), serde_json::json!("test_value")).unwrap();

        // Verify context exists
        assert!(stack.context_at_depth(0).is_some());
        assert!(stack.get_value("test_key").is_some());

        // Pop scope
        stack.pop_scope().unwrap();

        // Verify context is cleaned up
        assert!(stack.context_at_depth(0).is_none());
        assert!(stack.get_value("test_key").is_none());
    }

    #[test]
    fn test_pop_scope_empty_stack() {
        let mut stack = IteratorStack::new();
        
        let result = stack.pop_scope();
        assert!(result.is_none());
        assert!(stack.is_empty());
        assert_eq!(stack.current_depth(), 0);
    }

    #[test]
    fn test_iterator_stack_serialization() {
        let mut stack = IteratorStack::new();
        
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();

        // Test serialization
        let serialized = serde_json::to_string(&stack).unwrap();
        assert!(serialized.contains("blogpost"));
        assert!(serialized.contains("Schema"));

        // Test deserialization
        let deserialized: IteratorStack = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized.current_depth(), 1);
        assert!(!deserialized.is_empty());
    }

    #[test]
    fn test_iterator_stack_clone() {
        let mut stack = IteratorStack::new();
        
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack.push_scope(scope).unwrap();
        stack.set_current_value("test_key".to_string(), serde_json::json!("test_value")).unwrap();

        let cloned_stack = stack.clone();
        
        // Verify clone is identical
        assert_eq!(stack.len(), cloned_stack.len());
        assert_eq!(stack.current_depth(), cloned_stack.current_depth());
        assert_eq!(stack.max_depth(), cloned_stack.max_depth());
        
        // Verify clone has same data
        assert_eq!(stack.get_value("test_key"), cloned_stack.get_value("test_key"));
    }

    #[test]
    fn test_iterator_stack_debug_formatting() {
        let stack = IteratorStack::new();
        let debug_str = format!("{:?}", stack);
        
        assert!(debug_str.contains("IteratorStack"));
        assert!(debug_str.contains("scopes"));
        assert!(debug_str.contains("current_depth"));
        assert!(debug_str.contains("max_depth"));
    }

    #[test]
    fn test_blog_post_word_index_iterator_scenario() {
        let parser = ChainParser::new();
        
        // Test the complex iterator chain from BlogPostWordIndex schema
        let hash_chain = parser.parse("BlogPost.map().content.split_by_word().map()").unwrap();
        let range_chain = parser.parse("BlogPost.map().publish_date").unwrap();
        
        // Create iterator stacks for both chains
        let hash_stack = IteratorStack::from_chain(&hash_chain).unwrap();
        let range_stack = IteratorStack::from_chain(&range_chain).unwrap();
        
        // Verify hash chain stack structure (should have 2 scopes: BlogPost.map() and content.split_by_word())
        assert_eq!(hash_stack.len(), 2);
        assert_eq!(hash_stack.current_depth(), 2);
        
        // Verify range chain stack structure (should have 1 scope: BlogPost.map())
        assert_eq!(range_stack.len(), 1);
        assert_eq!(range_stack.current_depth(), 1);
        
        // Test hash chain iterator types
        let scope_0 = hash_stack.scope_at_depth(0).unwrap();
        assert!(matches!(&scope_0.iterator_type, IteratorType::Schema { field_name } if field_name == "BlogPost"));
        
        let scope_1 = hash_stack.scope_at_depth(1).unwrap();
        assert!(matches!(&scope_1.iterator_type, IteratorType::WordSplit { field_name } if field_name == "content"));
        
        // Test range chain iterator type
        let range_scope = range_stack.scope_at_depth(0).unwrap();
        assert!(matches!(&range_scope.iterator_type, IteratorType::Schema { field_name } if field_name == "BlogPost"));
        
        // Test that both stacks can handle the same root data structure
        let _test_data = serde_json::json!({
            "BlogPost": {
                "BlogPost": [
                    {
                        "author": "Alice Johnson",
                        "content": "DataFold is a powerful distributed database system.",
                        "publish_date": "2025-01-01T10:00:00Z",
                        "tags": ["tutorial", "beginners"],
                        "title": "Getting Started with DataFold"
                    },
                    {
                        "author": "Bob Smith", 
                        "content": "Range schemas are a key feature of DataFold.",
                        "publish_date": "2025-01-02T11:00:00Z",
                        "tags": ["schema", "range"],
                        "title": "Understanding Range Schemas"
                    }
                ]
            }
        });
        
        // Simulate setting up iterator states for both stacks
        let mut hash_stack_mut = hash_stack.clone();
        let mut range_stack_mut = range_stack.clone();
        
        // Set up hash stack with word-split content
        let content_items = vec![
            serde_json::json!("DataFold"),
            serde_json::json!("is"),
            serde_json::json!("a"),
            serde_json::json!("powerful"),
            serde_json::json!("distributed"),
            serde_json::json!("database"),
            serde_json::json!("system."),
            serde_json::json!("Range"),
            serde_json::json!("schemas"),
            serde_json::json!("are"),
            serde_json::json!("a"),
            serde_json::json!("key"),
            serde_json::json!("feature"),
            serde_json::json!("of"),
            serde_json::json!("DataFold.")
        ];
        
        // Update the scope's total_items to match our content
        if let Some(scope) = hash_stack_mut.current_scope_mut() {
            scope.total_items = content_items.len();
        }
        
        let hash_iterator_state = IteratorState {
            current_item: content_items.first().cloned(),
            items: content_items.clone(),
            completed: false,
            error: None,
        };
        
        hash_stack_mut.update_current_iterator_state(hash_iterator_state).unwrap();
        
        // Set up range stack with publish dates
        let date_items = vec![
            serde_json::json!("2025-01-01T10:00:00Z"),
            serde_json::json!("2025-01-02T11:00:00Z")
        ];
        
        // Update the scope's total_items to match our dates
        if let Some(scope) = range_stack_mut.current_scope_mut() {
            scope.total_items = date_items.len();
        }
        
        let range_iterator_state = IteratorState {
            current_item: date_items.first().cloned(),
            items: date_items.clone(),
            completed: false,
            error: None,
        };
        
        range_stack_mut.update_current_iterator_state(range_iterator_state).unwrap();
        
        // Test iterator advancement for hash stack (word iteration)
        // The iterator should start at position 0 and advance through the items
        let mut word_count = 0;
        loop {
            let has_more = hash_stack_mut.advance_current_iterator().unwrap();
            word_count += 1;
            if !has_more {
                break;
            }
        }
        
        // Should have advanced through all words (15 words total)
        assert_eq!(word_count, content_items.len());
        
        // Test iterator advancement for range stack (date iteration)
        let mut date_count = 0;
        loop {
            let has_more = range_stack_mut.advance_current_iterator().unwrap();
            date_count += 1;
            if !has_more {
                break;
            }
        }
        
        // Should have advanced through all dates
        assert_eq!(date_count, date_items.len());
        
        // Test value storage and retrieval across scopes
        hash_stack_mut.reset_current_iterator().unwrap();
        hash_stack_mut.set_current_value("current_word".to_string(), serde_json::json!("DataFold")).unwrap();
        
        let retrieved_word = hash_stack_mut.get_value("current_word").unwrap();
        assert_eq!(retrieved_word, &serde_json::json!("DataFold"));
        
        // Test completion tracking
        assert!(!hash_stack_mut.all_completed());
        assert!(!range_stack_mut.all_completed());
        
        // Mark all scopes as completed
        // For hash stack, we need to mark both scopes (depth 0 and 1)
        let hash_context_0 = hash_stack_mut.context_at_depth_mut(0).unwrap();
        hash_context_0.iterator_state.completed = true;
        
        let hash_context_1 = hash_stack_mut.context_at_depth_mut(1).unwrap();
        hash_context_1.iterator_state.completed = true;
        
        // For range stack, only one scope (depth 0)
        let range_context = range_stack_mut.context_at_depth_mut(0).unwrap();
        range_context.iterator_state.completed = true;
        
        assert!(hash_stack_mut.all_completed());
        assert!(range_stack_mut.all_completed());
        
        // Test stack summaries
        let hash_summary = hash_stack_mut.get_summary();
        assert_eq!(hash_summary.total_scopes, 2);
        assert_eq!(hash_summary.active_iterators.len(), 2);
        assert!(hash_summary.completion_status.get(&1).unwrap());
        
        let range_summary = range_stack_mut.get_summary();
        assert_eq!(range_summary.total_scopes, 1);
        assert_eq!(range_summary.active_iterators.len(), 1);
        assert!(range_summary.completion_status.get(&0).unwrap());
        
        // Test that the iterator types match the schema requirements
        assert!(matches!(hash_summary.active_iterators[0], IteratorType::Schema { .. }));
        assert!(matches!(hash_summary.active_iterators[1], IteratorType::WordSplit { .. }));
        assert!(matches!(range_summary.active_iterators[0], IteratorType::Schema { .. }));
    }

    #[test]
    fn test_iterator_stack_equality() {
        let mut stack1 = IteratorStack::new();
        let mut stack2 = IteratorStack::new();
        
        // Empty stacks should be equal
        assert_eq!(stack1, stack2);
        
        // Add same scope to both
        let scope = create_test_scope(
            0,
            IteratorType::Schema { field_name: TEST_FIELD_NAME.to_string() },
            None,
        );
        stack1.push_scope(scope.clone()).unwrap();
        stack2.push_scope(scope).unwrap();
        
        // Should still be equal
        assert_eq!(stack1, stack2);
        
        // Add different scope to one
        let different_scope = create_test_scope(
            1,
            IteratorType::ArraySplit { field_name: "different".to_string() },
            Some(0),
        );
        stack1.push_scope(different_scope).unwrap();
        
        // Should no longer be equal
        assert_ne!(stack1, stack2);
    }
}