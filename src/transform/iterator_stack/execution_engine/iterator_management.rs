//! Iterator stack management and item extraction

use crate::transform::iterator_stack::types::{IteratorStack, IteratorType, IteratorState};
use crate::transform::iterator_stack::errors::IteratorStackResult;
use serde_json::Value;
use log::debug;

/// Manager for iterator stack operations
pub struct IteratorManager {
    /// Field evaluator for processing field expressions
    #[allow(dead_code)]
    field_evaluator: super::field_evaluation::DefaultFieldEvaluator,
}

impl IteratorManager {
    /// Creates a new iterator manager
    pub fn new() -> Self {
        Self {
            field_evaluator: super::field_evaluation::DefaultFieldEvaluator,
        }
    }

    /// Initializes the iterator stack with input data (optimized single-pass approach)
    pub fn initialize_stack(&mut self, stack: &mut IteratorStack, input_data: &Value) -> IteratorStackResult<()> {
        debug!("Initializing iterator stack with {} scopes", stack.len());
        debug!("Input data structure: {}", input_data);

        // Set the root data directly in the root context
        if let Some(root_context) = stack.context_at_depth_mut(0) {
            root_context.values.insert("_root".to_string(), input_data.clone());
        }

        // Single-pass initialization: process scopes in depth order
        let scopes = stack.len();
        let mut parent_data = input_data.clone();

        for depth in 0..scopes {
            if let Some(scope) = stack.scope_at_depth(depth) {
                debug!("Processing scope at depth {} with iterator type: {:?}", depth, scope.iterator_type);
                debug!("Parent data at depth {}: {}", depth, parent_data);

                // Extract items for this scope using the appropriate parent data
                let items = self.extract_items_for_iterator(&scope.iterator_type, &parent_data)?;
                debug!("Extracted {} items for depth {}: {:?}", items.len(), depth, items);

                // Create iterator state
                let iterator_state = IteratorState {
                    current_item: items.first().cloned(),
                    items: items.clone(),
                    completed: items.is_empty(),
                    error: None,
                };

                debug!("Setting iterator state for depth {}: current_item={}, completed={}",
                    depth, iterator_state.current_item.is_some(), iterator_state.completed);

                // Update the context with the iterator state
                if let Some(context) = stack.context_at_depth_mut(depth) {
                    context.iterator_state = iterator_state;
                    context.values.insert(format!("depth_{}", depth), parent_data.clone());
                }

                // Update parent_data for the next iteration
                // For child scopes, use the current item from this scope
                if depth < scopes - 1 {
                    if let Some(current_item) = items.first() {
                        parent_data = current_item.clone();
                        debug!("Updated parent_data for next depth: {}", parent_data);
                    } else {
                        debug!("No items available for depth {}, using original data", depth);
                        // Keep the original parent_data if no items are available
                    }
                }
            }
        }

        Ok(())
    }

    /// Extracts items for iteration based on iterator type (optimized to reduce cloning)
    pub fn extract_items_for_iterator(
        &self,
        iterator_type: &IteratorType,
        data: &Value,
    ) -> IteratorStackResult<Vec<Value>> {
        debug!("extract_items_for_iterator called with iterator_type: {:?}, data: {}", iterator_type, data);
        debug!("Data type: {}, is_object: {}, is_array: {}", data, data.is_object(), data.is_array());

        match iterator_type {
            IteratorType::Schema { field_name } => {
                debug!("Schema iterator - looking for field '{}' in data", field_name);

                if let Some(field_value) = data.get(field_name) {
                    debug!("Found field '{}' with value: {}", field_name, field_value);
                    debug!("Field value type: {}, is_array: {}, is_object: {}",
                        field_value, field_value.is_array(), field_value.is_object());

                    if field_value.is_array() {
                        let array = field_value.as_array().unwrap();
                        debug!("Returning array with {} items", array.len());
                        Ok(array.clone())
                    } else if field_value.is_object() {
                        // If the field value is an object that contains an array, extract the array
                        if let Some(nested_array) = field_value.get(field_name) {
                            if nested_array.is_array() {
                                let array = nested_array.as_array().unwrap();
                                debug!("Found nested array '{}' with {} items", field_name, array.len());
                                Ok(array.clone())
                            } else {
                                debug!("Nested field '{}' is not an array, returning single item", field_name);
                                Ok(vec![nested_array.clone()])
                            }
                        } else {
                            debug!("Field '{}' is object but no nested array found, returning single item", field_name);
                            Ok(vec![field_value.clone()])
                        }
                    } else {
                        debug!("Returning single item as array");
                        Ok(vec![field_value.clone()])
                    }
                } else {
                    let available_fields = data.as_object()
                        .map(|obj| obj.keys().collect::<Vec<_>>())
                        .unwrap_or_default();
                    debug!("Field '{}' not found in data structure. Available fields: {:?}",
                        field_name, available_fields);
                    debug!("Data structure: {}", data);
                    Ok(vec![])
                }
            }
            IteratorType::ArraySplit { field_name } => {
                debug!("ArraySplit iterator - looking for field '{}' in data", field_name);
                if let Some(field_value) = data.get(field_name) {
                    debug!("Found field '{}' with value: {}", field_name, field_value);
                    if let Some(array) = field_value.as_array() {
                        debug!("Returning array with {} items for splitting", array.len());
                        Ok(array.clone())
                    } else {
                        debug!("Field '{}' is not an array, returning empty", field_name);
                        Ok(vec![])
                    }
                } else {
                    debug!("Field '{}' not found in data structure", field_name);
                    Ok(vec![])
                }
            }
            IteratorType::WordSplit { field_name } => {
                debug!("WordSplit iterator - looking for field '{}' in data", field_name);
                if let Some(field_value) = data.get(field_name) {
                    debug!("Found field '{}' with value: {}", field_name, field_value);
                    if let Some(text) = field_value.as_str() {
                        let words: Vec<Value> = text
                            .split_whitespace()
                            .map(|word| Value::String(word.to_string()))
                            .collect();
                        debug!("Split text '{}' into {} words: {:?}", text, words.len(), words);
                        Ok(words)
                    } else {
                        debug!("Field '{}' is not a string, returning empty", field_name);
                        Ok(vec![])
                    }
                } else {
                    debug!("Field '{}' not found in data structure", field_name);
                    Ok(vec![])
                }
            }
            IteratorType::Custom { name, config } => {
                debug!("Custom iterator '{}' with config: {:?}", name, config);
                // For now, return empty - custom iterators need specific implementation
                Ok(vec![])
            }
        }
    }
}

impl Default for IteratorManager {
    fn default() -> Self {
        Self::new()
    }
}
