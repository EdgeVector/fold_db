//! Result aggregation for transform execution.
//!
//! This module handles the aggregation of execution results from the ExecutionEngine
//! into the final output format for different schema types.

use crate::schema::types::DeclarativeSchemaDefinition;
use crate::schema::types::schema::SchemaType;
use crate::schema::types::SchemaError;
use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::execution_engine::{ExecutionResult, IndexEntry};
use crate::transform::shared_utilities::resolve_field_value_from_chain;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};

/// Extracts optimal field value from execution engine entry.
///
/// # Arguments
///
/// * `entry` - The execution engine entry
///
/// # Returns
///
/// The extracted field value
fn extract_optimal_field_value(
    entry: &crate::transform::iterator_stack::execution_engine::IndexEntry,
) -> JsonValue {
    // For now, return the hash_value as the primary value
    // This could be enhanced to choose between hash_value and range_value based on context
    serde_json::to_value(&entry.hash_value).unwrap_or(JsonValue::Null)
}

/// Unified result aggregation function that handles all aggregation patterns.
///
/// This function consolidates the duplicate aggregation logic that was previously
/// scattered across multiple executor modules.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result from ExecutionEngine
/// * `input_values` - The original input values for fallback
/// * `all_expressions` - All expressions (including failed parsing attempts)
/// * `schema_type` - The type of schema being processed
///
/// # Returns
///
/// The aggregated result object
struct AggregationAccumulator<'a> {
    schema: &'a DeclarativeSchemaDefinition,
    expressions: HashMap<String, String>,
    raw_fields: HashMap<String, Vec<JsonValue>>,
}

impl<'a> AggregationAccumulator<'a> {
    fn new(schema: &'a DeclarativeSchemaDefinition, expressions: &[(String, String)]) -> Self {
        let expression_map = expressions
            .iter()
            .map(|(field, expr)| (field.clone(), expr.clone()))
            .collect();

        Self {
            schema,
            expressions: expression_map,
            raw_fields: HashMap::new(),
        }
    }

    fn finalize(self) -> Result<JsonValue, SchemaError> {
        let mut shape_payload = serde_json::Map::new();
        let mut used_names: HashSet<String> = HashSet::new();

        for (field_name, values) in &self.raw_fields {
            let mut candidate = self
                .expressions
                .get(field_name)
                .and_then(|expr| expression_final_segment(expr))
                .unwrap_or_else(|| sanitize_field_name(field_name));

            if candidate.is_empty() {
                candidate = sanitize_field_name(field_name);
            }

            let unique_name = ensure_unique_name(&candidate, &used_names);
            used_names.insert(unique_name.clone());

            let value = if values.len() == 1 {
                values[0].clone()
            } else {
                JsonValue::Array(values.clone())
            };

            shape_payload.insert(unique_name, value);
        }

        let hash_value = self.derive_key_value("_hash_field");
        let range_value = self.derive_key_value("_range_field");

        let shaped_input = JsonValue::Object(shape_payload);
        
        // Create a simple structured result
        let mut shaped_result = serde_json::Map::new();
        shaped_result.insert("hash".to_string(), JsonValue::String(hash_value.unwrap_or_default()));
        shaped_result.insert("range".to_string(), JsonValue::String(range_value.unwrap_or_default()));
        shaped_result.insert("fields".to_string(), shaped_input);

        let mut final_object = shaped_result;

        Ok(JsonValue::Object(final_object))
    }

    fn derive_key_value(&self, field_name: &str) -> Option<String> {
        self.raw_fields
            .get(field_name)
            .and_then(|values| values.iter().find_map(json_value_to_string))
    }
}

pub fn aggregate_results_unified(
    schema: &DeclarativeSchemaDefinition,
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
) -> Result<JsonValue, SchemaError> {
    let mut accumulator = AggregationAccumulator::new(schema, all_expressions);

    if execution_result.index_entries.is_empty() {
        process_direct_value_resolution(
            schema,
            parsed_chains,
            input_values,
            all_expressions,
            &mut accumulator,
        )?;
    } else {
        process_execution_result_aggregation(
            schema,
            parsed_chains,
            execution_result,
            input_values,
            all_expressions,
            &mut accumulator,
        )?;
    }

    let result = accumulator.finalize()?;
    Ok(result)
}

fn process_direct_value_resolution(
    _schema: &DeclarativeSchemaDefinition,
    parsed_chains: &[(String, ParsedChain)],
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
    accumulator: &mut AggregationAccumulator,
) -> Result<(), SchemaError> {
    for (field_name, expression) in all_expressions {
        let field_value = if let Some((_, parsed_chain)) =
            parsed_chains.iter().find(|(name, _)| name == field_name)
        {
            match resolve_field_value_from_chain(parsed_chain, input_values, field_name) {
                Ok(value) => value,
                Err(err) => {
                    JsonValue::Null
                }
            }
        } else {
            match crate::transform::shared_utilities::resolve_dotted_path(expression, input_values)
            {
                Ok(value) => value,
                Err(err) => {
                    JsonValue::Null
                }
            }
        };

        accumulator.raw_fields
        .insert(field_name.to_string(), vec![field_value]);
    }

    Ok(())
}

