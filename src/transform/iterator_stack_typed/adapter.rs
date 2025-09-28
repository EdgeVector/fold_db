use std::collections::HashMap;

use serde_json::Value;

use crate::transform::iterator_stack::chain_parser::types::{ChainOperation, ParsedChain};
use crate::transform::result_types::{ExecutionResult, IndexEntry};
use crate::transform::iterator_stack_typed::engine::TypedEngine;
use crate::transform::iterator_stack_typed::types::{IteratorSpec, TypedInput};

/// Execute multiple chains against typed input and produce a legacy ExecutionResult
pub fn execute_fields_typed(
    chains: &HashMap<String, ParsedChain>,
    input: &TypedInput,
) -> ExecutionResult {
    let engine = TypedEngine::new();
    let mut index_entries: HashMap<String, Vec<IndexEntry>> = HashMap::new();
    let mut warnings: HashMap<String, Vec<crate::transform::result_types::ExecutionWarning>> = HashMap::new();

    for (field_name, chain) in chains.iter() {
        let specs = map_chain_to_specs(chain);
        let target_field_key = target_field_from_chain(chain);
        let emitted_map = engine.execute_chain(&specs, input, field_name);
        let emitted = emitted_map.get(field_name).cloned().unwrap_or_default();

        // Build reverse lookup from atom_uuid -> Value for non-split persistence
        let value_lookup: HashMap<String, Value> = input
            .get(&target_field_key)
            .map(|m| {
                m.values()
                    .map(|fv| (fv.atom_uuid.clone(), fv.value.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let mut entries: Vec<IndexEntry> = Vec::new();
        for e in emitted {
            let value = if let Some(text) = e.value_text {
                Value::String(text)
            } else {
                value_lookup.get(&e.atom_uuid).cloned().unwrap_or(Value::Null)
            };
            entries.push(IndexEntry {
                row_id: e.row_id,
                value,
                atom_uuid: e.atom_uuid,
                metadata: HashMap::new(),
                expression: chain.expression.clone(),
            });
        }
        index_entries.insert(field_name.clone(), entries);
        warnings.insert(field_name.clone(), Vec::new());
    }

    ExecutionResult { index_entries, warnings }
}

fn map_chain_to_specs(chain: &ParsedChain) -> Vec<IteratorSpec> {
    let target_field = target_field_from_chain(chain);
    let mut specs = Vec::new();
    specs.push(IteratorSpec::Schema { field_name: target_field.clone() });

    // Append split operations based on presence in the chain
    for op in &chain.operations {
        if let ChainOperation::SplitByWord = op {
            specs.push(IteratorSpec::WordSplit { field_name: target_field.clone() });
        } else if let ChainOperation::SplitArray = op {
            specs.push(IteratorSpec::ArraySplit { field_name: target_field.clone() });
        }
    }
    specs
}

fn target_field_from_chain(chain: &ParsedChain) -> String {
    let mut schema: Option<String> = None;
    let mut last_field: Option<String> = None;
    for op in &chain.operations {
        if let ChainOperation::FieldAccess(name) = op {
            if schema.is_none() {
                schema = Some(name.clone());
            } else {
                last_field = Some(name.clone());
            }
        }
    }
    match (schema, last_field) {
        (Some(s), Some(f)) => format!("{}.{}", s, f),
        (Some(s), None) => s,
        _ => chain.branch.clone(),
    }
}


