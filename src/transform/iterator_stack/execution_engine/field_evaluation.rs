//! Field expression evaluation and operation processing

use crate::transform::iterator_stack::chain_parser::{ParsedChain, ChainOperation};
use crate::transform::iterator_stack::types::IteratorStack;
use crate::transform::iterator_stack::errors::IteratorStackResult;
use serde_json::Value;
use log::debug;

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
        debug!("evaluate_field_expression called for chain: {} at depth: {}", chain.expression, iteration_depth);
        debug!("Stack has {} scopes", stack.len());
        
        // Get the current item from the iteration depth in the stack context
        // The iteration depth is where we're actually iterating
        let current_item = if let Some(context) = stack.context_at_depth(iteration_depth) {
            if let Some(item) = &context.iterator_state.current_item {
                debug!("evaluate_field_expression - current_item from depth {}: {}", iteration_depth, item);
                item.clone()
            } else {
                // No current item at this depth - try to get from parent context or use fallback
                debug!("evaluate_field_expression - no current_item in context at depth {}, trying fallback", iteration_depth);
                return self.get_fallback_context_value(stack, iteration_depth, chain);
            }
        } else {
            debug!("evaluate_field_expression - no context at depth {}, trying fallback", iteration_depth);
            return self.get_fallback_context_value(stack, iteration_depth, chain);
        };

        debug!("evaluate_field_expression - chain operations: {:?}", chain.operations);
        
        // Filter chain operations based on what has already been applied by the iterator
        // The iterator has already applied operations up to the current depth
        let remaining_operations = self.filter_operations_for_depth(&chain.operations, iteration_depth);
        debug!("evaluate_field_expression - remaining operations: {:?}", remaining_operations);
        
        // Evaluate the remaining chain operations step by step
        let mut current_value = current_item;
        
        for operation in &remaining_operations {
            debug!("evaluate_field_expression - processing operation: {:?}, current_value: {}", operation, current_value);
            current_value = self.process_operation(operation, current_value)?;
        }
        
        debug!("evaluate_field_expression returned: {}", current_value);
        Ok(current_value)
    }

    /// Gets a fallback context value when the primary context is not available
    fn get_fallback_context_value(
        &self,
        stack: &IteratorStack,
        iteration_depth: usize,
        chain: &ParsedChain,
    ) -> IteratorStackResult<Value> {
        debug!("get_fallback_context_value - depth: {}, chain: {:?}", iteration_depth, chain.operations);
        
        // Try to get from parent context
        if iteration_depth > 0 {
            for parent_depth in (0..iteration_depth).rev() {
                if let Some(parent_context) = stack.context_at_depth(parent_depth) {
                    if let Some(parent_item) = &parent_context.iterator_state.current_item {
                        debug!("get_fallback_context_value - found parent item at depth {}: {}", parent_depth, parent_item);
                        
                        // Apply the chain operations starting from the parent item
                        let mut current_value = parent_item.clone();
                        
                        // Apply operations that haven't been applied yet by the iterator
                        let remaining_operations = self.filter_operations_for_depth(&chain.operations, parent_depth);
                        debug!("get_fallback_context_value - applying remaining operations: {:?}", remaining_operations);
                        
                        for operation in &remaining_operations {
                            current_value = self.process_operation(operation, current_value)?;
                        }
                        
                        debug!("get_fallback_context_value - returning: {}", current_value);
                        return Ok(current_value);
                    }
                }
            }
        }
        
        // Try to get from root context
        if let Some(root_context) = stack.context_at_depth(0) {
            if let Some(root_item) = &root_context.iterator_state.current_item {
                debug!("get_fallback_context_value - found root item: {}", root_item);
                
                // Apply all chain operations from root
                let mut current_value = root_item.clone();
                
                for operation in &chain.operations {
                    current_value = self.process_operation(operation, current_value)?;
                }
                
                debug!("get_fallback_context_value - returning from root: {}", current_value);
                return Ok(current_value);
            }
        }
        
        // Last resort: try to get from root data
        if let Some(root_data) = stack.get_value("_root") {
            debug!("get_fallback_context_value - using root data: {}", root_data);
            
            // Apply all chain operations from root data
            let mut current_value = root_data.clone();
            
            for operation in &chain.operations {
                current_value = self.process_operation(operation, current_value)?;
            }
            
            debug!("get_fallback_context_value - returning from root data: {}", current_value);
            return Ok(current_value);
        }
        
        debug!("get_fallback_context_value - no fallback available, returning Null");
        Ok(Value::Null)
    }

    /// Filters chain operations based on what has already been applied by the iterator
    fn filter_operations_for_depth(
        &self,
        operations: &[ChainOperation],
        depth: usize,
    ) -> Vec<ChainOperation> {
        debug!("filter_operations_for_depth called with depth: {}, operations: {:?}", depth, operations);
        
        // The iterator has already applied operations up to the current depth
        // We need to skip the operations that correspond to the iterator scopes
        
        let mut remaining_operations = Vec::new();
        let mut operation_index = 0;
        
        // Count how many scopes we have (this corresponds to how many operations the iterator has applied)
        // For now, we'll use a simple heuristic: each depth level corresponds to one scope
        // In a more sophisticated implementation, we would track exactly which operations were applied
        
        if depth == 0 {
            // At depth 0, the iterator has applied the first FieldAccess and Map operations
            // Skip the first FieldAccess and Map operations
            if operations.len() >= 2 {
                if let (ChainOperation::FieldAccess(_), ChainOperation::Map) = (&operations[0], &operations[1]) {
                    operation_index = 2; // Skip FieldAccess and Map
                }
            }
        } else if depth == 1 {
            // At depth 1, the iterator has already applied operations up to the current scope
            // We need to check what type of operation was applied at depth 1
            
            if operations.len() >= 4 {
                // Check if it's a SplitByWord operation pattern
                if let (
                    ChainOperation::FieldAccess(_), 
                    ChainOperation::Map, 
                    ChainOperation::FieldAccess(_), 
                    ChainOperation::SplitByWord
                ) = (&operations[0], &operations[1], &operations[2], &operations[3]) {
                    operation_index = 4; // Skip FieldAccess, Map, FieldAccess, SplitByWord
                }
                // Check if it's a SplitArray operation pattern
                else if let (
                    ChainOperation::FieldAccess(_), 
                    ChainOperation::Map, 
                    ChainOperation::FieldAccess(_), 
                    ChainOperation::SplitArray
                ) = (&operations[0], &operations[1], &operations[2], &operations[3]) {
                    operation_index = 4; // Skip FieldAccess, Map, FieldAccess, SplitArray
                }
            }
        }
        
        // Add the remaining operations
        for operation in operations.iter().skip(operation_index) {
            remaining_operations.push(operation.clone());
        }
        
        debug!("Filtered operations: {:?}", remaining_operations);
        remaining_operations
    }

    /// Processes a single chain operation
    fn process_operation(
        &self,
        operation: &ChainOperation,
        current_value: Value,
    ) -> IteratorStackResult<Value> {
        match operation {
            ChainOperation::FieldAccess(field_name) => {
                debug!("process_operation - FieldAccess for '{}'", field_name);
                if let Value::Object(obj) = &current_value {
                    if let Some(field_value) = obj.get(field_name) {
                        debug!("process_operation - found field '{}': {}", field_name, field_value);
                        Ok(field_value.clone())
                    } else {
                        debug!("process_operation - field '{}' not found in object", field_name);
                        Ok(Value::Null)
                    }
                } else {
                    debug!("process_operation - current_value is not an object: {}", current_value);
                    Ok(Value::Null)
                }
            }
            ChainOperation::Map => {
                debug!("process_operation - Map operation, returning current value");
                Ok(current_value)
            }
            ChainOperation::SplitArray => {
                debug!("process_operation - SplitArray operation");
                if let Value::Array(arr) = &current_value {
                    if let Some(first_item) = arr.first() {
                        debug!("process_operation - returning first array item: {}", first_item);
                        Ok(first_item.clone())
                    } else {
                        debug!("process_operation - array is empty");
                        Ok(Value::Null)
                    }
                } else {
                    debug!("process_operation - current_value is not an array: {}", current_value);
                    Ok(current_value)
                }
            }
            ChainOperation::SplitByWord => {
                debug!("process_operation - SplitByWord operation");
                
                // Handle different data formats
                let text_to_split = if let Value::String(text) = &current_value {
                    // Direct string format: "blahblah"
                    text.clone()
                } else if let Value::Array(array) = &current_value {
                    // Array format: [{"value": "blahblah"}]
                    if let Some(first_item) = array.first() {
                        if let Some(value_obj) = first_item.as_object() {
                            if let Some(value_str) = value_obj.get("value").and_then(|v| v.as_str()) {
                                debug!("process_operation - extracted text from array format: '{}'", value_str);
                                value_str.to_string()
                            } else {
                                debug!("process_operation - array item doesn't have 'value' field");
                                return Ok(Value::Null);
                            }
                        } else {
                            debug!("process_operation - array item is not an object");
                            return Ok(Value::Null);
                        }
                    } else {
                        debug!("process_operation - array is empty");
                        return Ok(Value::Null);
                    }
                } else if let Value::Object(obj) = &current_value {
                    // Object format: {"value": "blahblah"}
                    if let Some(value_str) = obj.get("value").and_then(|v| v.as_str()) {
                        debug!("process_operation - extracted text from object format: '{}'", value_str);
                        value_str.to_string()
                    } else {
                        debug!("process_operation - object doesn't have 'value' field");
                        return Ok(Value::Null);
                    }
                } else {
                    debug!("process_operation - current_value is neither string, array, nor object: {}", current_value);
                    return Ok(current_value);
                };
                
           let words: Vec<&str> = text_to_split.split_whitespace().collect();
           if let Some(first_word) = words.first() {
               debug!("process_operation - returning first word: {}", first_word);
               Ok(Value::String(first_word.to_string()))
           } else {
               debug!("process_operation - no words found in text");
               Ok(Value::Null)
           }
            }
            ChainOperation::Reducer(_reducer_name) => {
                debug!("process_operation - Reducer operation (not implemented)");
                Ok(current_value)
            }
            ChainOperation::SpecialField(field_name) => {
                debug!("process_operation - SpecialField for '{}'", field_name);
                if let Value::Object(obj) = &current_value {
                    if let Some(field_value) = obj.get(field_name) {
                        debug!("process_operation - found special field '{}': {}", field_name, field_value);
                        Ok(field_value.clone())
                    } else {
                        debug!("process_operation - special field '{}' not found in object", field_name);
                        Ok(Value::Null)
                    }
                } else {
                    debug!("process_operation - current_value is not an object: {}", current_value);
                    Ok(Value::Null)
                }
            }
        }
    }
}
