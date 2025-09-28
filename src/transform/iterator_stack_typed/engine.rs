use std::collections::HashMap;

use crate::schema::types::field::FieldValue;
// use crate::schema::types::key_value::KeyValue;

use super::types::{EmittedEntry, IterationItem, IteratorSpec, TypedInput};

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
                        current_items.push(IterationItem { key: key.clone(), value: value.clone() });
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
            // At a leaf: emit the current items, preserving atom_uuid when not split
            for (i, item) in items.iter().enumerate() {
                let row_id = if depth_path.is_empty() {
                    i.to_string()
                } else {
                    let mut p = depth_path.clone();
                    p.push(i);
                    p.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("/")
                };

                emitted.push(EmittedEntry {
                    row_id,
                    atom_uuid: item.value.atom_uuid.clone(),
                    value_text: None,
                });
            }
            return;
        }

        let head = &specs[0];
        let tail = &specs[1..];

        match head {
            IteratorSpec::ArraySplit { .. } => {
                // For now treat as identity over items; array payload not represented
                let snapshot = items.to_owned();
                for (i, _item) in snapshot.iter().enumerate() {
                    depth_path.push(i);
                    self.recurse_specs(tail, _input, _field_key, &mut snapshot.clone(), depth_path, emitted);
                    depth_path.pop();
                }
            }
            IteratorSpec::WordSplit { .. } => {
                // For each item, split its textual value into words; emit per word
                for (i, item) in items.iter().enumerate() {
                    let text = extract_text_value(&item.value);
                    let words = split_words(&text);
                    for (j, w) in words.iter().enumerate() {
                        let mut path = depth_path.clone();
                        path.push(i);
                        path.push(j);
                        emitted.push(EmittedEntry {
                            row_id: path.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("/"),
                            atom_uuid: item.value.atom_uuid.clone(),
                            value_text: Some(w.clone()),
                        });
                    }
                }
                // No deeper recursion after a terminal split
            }
            IteratorSpec::Schema { .. } => {
                // Nested schema iteration is not needed for initial parallel version
                let snapshot = items.to_owned();
                for (i, _item) in snapshot.iter().enumerate() {
                    depth_path.push(i);
                    self.recurse_specs(tail, _input, _field_key, &mut snapshot.clone(), depth_path, emitted);
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

fn extract_text_value(field_value: &FieldValue) -> String {
    // FieldValue.value is serde_json::Value; extract best-effort string
    match &field_value.value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(map) => map
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        serde_json::Value::Array(arr) => arr
            .first()
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

fn split_words(text: &str) -> Vec<String> {
    text.split_whitespace().map(|s| s.to_string()).collect()
}


