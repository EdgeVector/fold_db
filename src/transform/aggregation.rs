//! Result aggregation for transform execution.
//!
//! This module handles the aggregation of execution results from the ExecutionEngine
//! into the final output format for different schema types.

use crate::schema::schema_operations::shape_unified_result;
use crate::schema::types::{
    json_schema::DeclarativeSchemaDefinition,
    schema::{Schema as RuntimeSchema, SchemaType as SchemaVariant},
    SchemaError,
};
use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::execution_engine::{ExecutionResult, IndexEntry};
use crate::transform::shared_utilities::resolve_field_value_from_chain;
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Instant;

/// Schema type for unified aggregation.
#[derive(Debug, Clone, Copy)]
enum AggregationSchemaType {
    Single,
    Range,
    HashRange,
}

/// Extracts optimal field value from execution engine entry.
///
/// # Arguments
///
/// * `entry` - The execution engine entry
///
/// # Returns
///
/// The extracted field value
fn extract_optimal_field_value(entry: &IndexEntry) -> JsonValue {
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
/// * `schema` - The declarative schema definition
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result from ExecutionEngine
/// * `input_values` - The original input values for fallback
/// * `all_expressions` - All expressions (including failed parsing attempts)
///
/// # Returns
///
/// The aggregated result object
pub fn aggregate_results_unified(
    schema: &DeclarativeSchemaDefinition,
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
) -> Result<JsonValue, SchemaError> {
    let start_time = Instant::now();
    let schema_type = match &schema.schema_type {
        SchemaVariant::Single => AggregationSchemaType::Single,
        SchemaVariant::Range { .. } => AggregationSchemaType::Range,
        SchemaVariant::HashRange => AggregationSchemaType::HashRange,
    };
    info!("🔄 Unified aggregation for {:?} schema", schema_type);

    let mut result_object = serde_json::Map::new();

    if execution_result.index_entries.is_empty() {
        info!("⚠️ ExecutionEngine produced empty results, using direct value resolution");
        process_direct_value_resolution(
            parsed_chains,
            input_values,
            all_expressions,
            &mut result_object,
            schema_type,
        )?;
    } else {
        info!("✅ Using ExecutionEngine results with aggregation processing");
        process_execution_result_aggregation(
            parsed_chains,
            execution_result,
            input_values,
            all_expressions,
            &mut result_object,
            schema_type,
        )?;
    }

    let mut data_for_shaping = serde_json::Map::new();
    for (key, value) in &result_object {
        data_for_shaping.insert(key.clone(), value.clone());
    }

    let mut hash_json: Option<JsonValue> = None;
    let mut range_json: Option<JsonValue> = None;

    match schema_type {
        AggregationSchemaType::HashRange => {
            if let Some(value) = result_object.get("hash_key") {
                hash_json = first_entry(value);
            }
            if let Some(value) = result_object.get("range_key") {
                range_json = first_entry(value);
            }
        }
        AggregationSchemaType::Range => {
            if let SchemaVariant::Range { range_key } = &schema.schema_type {
                if let Some(value) = result_object.get(range_key) {
                    range_json = first_entry(value).or_else(|| Some(value.clone()));
                }
            }
        }
        AggregationSchemaType::Single => {}
    }

    if hash_json.is_none() {
        if let Some(key_config) = &schema.key {
            if !key_config.hash_field.trim().is_empty() {
                if let Some(value) = result_object.get(last_segment(&key_config.hash_field)) {
                    hash_json = first_entry(value).or_else(|| Some(value.clone()));
                }
            }
        }
    }

    if range_json.is_none() {
        if let Some(key_config) = &schema.key {
            if !key_config.range_field.trim().is_empty() {
                if let Some(value) = result_object.get(last_segment(&key_config.range_field)) {
                    range_json = first_entry(value).or_else(|| Some(value.clone()));
                }
            }
        }
    }

    let runtime_schema = convert_to_runtime_schema(schema);
    let shaped = shape_unified_result(
        &runtime_schema,
        &JsonValue::Object(data_for_shaping.clone()),
        hash_json.clone(),
        range_json.clone(),
    )?;

    let mut shaped_map = shaped
        .as_object()
        .cloned()
        .unwrap_or_else(serde_json::Map::new);

    let fields_entry = shaped_map
        .entry("fields".to_string())
        .or_insert_with(|| JsonValue::Object(serde_json::Map::new()));
    let fields_obj = fields_entry
        .as_object_mut()
        .expect("fields should be an object");

    for (key, value) in data_for_shaping.iter() {
        fields_obj.entry(key.clone()).or_insert(value.clone());
    }

    if let Some(value) = result_object.get("hash_key") {
        fields_obj.insert("hash_key".to_string(), value.clone());
    }
    if let Some(value) = result_object.get("range_key") {
        fields_obj.insert("range_key".to_string(), value.clone());
    }

    for (key, value) in result_object {
        shaped_map.insert(key, value);
    }

    let result = JsonValue::Object(shaped_map);
    let duration = start_time.elapsed();
    info!("⏱️ Unified aggregation completed in {:?}", duration);
    Ok(result)
}

/// Unified direct value resolution for empty execution results.
///
/// When the ExecutionEngine produces no results, this function directly resolves
/// field values from input data using chain parsing or dotted path resolution.
fn process_direct_value_resolution(
    parsed_chains: &[(String, ParsedChain)],
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
    result_object: &mut serde_json::Map<String, JsonValue>,
    schema_type: AggregationSchemaType,
) -> Result<(), SchemaError> {
    for (field_name, expression) in all_expressions {
        let field_value = if let Some((_, parsed_chain)) =
            parsed_chains.iter().find(|(name, _)| name == field_name)
        {
            // Field was successfully parsed, use chain resolution
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
            // Field failed to parse, try direct dotted path resolution
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

        match schema_type {
            AggregationSchemaType::HashRange => match field_name.as_str() {
                "_hash_field" => {
                    result_object.insert("hash_key".to_string(), field_value);
                }
                "_range_field" => {
                    result_object.insert("range_key".to_string(), field_value);
                }
                _ => {
                    result_object.insert(field_name.clone(), field_value);
                }
            },
            _ => {
                if !field_name.starts_with('_') {
                    result_object.insert(field_name.clone(), field_value);
                }
            }
        }
    }
    Ok(())
}

/// Unified execution result aggregation for successful execution results.
///
/// When the ExecutionEngine produces IndexEntry results, this function aggregates
/// them into the final result object, handling schema-specific field mapping and
/// array creation for HashRange schemas.
fn process_execution_result_aggregation(
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
    result_object: &mut serde_json::Map<String, JsonValue>,
    schema_type: AggregationSchemaType,
) -> Result<(), SchemaError> {
    match schema_type {
        AggregationSchemaType::HashRange => {
            let mut field_arrays: HashMap<String, Vec<JsonValue>> = HashMap::new();

            // Initialize arrays for all fields
            for (field_name, _) in parsed_chains.iter() {
                field_arrays.insert(field_name.clone(), Vec::new());
            }

            // Collect all entries by expression (multiple entries per expression)
            let mut entries_by_expression: HashMap<String, Vec<&IndexEntry>> = HashMap::new();
            for entry in &execution_result.index_entries {
                entries_by_expression
                    .entry(entry.expression.clone())
                    .or_default()
                    .push(entry);
            }

            // Extract values from ExecutionEngine index entries for each field
            for (field_name, parsed_chain) in parsed_chains.iter() {
                if let Some(entries) = entries_by_expression.get(&parsed_chain.expression) {
                    for entry in entries {
                        let field_value = extract_optimal_field_value(entry);

                        if field_name == "_hash_field" {
                            field_arrays
                                .entry("_hash_field".to_string())
                                .or_default()
                                .push(field_value);
                        } else if field_name == "_range_field" {
                            field_arrays
                                .entry("_range_field".to_string())
                                .or_default()
                                .push(field_value);
                        } else {
                            field_arrays
                                .entry(field_name.clone())
                                .or_default()
                                .push(field_value);
                        }
                    }
                }
            }

            let hash_key_array = field_arrays.remove("_hash_field").unwrap_or_default();
            let range_key_array = field_arrays.remove("_range_field").unwrap_or_default();

            result_object.insert("hash_key".to_string(), JsonValue::Array(hash_key_array));
            result_object.insert("range_key".to_string(), JsonValue::Array(range_key_array));

            for (field_name, field_array) in field_arrays {
                result_object.insert(field_name, JsonValue::Array(field_array));
            }
        }
        _ => {
            let mut entries_by_expression: HashMap<String, &IndexEntry> = HashMap::new();
            for entry in &execution_result.index_entries {
                entries_by_expression.insert(entry.expression.clone(), entry);
            }

            // Process all fields, including those that failed to parse
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
                    result_object.insert(field_name.clone(), field_value);
                }
            }
        }
    }

    Ok(())
}

