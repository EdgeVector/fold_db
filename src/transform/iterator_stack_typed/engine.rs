use std::collections::HashMap;

use super::types::{EmittedEntry, IterationItem, IteratorSpec, TypedInput};
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::transform::functions::{registry, IteratorExecutionResult};

/// A minimal typed iterator engine that supports:
/// - Schema iteration over a field's items
/// - split_by_word on a field's textual value
/// - array split passthrough for Vec<String> or Vec<Object>{value}
///
/// This engine does not use serde_json internally. It works directly with
/// FieldValue and preserves atom_uuid for persistence when not splitting.
pub struct TypedEngine;

impl TypedEngine {
    pub fn new() -> Self {
        Self
    }

    /// Execute a single chain given as iterator specs (outer to inner) for a
    /// specific output field key.
    pub fn execute_chain(
        &self,
        specs: &[IteratorSpec],
        input: &TypedInput,
        output_field_key: &str,
    ) -> HashMap<String, Vec<EmittedEntry>> {
        let mut result: HashMap<String, Vec<EmittedEntry>> = HashMap::new();
        let emitted = self.evaluate_specs(specs, input, output_field_key);
        result.insert(output_field_key.to_string(), emitted);
        result
    }

    fn evaluate_specs(
        &self,
        specs: &[IteratorSpec],
        input: &TypedInput,
        field_key: &str,
    ) -> Vec<EmittedEntry> {
        if specs.is_empty() {
            return Vec::new();
        }

        // Start with root items from the first Schema spec
        let mut current_items: Vec<IterationItem> = Vec::new();
        match &specs[0] {
            IteratorSpec::Schema { field_name } => {
                if let Some(map) = input.get(field_name) {
                    for (key, value) in map.iter() {
                        current_items.push(IterationItem {
                            key: key.clone(),
                            value: value.clone(),
                            is_text_token: false,
                        });
                    }
                }
            }
            _ => {
                // If first is not Schema, there is nothing to iterate
                return Vec::new();
            }
        }

        // Process the rest of the specs
        let mut depth_path: Vec<usize> = Vec::new();
        let mut emitted: Vec<EmittedEntry> = Vec::new();

        self.recurse_specs(
            &specs[1..],
            input,
            field_key,
            &mut current_items,
            &mut depth_path,
            &mut emitted,
        );

        // If there were no nested specs producing emission, persist original atoms once
        if emitted.is_empty() {
            for item in current_items {
                emitted.push(EmittedEntry {
                    row_id: "0".to_string(),
                    atom_uuid: item.value.atom_uuid.clone(),
                    value_text: None,
                });
            }
        }

        emitted
    }

