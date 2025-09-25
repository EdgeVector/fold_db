//! Field expression evaluation and operation processing

use crate::transform::iterator_stack::chain_parser::{ChainOperation, ParsedChain};
use crate::transform::iterator_stack::errors::IteratorStackResult;
use crate::transform::iterator_stack::types::IteratorStack;
use serde_json::Value;

/// Error types for field evaluation
#[derive(Debug, Clone, PartialEq)]
pub enum FieldEvaluationError {
    /// Field not found in context
    FieldNotFound(String),
    /// Invalid operation for current value type
    InvalidOperation(String),
    /// Evaluation failed with specific reason
    EvaluationFailed(String),
}

impl std::fmt::Display for FieldEvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldEvaluationError::FieldNotFound(field) => {
                write!(f, "Field not found: {}", field)
            }
            FieldEvaluationError::InvalidOperation(op) => {
                write!(f, "Invalid operation: {}", op)
            }
            FieldEvaluationError::EvaluationFailed(reason) => {
                write!(f, "Evaluation failed: {}", reason)
            }
        }
    }
}

impl std::error::Error for FieldEvaluationError {}

/// Field evaluation methods
pub trait FieldEvaluator {
    /// Evaluates a field expression at a specific iteration depth
    fn evaluate_field_expression(
        &self,
        chain: &ParsedChain,
        stack: &IteratorStack,
        iteration_depth: usize,
    ) -> IteratorStackResult<Value>;

    /// Gets a fallback context value when the primary context is not available
    fn get_fallback_context_value(
        &self,
        stack: &IteratorStack,
        iteration_depth: usize,
        chain: &ParsedChain,
    ) -> IteratorStackResult<Value>;

    /// Filters chain operations based on what has already been applied by the iterator
    fn filter_operations_for_depth(
        &self,
        operations: &[ChainOperation],
        depth: usize,
    ) -> Vec<ChainOperation>;

    /// Processes a single chain operation
    fn process_operation(
        &self,
        operation: &ChainOperation,
        current_value: Value,
    ) -> IteratorStackResult<Value>;
}

/// Default implementation of field evaluation methods
pub struct DefaultFieldEvaluator;

impl FieldEvaluator for DefaultFieldEvaluator {
    /// Evaluates a field expression at a specific iteration depth
    fn evaluate_field_expression(
        &self,
        chain: &ParsedChain,
        stack: &IteratorStack,
        iteration_depth: usize,
    ) -> IteratorStackResult<Value> {
        // Get the current item from the iteration depth in the stack context
        // The iteration depth is where we're actually iterating
        let current_item = if let Some(context) = stack.context_at_depth(iteration_depth) {
            if let Some(item) = &context.iterator_state.current_item {
                item.clone()
            } else {
                // No current item at this depth - try to get from parent context or use fallback
                return self.get_fallback_context_value(stack, iteration_depth, chain);
            }
        } else {
            return self.get_fallback_context_value(stack, iteration_depth, chain);
        };

        // Filter chain operations based on what has already been applied by the iterator
        // The iterator has already applied operations up to the current depth
        let remaining_operations =
            self.filter_operations_for_depth(&chain.operations, iteration_depth);

        // Evaluate the remaining chain operations step by step
        let mut current_value = current_item;

        for operation in &remaining_operations {
            current_value = self.process_operation(operation, current_value)?;
        }

        Ok(current_value)
    }

