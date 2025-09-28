//! Iterator stack management and item extraction

use crate::transform::iterator_stack::errors::IteratorStackResult;
use crate::transform::iterator_stack::types::{
    ActiveScope, IteratorStack, IteratorState, IteratorType,
};
use log::debug;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::field::FieldValue;

#[derive(Default)]
pub struct IteratorDatasetCache {
    entries: HashMap<String, Vec<Value>>,
    hits: usize,
    misses: usize,
}

impl IteratorDatasetCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fetch_or_store<F>(
        &mut self,
        scope: &ActiveScope,
        parent_hash: Option<&str>,
        generator: F,
    ) -> IteratorStackResult<(Vec<Value>, String)>
    where
        F: FnOnce() -> IteratorStackResult<Vec<Value>>,
    {
        let cache_key = Self::compute_key(scope, parent_hash);

        if let Some(cached) = self.entries.get(&cache_key) {
            debug!(
                "IteratorDatasetCache hit for branch '{}' (key: {})",
                scope.branch_path, cache_key
            );
            self.hits += 1;
            return Ok((cached.clone(), cache_key));
        }

        debug!(
            "IteratorDatasetCache miss for branch '{}' (key: {})",
            scope.branch_path, cache_key
        );
        let items = generator()?;
        self.misses += 1;
        self.entries.insert(cache_key.clone(), items.clone());
        Ok((items, cache_key))
    }

    pub fn stats(&self) -> (usize, usize) {
        (self.hits, self.misses)
    }

    fn compute_key(scope: &ActiveScope, parent_hash: Option<&str>) -> String {
        let normalized_branch = normalize_branch_path(&scope.branch_path);
        let iterator_signature = iterator_signature(&scope.iterator_type);
        let parent_component = parent_hash.unwrap_or("root");
        let key_input = format!(
            "{}|{}|{}",
            parent_component, normalized_branch, iterator_signature
        );
        format!("{:x}", hash_string(&key_input))
    }
}

fn normalize_branch_path(branch_path: &str) -> String {
    if branch_path.is_empty() {
        return "_root".to_string();
    }

    branch_path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(".")
}

fn iterator_signature(iterator_type: &IteratorType) -> String {
    match iterator_type {
        IteratorType::Schema { field_name } => format!("schema:{}", field_name),
        IteratorType::ArraySplit { field_name } => format!("array_split:{}", field_name),
        IteratorType::WordSplit { field_name } => format!("word_split:{}", field_name),
        IteratorType::Custom { name, config } => {
            let serialized =
                serde_json::to_string(config).unwrap_or_else(|_| format!("{:?}", config));
            format!("custom:{}:{}", name, serialized)
        }
    }
}

fn resolve_nested_field<'a>(data: &'a Value, field_name: &str) -> Option<&'a Value> {
    if let Some(value) = data.get(field_name) {
        return Some(value);
    }

    if let Some(value) = data.get("fields").and_then(|fields_value| match fields_value {
        Value::Object(obj) => obj.get(field_name),
        _ => None,
    }) {
        return Some(value);
    }

    data.get("input")
        .and_then(|input_value| resolve_nested_field(input_value, field_name))
}