fn process_execution_result_aggregation(
    schema: &DeclarativeSchemaDefinition,
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
    accumulator: &mut AggregationAccumulator,
) -> Result<(), SchemaError> {
    match &schema.schema_type {
        SchemaType::HashRange { keyconfig: _ } => {
            let mut field_arrays: HashMap<String, Vec<JsonValue>> = HashMap::new();
            for (field_name, _) in parsed_chains.iter() {
                field_arrays.insert(field_name.clone(), Vec::new());
            }

            let mut entries_by_expression: HashMap<String, Vec<&IndexEntry>> = HashMap::new();
            for entry in &execution_result.index_entries {
                entries_by_expression
                    .entry(entry.expression.clone())
                    .or_default()
                    .push(entry);
            }

            for (field_name, parsed_chain) in parsed_chains.iter() {
                if let Some(entries) = entries_by_expression.get(&parsed_chain.expression) {
                    if let Some(values) = field_arrays.get_mut(field_name) {
                        for entry in entries {
                            values.push(extract_optimal_field_value(entry));
                        }
                    }
                }
            }

            for (field_name, values) in field_arrays {
                accumulator.raw_fields
                .insert(field_name.to_string(), values);
            }
        }
        _ => {
            let mut entries_by_expression: HashMap<String, &IndexEntry> = HashMap::new();
            for entry in &execution_result.index_entries {
                entries_by_expression.insert(entry.expression.clone(), entry);
            }

            for (field_name, expression) in all_expressions {
                let field_value = if let Some((_, parsed_chain)) =
                    parsed_chains.iter().find(|(name, _)| name == field_name)
                {
                    if let Some(entry) = entries_by_expression.get(&parsed_chain.expression) {
                        extract_optimal_field_value(entry)
                    } else {
                        match resolve_field_value_from_chain(parsed_chain, input_values, field_name)
                        {
                            Ok(value) => value,
                            Err(err) => {
                                JsonValue::Null
                            }
                        }
                    }
                } else {
                    match crate::transform::shared_utilities::resolve_dotted_path(
                        expression,
                        input_values,
                    ) {
                        Ok(value) => value,
                        Err(err) => {
                            JsonValue::Null
                        }
                    }
                };

                if !field_name.starts_with('_') {
                    accumulator.raw_fields
                    .insert(field_name.to_string(), vec![field_value]);
                }
            }
        }
    }

    Ok(())
}

fn expression_final_segment(expression: &str) -> Option<String> {
    expression.split('.').rev().find_map(|segment| {
        let trimmed = segment.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("input") || trimmed.ends_with("()") {
            None
        } else {
            Some(trimmed.trim_matches(|c| "\"'".contains(c)).to_string())
        }
    })
}

fn sanitize_field_name(field_name: &str) -> String {
    let sanitized = field_name.trim_start_matches('_');
    if sanitized.is_empty() {
        field_name.to_string()
    } else {
        sanitized.to_string()
    }
}