    #[allow(clippy::only_used_in_recursion)]
    fn recurse_specs(
        &self,
        specs: &[IteratorSpec],
        _input: &TypedInput,
        _field_key: &str,
        items: &mut [IterationItem],
        depth_path: &mut Vec<usize>,
        emitted: &mut Vec<EmittedEntry>,
    ) {
        if specs.is_empty() {
            // At a leaf: emit the current items
            for (i, item) in items.iter().enumerate() {
                let row_id = if depth_path.is_empty() {
                    i.to_string()
                } else {
                    let mut p = depth_path.clone();
                    p.push(i);
                    p.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join("/")
                };

                // For text tokens (like from split_by_word), use the text as value_text
                let value_text = if item.is_text_token {
                    if let serde_json::Value::String(text) = &item.value.value {
                        Some(text.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };

                emitted.push(EmittedEntry {
                    row_id,
                    atom_uuid: item.value.atom_uuid.clone(),
                    value_text,
                });
            }
            return;
        }

        let head = &specs[0];
        let tail = &specs[1..];

        match head {
            IteratorSpec::IteratorFunction { name, .. } => {
                let reg = registry();

                // Get the iterator function from registry and execute it
                if let Some(func) = reg.get_iterator(name) {
                    for (i, item) in items.iter().enumerate() {
                        match func.execute(item) {
                            IteratorExecutionResult::TextTokens(tokens) => {
                                // Convert text tokens to IterationItems for further processing
                                let new_items: Vec<IterationItem> = tokens
                                    .iter()
                                    .enumerate()
                                    .map(|(j, token)| IterationItem {
                                        key: KeyValue::new(Some(format!("{}_{}", i, j)), None),
                                        value: FieldValue {
                                            value: serde_json::Value::String(token.clone()),
                                            atom_uuid: item.value.atom_uuid.clone(),
                                            source_file_name: item.value.source_file_name.clone(),
                                        },
                                        is_text_token: true,
                                    })
                                    .collect();

                                // Check if the next spec is a reducer
                                if let Some(next_spec) = tail.first() {
                                    if matches!(next_spec, IteratorSpec::ReducerFunction { .. }) {
                                        // For reducers, process all tokens as a group
                                        depth_path.push(i);
                                        let mut new_items_mut = new_items.clone();
                                        self.recurse_specs(
                                            tail,
                                            _input,
                                            _field_key,
                                            &mut new_items_mut,
                                            depth_path,
                                            emitted,
                                        );
                                        depth_path.pop();
                                    } else {
                                        // For other specs, process each token individually
                                        for (j, new_item) in new_items.iter().enumerate() {
                                            depth_path.push(i);
                                            depth_path.push(j);
                                            self.recurse_specs(
                                                tail,
                                                _input,
                                                _field_key,
                                                &mut [new_item.clone()],
                                                depth_path,
                                                emitted,
                                            );
                                            depth_path.pop();
                                            depth_path.pop();
                                        }
                                    }
                                } else {
                                    // No more specs, emit each token individually
                                    for (j, new_item) in new_items.iter().enumerate() {
                                        let mut path = depth_path.clone();
                                        path.push(i);
                                        path.push(j);
                                        let value_text = match &new_item.value.value {
                                            serde_json::Value::String(s) => s.clone(),
                                            other => other.to_string(),
                                        };
                                        emitted.push(EmittedEntry {
                                            row_id: path
                                                .iter()
                                                .map(|x| x.to_string())
                                                .collect::<Vec<_>>()
                                                .join("/"),
                                            atom_uuid: new_item.value.atom_uuid.clone(),
                                            value_text: Some(value_text),
                                        });
                                    }
                                }
                            }
                            IteratorExecutionResult::Items(new_items) => {
                                // Check if the next spec is a reducer
                                if let Some(next_spec) = tail.first() {
                                    if matches!(next_spec, IteratorSpec::ReducerFunction { .. }) {
                                        // For reducers, process all items as a group
                                        depth_path.push(i);
                                        let mut new_items_mut = new_items.clone();
                                        self.recurse_specs(
                                            tail,
                                            _input,
                                            _field_key,
                                            &mut new_items_mut,
                                            depth_path,
                                            emitted,
                                        );
                                        depth_path.pop();
                                    } else {
                                        // Handle item-producing functions (like split_array)
                                        for (j, new_item) in new_items.iter().enumerate() {
                                            depth_path.push(i);
                                            depth_path.push(j);
                                            self.recurse_specs(
                                                tail,
                                                _input,
                                                _field_key,
                                                &mut [new_item.clone()],
                                                depth_path,
                                                emitted,
                                            );
                                            depth_path.pop();
                                            depth_path.pop();
                                        }
                                    }
                                } else {
                                    // No more specs - emit items
                                    for (j, new_item) in new_items.iter().enumerate() {
                                        depth_path.push(i);
                                        depth_path.push(j);
                                        self.recurse_specs(
                                            tail,
                                            _input,
                                            _field_key,
                                            &mut [new_item.clone()],
                                            depth_path,
                                            emitted,
                                        );
                                        depth_path.pop();
                                        depth_path.pop();
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Function not found - pass through as identity
                    let snapshot = items.to_owned();
                    for (i, _item) in snapshot.iter().enumerate() {
                        depth_path.push(i);
                        self.recurse_specs(
                            tail,
                            _input,
                            _field_key,
                            &mut snapshot.clone(),
                            depth_path,
                            emitted,
                        );
                        depth_path.pop();
                    }
                }
            }
            IteratorSpec::ReducerFunction { name, .. } => {
                let reg = registry();

                // Get the reducer function from registry and execute it on all items
                if let Some(reducer) = reg.get_reducer(name) {
                    let result = reducer.execute(items);

                    // Create a single emitted entry for the reducer result
                    let row_id = if depth_path.is_empty() {
                        "0".to_string()
                    } else {
                        depth_path
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join("/")
                    };

                    // Use the first item's atom_uuid for traceability
                    let atom_uuid = items
                        .first()
                        .map(|item| item.value.atom_uuid.clone())
                        .unwrap_or_else(|| "reducer-result".to_string());

                    emitted.push(EmittedEntry {
                        row_id,
                        atom_uuid,
                        value_text: Some(result),
                    });
                } else {
                    // Reducer not found - pass through as identity
                    let snapshot = items.to_owned();
                    for (i, _item) in snapshot.iter().enumerate() {
                        depth_path.push(i);
                        self.recurse_specs(
                            tail,
                            _input,
                            _field_key,
                            &mut snapshot.clone(),
                            depth_path,
                            emitted,
                        );
                        depth_path.pop();
                    }
                }
            }
            IteratorSpec::Schema { .. } => {
                // Nested schema iteration is not needed for initial parallel version
                let snapshot = items.to_owned();
                for (i, _item) in snapshot.iter().enumerate() {
                    depth_path.push(i);
                    self.recurse_specs(
                        tail,
                        _input,
                        _field_key,
                        &mut snapshot.clone(),
                        depth_path,
                        emitted,
                    );
                    depth_path.pop();
                }
            }
        }
    }
}

impl Default for TypedEngine {
    fn default() -> Self {
        Self::new()
    }
}