fn hash_string(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Manager for iterator stack operations
pub struct IteratorManager;

impl IteratorManager {
    /// Creates a new iterator manager
    pub fn new() -> Self {
        Self
    }

    /// Computes a deterministic row identifier for the current iterator stack state.
    /// This composes cache-like keys across scopes to produce a stable row id.
    pub fn compute_key_for_stack(stack: &IteratorStack, parent_hash: Option<&str>) -> String {
        let mut current_parent = parent_hash.unwrap_or("root").to_string();
        for depth in 0..stack.len() {
            if let Some(scope) = stack.scope_at_depth(depth) {
                let normalized_branch = normalize_branch_path(&scope.branch_path);
                let iter_sig = iterator_signature(&scope.iterator_type);
                let key_input = format!("{}|{}|{}", current_parent, normalized_branch, iter_sig);
                current_parent = format!("{:x}", hash_string(&key_input));
            }
        }
        current_parent
    }

    /// Computes a row identifier using iterator positions across all active scopes
    pub fn compute_row_id_from_positions(stack: &IteratorStack) -> String {
        let mut parts: Vec<String> = Vec::new();
        for depth in 0..stack.len() {
            if let Some(scope) = stack.scope_at_depth(depth) {
                parts.push(scope.position.to_string());
            }
        }
        if parts.is_empty() { "root".to_string() } else { parts.join("/") }
    }

    /// Initializes the iterator stack with input data (optimized single-pass approach)
    pub fn initialize_stack(
        &mut self,
        stack: &mut IteratorStack,
        input_data: &HashMap<KeyValue, FieldValue>,
        cache: &mut IteratorDatasetCache,
    ) -> IteratorStackResult<()> {
        // Set the root data directly in the root context
        if let Some(root_context) = stack.context_at_depth_mut(0) {
            root_context
                .values
                .insert("_root".to_string(), input_data.clone());
        }

        // Single-pass initialization: process scopes in depth order
        let scopes = stack.len();
        let mut parent_data = input_data.clone();
        let mut scope_parent_hashes: HashMap<usize, String> = HashMap::new();

        for depth in 0..scopes {
            if let Some(scope) = stack.scope_at_depth(depth) {
                debug!(
                    "Processing scope at depth {} with iterator type: {:?}",
                    depth, scope.iterator_type
                );
                debug!("Parent data at depth {}: {}", depth, parent_data);

                let parent_hash = scope
                    .parent_depth
                    .and_then(|parent_depth| scope_parent_hashes.get(&parent_depth));

                let (items, cache_key) =
                    cache.fetch_or_store(scope, parent_hash.map(|hash| hash.as_str()), || {
                        self.extract_items_for_iterator(&scope.iterator_type, &parent_data)
                    })?;

                debug!(
                    "Using {} items for depth {} (cache key: {})",
                    items.len(),
                    depth,
                    cache_key
                );

                scope_parent_hashes.insert(depth, cache_key);

                // Create iterator state
                let iterator_state = IteratorState {
                    current_item: items.first().cloned(),
                    items: items.clone(),
                    completed: items.is_empty(),
                    error: None,
                };

                debug!(
                    "Setting iterator state for depth {}: current_item={}, completed={}",
                    depth,
                    iterator_state.current_item.is_some(),
                    iterator_state.completed
                );

                // Update the context with the iterator state
                if let Some(context) = stack.context_at_depth_mut(depth) {
                    context.iterator_state = iterator_state;
                    context
                        .values
                        .insert(format!("depth_{}", depth), parent_data.clone());
                }

                // Update parent_data for the next iteration
                // For child scopes, use the current item from this scope
                if depth < scopes - 1 {
                    if let Some(current_item) = items.first() {
                        parent_data = current_item.clone();
                        debug!("Updated parent_data for next depth: {}", parent_data);
                    } else {
                        debug!(
                            "No items available for depth {}, using original data",
                            depth
                        );
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
        debug!(
            "extract_items_for_iterator called with iterator_type: {:?}, data: {}",
            iterator_type, data
        );
        debug!(
            "Data type: {}, is_object: {}, is_array: {}",
            data,
            data.is_object(),
            data.is_array()
        );

        match iterator_type {
            IteratorType::Schema { field_name } => {
                debug!(
                    "Schema iterator - looking for field '{}' in data",
                    field_name
                );

                if let Some(field_value) = resolve_nested_field(data, field_name) {
                    debug!("Found field '{}' with value: {}", field_name, field_value);
                    debug!(
                        "Field value type: {}, is_array: {}, is_object: {}",
                        field_value,
                        field_value.is_array(),
                        field_value.is_object()
                    );

                    if let Some(array) = field_value.as_array() {
                        debug!("Returning array with {} items", array.len());
                        Ok(array.clone())
                    } else if let Some(obj) = field_value.as_object() {
                        if let Some(nested_array) =
                            obj.get(field_name).and_then(|value| value.as_array())
                        {
                            debug!(
                                "Found nested array '{}' with {} items",
                                field_name,
                                nested_array.len()
                            );
                            Ok(nested_array.clone())
                        } else if let Some(value) = obj.get("value") {
                            debug!(
                                "Object contains 'value' entry, returning as single item: {}",
                                value
                            );
                            Ok(vec![value.clone()])
                        } else {
                            debug!(
                                "Object value did not contain nested array, returning single item"
                            );
                            Ok(vec![field_value.clone()])
                        }
                    } else {
                        debug!("Returning single item as array");
                        Ok(vec![field_value.clone()])
                    }
                } else {
                    let available_fields = data
                        .as_object()
                        .map(|obj| obj.keys().collect::<Vec<_>>())
                        .unwrap_or_default();
                    debug!(
                        "Field '{}' not found in data structure. Available fields: {:?}",
                        field_name, available_fields
                    );
                    debug!("Data structure: {}", data);
                    Ok(vec![])
                }
            }
            IteratorType::ArraySplit { field_name } => {
                debug!(
                    "ArraySplit iterator - looking for field '{}' in data",
                    field_name
                );
                if let Some(field_value) = resolve_nested_field(data, field_name) {
                    debug!("Found field '{}' with value: {}", field_name, field_value);
                    if let Some(array) = field_value.as_array() {
                        debug!("Returning array with {} items for splitting", array.len());
                        Ok(array.clone())
                    } else if let Some(obj) = field_value.as_object() {
                        if let Some(nested_array) =
                            obj.get("value").and_then(|value| value.as_array())
                        {
                            debug!(
                                "Found nested 'value' array for '{}', length {}",
                                field_name,
                                nested_array.len()
                            );
                            Ok(nested_array.clone())
                        } else {
                            debug!(
                                "Field '{}' is object without array, returning empty",
                                field_name
                            );
                            Ok(vec![])
                        }
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
                debug!(
                    "WordSplit iterator - looking for field '{}' in data",
                    field_name
                );
                if let Some(field_value) = resolve_nested_field(data, field_name) {
                    debug!("Found field '{}' with value: {}", field_name, field_value);

                    // Handle different data formats
                    let text_to_split = if let Some(text) = field_value.as_str() {
                        // Direct string format: "blahblah"
                        text.to_string()
                    } else if let Some(array) = field_value.as_array() {
                        // Array format: [{"value": "blahblah"}]
                        if let Some(first_item) = array.first() {
                            if let Some(value_obj) = first_item.as_object() {
                                if let Some(value_str) =
                                    value_obj.get("value").and_then(|v| v.as_str())
                                {
                                    debug!("WordSplit iterator - extracted text from array format: '{}'", value_str);
                                    value_str.to_string()
                                } else {
                                    debug!("WordSplit iterator - array item doesn't have 'value' field");
                                    return Ok(vec![]);
                                }
                            } else {
                                debug!("WordSplit iterator - array item is not an object");
                                return Ok(vec![]);
                            }
                        } else {
                            debug!("WordSplit iterator - array is empty");
                            return Ok(vec![]);
                        }
                    } else if let Some(obj) = field_value.as_object() {
                        // Object format: {"value": "blahblah"}
                        if let Some(value_str) = obj.get("value").and_then(|v| v.as_str()) {
                            debug!(
                                "WordSplit iterator - extracted text from object format: '{}'",
                                value_str
                            );
                            value_str.to_string()
                        } else {
                            debug!("WordSplit iterator - object doesn't have 'value' field");
                            return Ok(vec![]);
                        }
                    } else {
                        debug!("WordSplit iterator - field '{}' is neither string, array, nor object, returning empty", field_name);
                        return Ok(vec![]);
                    };

                    let words: Vec<Value> = text_to_split
                        .split_whitespace()
                        .map(|word| Value::String(word.to_string()))
                        .collect();
                    debug!(
                        "Split text '{}' into {} words: {:?}",
                        text_to_split,
                        words.len(),
                        words
                    );
                    Ok(words)
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