    /// Gets a fallback context value when the primary context is not available
    fn get_fallback_context_value(
        &self,
        stack: &IteratorStack,
        iteration_depth: usize,
        chain: &ParsedChain,
    ) -> IteratorStackResult<Value> {
        // Try to get from parent context
        if iteration_depth > 0 {
            for parent_depth in (0..iteration_depth).rev() {
                if let Some(parent_context) = stack.context_at_depth(parent_depth) {
                    if let Some(parent_item) = &parent_context.iterator_state.current_item {
                        // Apply the chain operations starting from the parent item
                        let mut current_value = parent_item.clone();

                        // Apply operations that haven't been applied yet by the iterator
                        let remaining_operations =
                            self.filter_operations_for_depth(&chain.operations, parent_depth);

                        for operation in &remaining_operations {
                            current_value = self.process_operation(operation, current_value)?;
                        }

                        return Ok(current_value);
                    }
                }
            }
        }

        // Try to get from root context
        if let Some(root_context) = stack.context_at_depth(0) {
            if let Some(root_item) = &root_context.iterator_state.current_item {
                // Apply all chain operations from root
                let mut current_value = root_item.clone();

                for operation in &chain.operations {
                    current_value = self.process_operation(operation, current_value)?;
                }

                return Ok(current_value);
            }
        }

        // Last resort: try to get from root data
        if let Some(root_data) = stack.get_value("_root") {
            // Apply all chain operations from root data
            let mut current_value = root_data.clone();

            for operation in &chain.operations {
                current_value = self.process_operation(operation, current_value)?;
            }

            return Ok(current_value);
        }

        Ok(Value::Null)
    }

    /// Filters chain operations based on what has already been applied by the iterator
    fn filter_operations_for_depth(
        &self,
        operations: &[ChainOperation],
        depth: usize,
    ) -> Vec<ChainOperation> {
        let map_indices: Vec<usize> = operations
            .iter()
            .enumerate()
            .filter_map(|(idx, op)| matches!(op, ChainOperation::Map).then_some(idx))
            .collect();

        if map_indices.is_empty() {
            return operations.to_vec();
        }

        let skip_index = if depth < map_indices.len() {
            map_indices[depth] + 1
        } else {
            operations.len()
        };

        operations.iter().skip(skip_index).cloned().collect()
    }

    /// Processes a single chain operation
    fn process_operation(
        &self,
        operation: &ChainOperation,
        current_value: Value,
    ) -> IteratorStackResult<Value> {
        match operation {
            ChainOperation::FieldAccess(field_name) => {
                if let Value::Object(obj) = &current_value {
                    if let Some(field_value) = obj.get(field_name) {
                        Ok(field_value.clone())
                    } else {
                        Ok(Value::Null)
                    }
                } else {
                    Ok(Value::Null)
                }
            }
            ChainOperation::Map => {
                Ok(current_value)
            }
            ChainOperation::SplitArray => {
                if let Value::Array(arr) = &current_value {
                    if let Some(first_item) = arr.first() {
                        Ok(first_item.clone())
                    } else {
                        Ok(Value::Null)
                    }
                } else {
                    Ok(current_value)
                }
            }
            ChainOperation::SplitByWord => {
                // Handle different data formats
                let text_to_split = if let Value::String(text) = &current_value {
                    // Direct string format: "blahblah"
                    text.clone()
                } else if let Value::Array(array) = &current_value {
                    // Array format: [{"value": "blahblah"}]
                    if let Some(first_item) = array.first() {
                        if let Some(value_obj) = first_item.as_object() {
                            if let Some(value_str) = value_obj.get("value").and_then(|v| v.as_str())
                            {
                                value_str.to_string()
                            } else {
                                return Ok(Value::Null);
                            }
                        } else {
                            return Ok(Value::Null);
                        }
                    } else {
                        return Ok(Value::Null);
                    }
                } else if let Value::Object(obj) = &current_value {
                    // Object format: {"value": "blahblah"}
                    if let Some(value_str) = obj.get("value").and_then(|v| v.as_str()) {
                        value_str.to_string()
                    } else {
                        return Ok(Value::Null);
                    }
                } else {
                    return Ok(current_value);
                };

                let words: Vec<&str> = text_to_split.split_whitespace().collect();
                if let Some(first_word) = words.first() {
                    Ok(Value::String(first_word.to_string()))
                } else {
                    Ok(Value::Null)
                }
            }
            ChainOperation::Reducer(_reducer_name) => {
                Ok(current_value)
            }
            ChainOperation::SpecialField(field_name) => {
                if let Value::Object(obj) = &current_value {
                    if let Some(field_value) = obj.get(field_name) {
                        Ok(field_value.clone())
                    } else {
                        Ok(Value::Null)
                    }
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }
}