fn convert_to_runtime_schema(schema: &DeclarativeSchemaDefinition) -> RuntimeSchema {
    let mut runtime_schema = RuntimeSchema::new(schema.name.clone());
    runtime_schema.schema_type = schema.schema_type.clone();
    runtime_schema.key = schema.key.clone();
    runtime_schema
}

fn first_entry(value: &JsonValue) -> Option<JsonValue> {
    match value {
        JsonValue::Array(values) => values.first().cloned(),
        JsonValue::Null => None,
        _ => Some(value.clone()),
    }
}

fn last_segment(expression: &str) -> &str {
    expression.rsplit('.').next().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::json_schema::{FieldDefinition, KeyConfig};
    use crate::schema::types::schema::SchemaType;
    use crate::transform::iterator_stack::chain_parser::{ChainOperation, ParsedChain};
    use crate::transform::iterator_stack::execution_engine::{ExecutionResult, IndexEntry};
    use serde_json::json;

    fn build_single_schema() -> DeclarativeSchemaDefinition {
        DeclarativeSchemaDefinition {
            name: "single_schema".to_string(),
            schema_type: SchemaType::Single,
            key: None,
            fields: HashMap::from([(
                "field1".to_string(),
                FieldDefinition {
                    atom_uuid: Some("input.field1".to_string()),
                    field_type: Some("String".to_string()),
                },
            )]),
        }
    }

    fn build_hashrange_schema() -> DeclarativeSchemaDefinition {
        DeclarativeSchemaDefinition {
            name: "hashrange_schema".to_string(),
            schema_type: SchemaType::HashRange,
            key: Some(KeyConfig {
                hash_field: "input.hash_field".to_string(),
                range_field: "input.range_field".to_string(),
            }),
            fields: HashMap::from([(
                "value".to_string(),
                FieldDefinition {
                    atom_uuid: Some("input.value".to_string()),
                    field_type: Some("String".to_string()),
                },
            )]),
        }
    }

    #[test]
    fn test_aggregate_results_unified_empty_execution() {
        let schema = build_single_schema();

        let parsed_chains = vec![(
            "field1".to_string(),
            ParsedChain {
                operations: vec![
                    ChainOperation::FieldAccess("input".to_string()),
                    ChainOperation::FieldAccess("field1".to_string()),
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
        assert_eq!(result_value["hash"], json!(""));
        assert_eq!(result_value["range"], json!(""));
        assert_eq!(result_value["field1"], json!("value1"));
        let fields = result_value["fields"].as_object().expect("fields map");
        assert_eq!(fields.get("field1"), Some(&json!("value1")));
    }

    #[test]
    fn test_aggregate_results_unified_with_execution() {
        let schema = build_single_schema();

        let parsed_chains = vec![(
            "field1".to_string(),
            ParsedChain {
                operations: vec![
                    ChainOperation::FieldAccess("input".to_string()),
                    ChainOperation::FieldAccess("field1".to_string()),
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
                hash_value: json!("value1"),
                range_value: json!("value1"),
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
                "field1": "fallback"
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
        assert_eq!(result_value["hash"], json!(""));
        assert_eq!(result_value["range"], json!(""));
        assert_eq!(result_value["field1"], json!("value1"));
        let fields = result_value["fields"].as_object().expect("fields map");
        assert_eq!(fields.get("field1"), Some(&json!("value1")));
    }

    #[test]
    fn test_aggregate_results_unified_hashrange_arrays() {
        let schema = build_hashrange_schema();

        let parsed_chains = vec![
            (
                "_hash_field".to_string(),
                ParsedChain {
                    operations: vec![
                        ChainOperation::FieldAccess("input".to_string()),
                        ChainOperation::FieldAccess("hash_field".to_string()),
                    ],
                    expression: "input.hash_field".to_string(),
                    depth: 0,
                    branch: "main".to_string(),
                    scopes: vec![],
                },
            ),
            (
                "_range_field".to_string(),
                ParsedChain {
                    operations: vec![
                        ChainOperation::FieldAccess("input".to_string()),
                        ChainOperation::FieldAccess("range_field".to_string()),
                    ],
                    expression: "input.range_field".to_string(),
                    depth: 0,
                    branch: "main".to_string(),
                    scopes: vec![],
                },
            ),
            (
                "value".to_string(),
                ParsedChain {
                    operations: vec![
                        ChainOperation::FieldAccess("input".to_string()),
                        ChainOperation::FieldAccess("value".to_string()),
                    ],
                    expression: "input.value".to_string(),
                    depth: 0,
                    branch: "main".to_string(),
                    scopes: vec![],
                },
            ),
        ];

        let execution_result = ExecutionResult {
            index_entries: vec![
                IndexEntry {
                    expression: "input.hash_field".to_string(),
                    hash_value: json!("hash_a"),
                    range_value: json!("range_a"),
                    atom_uuid: "hash-entry-a".to_string(),
                    metadata: HashMap::new(),
                },
                IndexEntry {
                    expression: "input.hash_field".to_string(),
                    hash_value: json!("hash_b"),
                    range_value: json!("range_b"),
                    atom_uuid: "hash-entry-b".to_string(),
                    metadata: HashMap::new(),
                },
                IndexEntry {
                    expression: "input.range_field".to_string(),
                    hash_value: json!("range_a"),
                    range_value: json!("range_a"),
                    atom_uuid: "range-entry-a".to_string(),
                    metadata: HashMap::new(),
                },
                IndexEntry {
                    expression: "input.range_field".to_string(),
                    hash_value: json!("range_b"),
                    range_value: json!("range_b"),
                    atom_uuid: "range-entry-b".to_string(),
                    metadata: HashMap::new(),
                },
                IndexEntry {
                    expression: "input.value".to_string(),
                    hash_value: json!("data_a"),
                    range_value: json!("data_a"),
                    atom_uuid: "value-entry-a".to_string(),
                    metadata: HashMap::new(),
                },
                IndexEntry {
                    expression: "input.value".to_string(),
                    hash_value: json!("data_b"),
                    range_value: json!("data_b"),
                    atom_uuid: "value-entry-b".to_string(),
                    metadata: HashMap::new(),
                },
            ],
            statistics:
                crate::transform::iterator_stack::execution_engine::core::ExecutionStatistics {
                    total_entries: 6,
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
                "hash_field": "hash_a",
                "range_field": "range_a",
                "value": "data_a"
            }),
        )]);

        let all_expressions = vec![
            ("_hash_field".to_string(), "input.hash_field".to_string()),
            ("_range_field".to_string(), "input.range_field".to_string()),
            ("value".to_string(), "input.value".to_string()),
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
        assert_eq!(result_value["hash"], json!("hash_a"));
        assert_eq!(result_value["range"], json!("range_a"));
        assert_eq!(result_value["hash_key"], json!(["hash_a", "hash_b"]));
        assert_eq!(result_value["range_key"], json!(["range_a", "range_b"]));
        assert_eq!(result_value["value"], json!(["data_a", "data_b"]));
        let fields = result_value["fields"].as_object().expect("fields map");
        assert_eq!(fields.get("hash_key"), Some(&json!(["hash_a", "hash_b"])));
        assert_eq!(
            fields.get("range_key"),
            Some(&json!(["range_a", "range_b"]))
        );
        assert_eq!(fields.get("value"), Some(&json!(["data_a", "data_b"])));
    }
}
