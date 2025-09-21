//! Iterator stack implementation
//!
//! This module contains the core implementation of the IteratorStack functionality,
//! including scope management, iterator operations, and state handling.

use crate::transform::iterator_stack::chain_parser::{IteratorScope, ParsedChain};
use crate::transform::iterator_stack::errors::{IteratorStackError, IteratorStackResult};
use crate::transform::iterator_stack::types::{
    ActiveScope, IteratorStack, IteratorStackSummary, IteratorState, IteratorType, ScopeContext,
};
use log::debug;
use std::collections::HashMap;

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
        debug!(
            "🔧 IteratorStack::from_chain called for expression: {}",
            chain.expression
        );
        let mut stack = Self::new();
        stack.build_from_chain(chain)?;
        debug!(
            "✅ IteratorStack::from_chain completed with {} scopes",
            stack.len()
        );
        Ok(stack)
    }

    /// Builds the iterator stack from a parsed chain expression
    fn build_from_chain(&mut self, chain: &ParsedChain) -> IteratorStackResult<()> {
        debug!(
            "🔧 build_from_chain called for expression: {} with {} scopes",
            chain.expression,
            chain.scopes.len()
        );

        if chain.depth > self.max_depth {
            debug!(
                "❌ Max depth exceeded: {} > {}",
                chain.depth, self.max_depth
            );
            return Err(IteratorStackError::MaxDepthExceeded {
                current_depth: chain.depth,
                max_depth: self.max_depth,
            });
        }

        // Build scopes from the chain's iterator scopes
        for (i, scope) in chain.scopes.iter().enumerate() {
            debug!(
                "🔧 Building scope {} for expression: {}",
                i, chain.expression
            );
            self.push_scope_from_iterator_scope(scope)?;
        }

        debug!(
            "✅ build_from_chain completed for expression: {}",
            chain.expression
        );
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
            parent_depth: if scope.depth > 0 {
                Some(scope.depth - 1)
            } else {
                None
            },
        };

        self.push_scope(active_scope)?;
        Ok(())
    }

    /// Determines the iterator type from an iterator scope
    fn determine_iterator_type_from_scope(
        &self,
        scope: &IteratorScope,
    ) -> IteratorStackResult<IteratorType> {
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
                crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess(
                    field_name,
                ) => {
                    // Check if this is followed by a split operation
                    if i + 1 < operations.len() {
                        match &operations[i + 1] {
                            crate::transform::iterator_stack::chain_parser::ChainOperation::SplitArray => {
                                return Ok(IteratorType::ArraySplit {
                                    field_name: field_name.clone(),
                                });
                            }
                            crate::transform::iterator_stack::chain_parser::ChainOperation::SplitByWord => {
                                return Ok(IteratorType::WordSplit {
                                    field_name: field_name.clone(),
                                });
                            }
                            crate::transform::iterator_stack::chain_parser::ChainOperation::Map => {
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
        let field_name = scope
            .branch_path
            .split('.')
            .next()
            .unwrap_or("unknown")
            .to_string();
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
    pub fn set_current_value(
        &mut self,
        key: String,
        value: serde_json::Value,
    ) -> IteratorStackResult<()> {
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
    pub fn update_current_iterator_state(
        &mut self,
        state: IteratorState,
    ) -> IteratorStackResult<()> {
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
            active_iterators: self
                .scopes
                .iter()
                .map(|s| s.iterator_type.clone())
                .collect(),
            completion_status: self
                .scope_contexts
                .iter()
                .map(|(depth, context)| (*depth, context.iterator_state.completed))
                .collect(),
        }
    }
}