fn ensure_unique_name(base: &str, used_names: &HashSet<String>) -> String {
    if !used_names.contains(base) {
        return base.to_string();
    }

    let mut index = 1;
    loop {
        let candidate = format!("{}_{}", base, index);
        if !used_names.contains(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn json_value_to_string(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(s) => Some(s.clone()),
        JsonValue::Number(n) => Some(n.to_string()),
        JsonValue::Bool(b) => Some(b.to_string()),
        JsonValue::Null => None,
        JsonValue::Array(arr) => arr.first().and_then(json_value_to_string),
        JsonValue::Object(_) => Some(value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::{
        DeclarativeSchemaDefinition, FieldDefinition,
    };
    use crate::schema::types::schema::SchemaType;
    use crate::transform::iterator_stack::chain_parser::{ChainOperation, ParsedChain};
    use crate::transform::iterator_stack::execution_engine::core::ExecutionStatistics;
    use crate::transform::iterator_stack::execution_engine::{ExecutionResult, IndexEntry};
    use serde_json::json;
    use serde_json::Value as JsonValue;
    use std::collections::HashMap;

    fn build_parsed_chain(expression: &str, segments: &[&str]) -> ParsedChain {
        ParsedChain {
            expression: expression.to_string(),
            operations: segments
                .iter()
                .map(|segment| ChainOperation::FieldAccess(segment.to_string()))
                .collect(),
            depth: 0,
            branch: "main".to_string(),
            scopes: vec![],
        }
    }

    fn build_index_entry(expression: &str, value: JsonValue) -> IndexEntry {
        IndexEntry {
            expression: expression.to_string(),
            hash_value: value,
            range_value: JsonValue::Null,
            atom_uuid: "test-uuid".to_string(),
            metadata: HashMap::new(),
        }
    }

    fn build_execution_stats(total_entries: usize) -> ExecutionStatistics {
        ExecutionStatistics {
            total_entries,
            items_per_depth: HashMap::new(),
            memory_usage_bytes: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    fn collect_expressions(chains: &[(String, ParsedChain)]) -> Vec<(String, String)> {
        chains
            .iter()
            .map(|(field, chain)| (field.clone(), chain.expression.clone()))
            .collect()
    }

    #[test]
    fn test_aggregate_results_unified_empty_execution() {
        let mut fields = HashMap::new();
        fields.insert(
            "field1".to_string(),
            FieldDefinition {
                field_expression: Some("input.field1".to_string()),
            },
        );

        let schema = DeclarativeSchemaDefinition::new(
            "single_schema".to_string(),
            SchemaType::Single,
            None,
            fields,
        );

        let parsed_chains = vec![(
            "field1".to_string(),
            ParsedChain {
                operations: vec![
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess(
                        "input".to_string(),
                    ),
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess(
                        "field1".to_string(),
                    ),
                ],
                expression: "input.field1".to_string(),
                depth: 0,
                branch: "main".to_string(),
                scopes: vec![],
            },
        )];

        let execution_result = ExecutionResult {
            index_entries: vec![],
            statistics:
                crate::transform::iterator_stack::execution_engine::core::ExecutionStatistics {
                    total_entries: 0,
                    items_per_depth: HashMap::new(),
                    memory_usage_bytes: 0,
                    cache_hits: 0,
                    cache_misses: 0,
                },
            warnings: vec![],
        };

        let input_values = HashMap::from([(
            "input".to_string(),
            json!({
                "field1": "value1"
            }),
        )]);

        let all_expressions = vec![("field1".to_string(), "input.field1".to_string())];

        let result = aggregate_results_unified(
            &schema,
            &parsed_chains,
            &execution_result,
            &input_values,
            &all_expressions,
        );

        assert!(result.is_ok());
        let result_value = result.unwrap();
        assert_eq!(result_value["field1"], json!("value1"));
        assert_eq!(result_value["hash"], json!(""));
        assert_eq!(result_value["range"], json!(""));
        assert_eq!(result_value["fields"]["field1"], json!("value1"));
    }

    #[test]
    fn test_aggregate_results_unified_with_execution() {
        let mut fields = HashMap::new();
        fields.insert(
            "field1".to_string(),
            FieldDefinition {
                field_expression: Some("input.field1".to_string()),
            },
        );

        let schema = DeclarativeSchemaDefinition::new(
            "single_schema".to_string(),
            SchemaType::Single,
            None,
            fields,
        );

        let parsed_chains = vec![(
            "field1".to_string(),
            ParsedChain {
                operations: vec![
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess(
                        "input".to_string(),
                    ),
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess(
                        "field1".to_string(),
                    ),
                ],
                expression: "input.field1".to_string(),
                depth: 0,
                branch: "main".to_string(),
                scopes: vec![],
            },
        )];

        let execution_result = ExecutionResult {
            index_entries: vec![IndexEntry {
                expression: "input.field1".to_string(),
                hash_value: json!("executed_value1"),
                range_value: json!("range_value1"),
                atom_uuid: "test-uuid".to_string(),
                metadata: HashMap::new(),
            }],
            statistics:
                crate::transform::iterator_stack::execution_engine::core::ExecutionStatistics {
                    total_entries: 1,
                    items_per_depth: HashMap::new(),
                    memory_usage_bytes: 0,
                    cache_hits: 0,
                    cache_misses: 0,
                },
            warnings: vec![],
        };

        let input_values = HashMap::from([(
            "input".to_string(),
            json!({
                "field1": "fallback_value1"
            }),
        )]);

        let all_expressions = vec![("field1".to_string(), "input.field1".to_string())];

        let result = aggregate_results_unified(
            &schema,
            &parsed_chains,
            &execution_result,
            &input_values,
            &all_expressions,
        );

        assert!(result.is_ok());
        let result_value = result.unwrap();
        assert_eq!(result_value["field1"], json!("executed_value1"));
        assert_eq!(result_value["hash"], json!(""));
        assert_eq!(result_value["range"], json!(""));
        assert_eq!(result_value["fields"]["field1"], json!("executed_value1"));
    }
}