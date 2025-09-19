//! Scope creation helpers for execution engine
//!
//! Contains logic for creating default iterator scopes when none are provided
//! and determining when iterator scopes should be created.

use crate::transform::iterator_stack::chain_parser::{ChainOperation, ParsedChain};
use crate::transform::iterator_stack::errors::IteratorStackResult;
use crate::transform::iterator_stack::types::IteratorStack;
use log::debug;
use serde_json::Value;

/// Helper methods for scope creation logic
pub struct ScopeCreationHelper;

impl ScopeCreationHelper {
    /// Creates default iterator scopes when none are provided
    pub fn create_default_scopes(
        stack: &mut IteratorStack,
        chain: &ParsedChain,
        input_data: &Value,
    ) -> IteratorStackResult<()> {
        debug!("Creating default scopes for chain: {}", chain.expression);

        // Analyze the chain operations to determine what scopes to create
        let mut current_depth = 0;
        let mut current_ops = Vec::new();
        let mut branch_path = String::new();
        let mut last_field_name = String::new();

        for operation in chain.operations.iter() {
            current_ops.push(operation.clone());

            match operation {
                ChainOperation::FieldAccess(field_name) => {
                    last_field_name = field_name.clone();
                    if branch_path.is_empty() {
                        branch_path = field_name.clone();
                    } else {
                        branch_path = format!("{}.{}", branch_path, field_name);
                    }

                    // Check if this field should create an iterator scope (for arrays)
                    if Self::should_create_iterator_scope(field_name, input_data) {
                        debug!(
                            "Creating Schema iterator scope for field: {} at depth: {}",
                            field_name, current_depth
                        );

                        let iterator_type =
                            crate::transform::iterator_stack::IteratorType::Schema {
                                field_name: field_name.clone(),
                            };

                        let active_scope = crate::transform::iterator_stack::ActiveScope {
                            depth: current_depth,
                            iterator_type,
                            position: 0,
                            total_items: 0,
                            branch_path: branch_path.clone(),
                            parent_depth: if current_depth > 0 {
                                Some(current_depth - 1)
                            } else {
                                None
                            },
                        };

                        stack.push_scope(active_scope)?;
                        current_depth += 1;
                        current_ops.clear();
                    }
                }
                ChainOperation::Map => {
                    // Check if we should create a scope for the last field
                    if !last_field_name.is_empty()
                        && Self::should_create_iterator_scope(&last_field_name, input_data)
                    {
                        debug!(
                            "Creating Schema iterator scope for field: {} at depth: {}",
                            last_field_name, current_depth
                        );

                        let iterator_type =
                            crate::transform::iterator_stack::IteratorType::Schema {
                                field_name: last_field_name.clone(),
                            };

                        let active_scope = crate::transform::iterator_stack::ActiveScope {
                            depth: current_depth,
                            iterator_type,
                            position: 0,
                            total_items: 0,
                            branch_path: branch_path.clone(),
                            parent_depth: if current_depth > 0 {
                                Some(current_depth - 1)
                            } else {
                                None
                            },
                        };

                        stack.push_scope(active_scope)?;
                        current_depth += 1;
                        current_ops.clear();
                    }
                }
                ChainOperation::SplitByWord => {
                    // Create a WordSplit iterator scope for the last field
                    if !last_field_name.is_empty() {
                        debug!(
                            "Creating WordSplit iterator scope for field: {} at depth: {}",
                            last_field_name, current_depth
                        );

                        let iterator_type =
                            crate::transform::iterator_stack::IteratorType::WordSplit {
                                field_name: last_field_name.clone(),
                            };

                        let active_scope = crate::transform::iterator_stack::ActiveScope {
                            depth: current_depth,
                            iterator_type,
                            position: 0,
                            total_items: 0,
                            branch_path: branch_path.clone(),
                            parent_depth: if current_depth > 0 {
                                Some(current_depth - 1)
                            } else {
                                None
                            },
                        };

                        stack.push_scope(active_scope)?;
                        current_depth += 1;
                        current_ops.clear();
                    }
                }
                ChainOperation::SplitArray => {
                    // Create an ArraySplit iterator scope for the last field
                    if !last_field_name.is_empty() {
                        debug!(
                            "Creating ArraySplit iterator scope for field: {} at depth: {}",
                            last_field_name, current_depth
                        );

                        let iterator_type =
                            crate::transform::iterator_stack::IteratorType::ArraySplit {
                                field_name: last_field_name.clone(),
                            };

                        let active_scope = crate::transform::iterator_stack::ActiveScope {
                            depth: current_depth,
                            iterator_type,
                            position: 0,
                            total_items: 0,
                            branch_path: branch_path.clone(),
                            parent_depth: if current_depth > 0 {
                                Some(current_depth - 1)
                            } else {
                                None
                            },
                        };

                        stack.push_scope(active_scope)?;
                        current_depth += 1;
                        current_ops.clear();
                    }
                }
                _ => {
                    // For other operations, continue processing
                }
            }
        }

        debug!("Created {} scopes", stack.len());
        Ok(())
    }

    /// Determines if a field should create an iterator scope
    pub fn should_create_iterator_scope(field_name: &str, input_data: &Value) -> bool {
        // Check if the field exists and is an array
        if let Some(field_value) = input_data.get(field_name) {
            if field_value.is_array() {
                let array = field_value.as_array().unwrap();
                // Create iterator scope for any array, even single-element arrays
                // This is needed for expressions like "BlogPost.map().title" where BlogPost
                // contains an array of blog post objects
                debug!(
                    "Field {} is an array with {} elements",
                    field_name,
                    array.len()
                );
                return !array.is_empty();
            } else if field_value.is_string() {
                // For string fields, we can create scopes for word splitting
                let text = field_value.as_str().unwrap();
                let word_count = text.split_whitespace().count();
                debug!("Field {} is a string with {} words", field_name, word_count);
                return word_count > 1;
            }
        }
        debug!(
            "Field {} is not an array/string or doesn't exist",
            field_name
        );
        false
    }
}
