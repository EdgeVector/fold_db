//! Result aggregation for transform execution.
//!
//! This module handles the aggregation of execution results from the ExecutionEngine
//! into the final output format for different schema types.

use crate::schema::schema_operations::shape_unified_result;
use crate::schema::types::json_schema::DeclarativeSchemaDefinition;
use crate::schema::types::schema::SchemaType;
use crate::schema::types::SchemaError;
use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::execution_engine::{ExecutionResult, IndexEntry};
use crate::transform::shared_utilities::resolve_field_value_from_chain;
use log::info;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

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
    legacy_fields: serde_json::Map<String, JsonValue>,
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
            legacy_fields: serde_json::Map::new(),
        }
    }

    fn insert_values(&mut self, field_name: &str, values: Vec<JsonValue>, treat_as_array: bool) {
        self.raw_fields
            .insert(field_name.to_string(), values.clone());

        let compat_key = match (&self.schema.schema_type, field_name) {
            (SchemaType::HashRange, "_hash_field") => Some("hash_key"),
            (SchemaType::HashRange, "_range_field") => Some("range_key"),
            (_, name) if !name.starts_with('_') => Some(name),
            _ => None,
        };

        if let Some(key) = compat_key {
            let compat_value = if treat_as_array {
                JsonValue::Array(values)
            } else {
                values.into_iter().next().unwrap_or(JsonValue::Null)
            };
            self.legacy_fields.insert(key.to_string(), compat_value);
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
        let shaped_result =
            shape_unified_result(self.schema, &shaped_input, hash_value, range_value)?;

        let mut final_object = shaped_result
            .as_object()
            .cloned()
            .unwrap_or_else(serde_json::Map::new);

        for (key, value) in self.legacy_fields {
            final_object.entry(key).or_insert(value);
        }

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
    let start_time = Instant::now();
    info!("🔄 Unified aggregation for {:?} schema", schema.schema_type);

    let mut accumulator = AggregationAccumulator::new(schema, all_expressions);

    if execution_result.index_entries.is_empty() {
        info!("⚠️ ExecutionEngine produced empty results, using direct value resolution");
        process_direct_value_resolution(
            schema,
            parsed_chains,
            input_values,
            all_expressions,
            &mut accumulator,
        )?;
    } else {
        info!("✅ Using ExecutionEngine results with aggregation processing");
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
    let duration = start_time.elapsed();
    info!("⏱️ Unified aggregation completed in {:?}", duration);
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
                    info!(
                        "⚠️ Chain resolution failed for field '{}': {}",
                        field_name, err
                    );
                    JsonValue::Null
                }
            }
        } else {
            match crate::transform::shared_utilities::resolve_dotted_path(expression, input_values)
            {
                Ok(value) => value,
                Err(err) => {
                    info!(
                        "⚠️ Direct dotted path resolution failed for field '{}': {}",
                        field_name, err
                    );
                    JsonValue::Null
                }
            }
        };

        accumulator.insert_values(field_name, vec![field_value], false);
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
        SchemaType::HashRange => {
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
                accumulator.insert_values(&field_name, values, true);
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
                                info!(
                                    "⚠️ Fallback resolution failed for field '{}': {}",
                                    field_name, err
                                );
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
                            info!(
                                "⚠️ Direct dotted path resolution failed for field '{}': {}",
                                field_name, err
                            );
                            JsonValue::Null
                        }
                    }
                };

                if !field_name.starts_with('_') {
                    accumulator.insert_values(field_name, vec![field_value], false);
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
    use crate::schema::types::json_schema::{
        DeclarativeSchemaDefinition, FieldDefinition, KeyConfig,
    };
    use crate::schema::types::schema::SchemaType;
    use crate::transform::iterator_stack::chain_parser::ParsedChain;
    use crate::transform::iterator_stack::execution_engine::{ExecutionResult, IndexEntry};
    use serde_json::json;

    #[test]
    fn test_aggregate_results_unified_empty_execution() {
        let mut fields = HashMap::new();
        fields.insert(
            "field1".to_string(),
            FieldDefinition {
                atom_uuid: Some("input.field1".to_string()),
                field_type: Some("String".to_string()),
            },
        );

        let schema = DeclarativeSchemaDefinition {
            name: "single_schema".to_string(),
            schema_type: SchemaType::Single,
            key: None,
            fields,
        };

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
                atom_uuid: Some("input.field1".to_string()),
                field_type: Some("String".to_string()),
            },
        );

        let schema = DeclarativeSchemaDefinition {
            name: "single_schema".to_string(),
            schema_type: SchemaType::Single,
            key: None,
            fields,
        };

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

    #[test]
    fn test_aggregate_results_unified_key_field_filtering() {
        let mut fields = HashMap::new();
        fields.insert(
            "field1".to_string(),
            FieldDefinition {
                atom_uuid: Some("input.field1".to_string()),
                field_type: Some("String".to_string()),
            },
        );

        let schema = DeclarativeSchemaDefinition {
            name: "hashrange_schema".to_string(),
            schema_type: SchemaType::HashRange,
            key: Some(KeyConfig {
                hash_field: "input.hash".to_string(),
                range_field: "input.range".to_string(),
            }),
            fields,
        };

        let parsed_chains = vec![
            (
                "_hash_field".to_string(),
                ParsedChain {
                    operations: vec![
                        crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess(
                            "input".to_string(),
                        ),
                        crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess(
                            "hash".to_string(),
                        ),
                    ],
                    expression: "input.hash".to_string(),
                    depth: 0,
                    branch: "main".to_string(),
                    scopes: vec![],
                },
            ),
            (
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
            ),
        ];

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
                "hash": "hash_value",
                "field1": "value1"
            }),
        )]);

        let all_expressions = vec![
            ("_hash_field".to_string(), "input.hash".to_string()),
            ("field1".to_string(), "input.field1".to_string()),
        ];

        let result = aggregate_results_unified(
            &schema,
            &parsed_chains,
            &execution_result,
            &input_values,
            &all_expressions,
        );

        assert!(result.is_ok());
        let result_value = result.unwrap();

        // Key fields should not be included in final output
        assert!(!result_value
            .as_object()
            .unwrap()
            .contains_key("_hash_field"));
        assert!(result_value.as_object().unwrap().contains_key("field1"));
        assert_eq!(result_value["field1"], json!("value1"));
        assert_eq!(result_value["hash"], json!("hash_value"));
        assert_eq!(result_value["range"], json!(""));
        assert_eq!(result_value["fields"]["field1"], json!("value1"));
    }
}
