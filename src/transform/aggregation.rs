//! Result aggregation for transform execution.
//!
//! This module handles the aggregation of execution results from the ExecutionEngine
//! into the final output format for different schema types. It provides a unified
//! interface for processing both direct value resolution and execution result aggregation.

use crate::schema::types::{DeclarativeSchemaDefinition, SchemaError, KeyConfig, KeyValue};
use crate::schema::types::field::FieldValue;
use crate::schema::types::schema::SchemaType;
use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::result_types::{ExecutionResult, IndexEntry};
use crate::transform::shared_utilities::resolve_field_value_from_chain;
use crate::fold_db_core::query::formatter::Record;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Constants (legacy fallbacks only used when needed)
const HASH_FIELD_PREFIX: &str = "_hash_field"; // legacy fallback, will be removed
const RANGE_FIELD_PREFIX: &str = "_range_field"; // legacy fallback, will be removed

/// Extracts the optimal field value from an execution engine entry.
///
/// For now, this returns the hash_value as the primary value, but could be
/// enhanced to choose between hash_value and range_value based on context.
///
/// # Arguments
///
/// * `entry` - The execution engine entry to extract value from
///
/// # Returns
///
/// The extracted field value as JSON
fn extract_optimal_field_value(entry: &IndexEntry) -> JsonValue { serde_json::to_value(&entry.value).unwrap_or(JsonValue::Null) }

/// Aggregation accumulator that builds the final result structure.
///
/// This struct encapsulates the logic for collecting field values and
/// constructing the final hash->range->fields structure.
struct AggregationAccumulator<'a> {
    #[allow(dead_code)]
    schema: &'a DeclarativeSchemaDefinition,
    expressions: HashMap<String, String>,
    // row_id -> (field_name -> collected values)
    raw_rows: HashMap<String, HashMap<String, Vec<JsonValue>>>,
}

impl<'a> AggregationAccumulator<'a> {
    /// Creates a new aggregation accumulator.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema definition being processed
    /// * `expressions` - Field name to expression mapping
    ///
    /// # Returns
    ///
    /// A new accumulator instance
    fn new(schema: &'a DeclarativeSchemaDefinition, expressions: &[(String, String)]) -> Self {
        let expression_map = expressions
            .iter()
            .map(|(field, expr)| (field.clone(), expr.clone()))
            .collect();

        Self { schema, expressions: expression_map, raw_rows: HashMap::new() }
    }

    /// Finalizes the aggregation and returns the structured list of rows.
    ///
    /// # Returns
    ///
    /// The final aggregated result in hash->range->fields format
    fn finalize(self) -> Result<JsonValue, SchemaError> {
        let mut rows_array: Vec<JsonValue> = Vec::new();

        for fields_map in self.raw_rows.values() {
            let mut shaped_fields = serde_json::Map::new();

            for (field_name, values) in fields_map {
                // Preserve original declared field name for row shaping
                let value = self.format_field_value(values);
                shaped_fields.insert(field_name.clone(), value);
            }

            // Ensure all declared transform fields are present in each row
            // If a field did not produce a value for this row, throw an error
            for declared_field in self.expressions.keys() {
                if !shaped_fields.contains_key(declared_field) {
                    return Err(SchemaError::InvalidData(format!(
                        "Missing required field '{}' in aggregation result. All declared transform fields must be present in each row.",
                        declared_field
                    )));
                }
            }

            let key = self.derive_key_from_row(&shaped_fields);

            let mut row_obj = serde_json::Map::new();
            row_obj.insert("key".to_string(), serde_json::to_value(&key).unwrap());
            row_obj.insert("fields".to_string(), JsonValue::Object(shaped_fields));
            rows_array.push(JsonValue::Object(row_obj));
        }

        Ok(JsonValue::Array(rows_array))
    }

    /// Derive hash/range values using KeyConfig if available; fallback to legacy internal fields.
    fn derive_key_from_row(&self, row_fields: &serde_json::Map<String, JsonValue>) -> KeyValue {
        let mut hash_value: Option<String> = None;
        let mut range_value: Option<String> = None;

        if let Some(KeyConfig { hash_field, range_field }) = &self.schema.key {
            if let Some(hf) = hash_field {
                if let Some(v) = row_fields.get(hf) {
                    hash_value = convert_json_to_string(v);
                }
            }
            if let Some(rf) = range_field {
                if let Some(v) = row_fields.get(rf) {
                    range_value = convert_json_to_string(v);
                }
            }
        }

        // Fallbacks for older tests
        if hash_value.is_none() {
            if let Some(v) = row_fields.get(HASH_FIELD_PREFIX) {
                hash_value = convert_json_to_string(v);
            }
        }
        if range_value.is_none() {
            if let Some(v) = row_fields.get(RANGE_FIELD_PREFIX) {
                range_value = convert_json_to_string(v);
            }
        }

        KeyValue::new(hash_value, range_value)
    }


    /// Formats field values into the appropriate JSON structure.
    ///
    /// # Arguments
    ///
    /// * `values` - Vector of field values
    ///
    /// # Returns
    ///
    /// Single value or array of values as appropriate
    fn format_field_value(&self, values: &[JsonValue]) -> JsonValue { if values.len() == 1 { values[0].clone() } else { JsonValue::Array(values.to_vec()) } }

    
}

/// Main aggregation function that handles all aggregation patterns.
///
/// This function consolidates the duplicate aggregation logic that was previously
/// scattered across multiple executor modules, providing a unified interface for
/// processing execution results.
///
/// # Arguments
///
/// * `schema` - The schema definition being processed
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result from ExecutionEngine
/// * `input_values` - The original input values for fallback
/// * `all_expressions` - All expressions (including failed parsing attempts)
///
/// # Returns
///
/// The aggregated result object in hash->range->fields format
pub fn aggregate_results_unified_typed(
    schema: &DeclarativeSchemaDefinition,
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, HashMap<KeyValue, FieldValue>>,
    all_expressions: &[(String, String)],
) -> Result<JsonValue, SchemaError> {
    let mut accumulator = AggregationAccumulator::new(schema, all_expressions);

    if execution_result.index_entries.values().all(|entries| entries.is_empty()) {
        process_direct_value_resolution_typed(
            parsed_chains,
            input_values,
            all_expressions,
            &mut accumulator,
        )?;
    } else {
        process_execution_result_aggregation_typed(
            schema,
            parsed_chains,
            execution_result,
            input_values,
            all_expressions,
            &mut accumulator,
        )?;
    }

    accumulator.finalize()
}

/// Variant of aggregation that returns structured Record instead of JSON
pub fn aggregate_results_unified_typed_as_records(
    schema: &DeclarativeSchemaDefinition,
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, HashMap<KeyValue, FieldValue>>,
    all_expressions: &[(String, String)],
) -> Result<Vec<Record>, SchemaError> {
    let json_result = aggregate_results_unified_typed(
        schema,
        parsed_chains,
        execution_result,
        input_values,
        all_expressions,
    )?;
    
    // Convert JSON array to Vec<Record>
    let mut records = Vec::new();
    if let Some(result_array) = json_result.as_array() {
        for row in result_array {
            if let Some(fields_obj) = row.get("fields").and_then(|f| f.as_object()) {
                let mut fields = HashMap::new();
                for (field_name, field_value) in fields_obj {
                    fields.insert(field_name.clone(), field_value.clone());
                }
                records.push(Record { fields });
            }
        }
    }
    
    Ok(records)
}

/// Processes direct value resolution when no execution results are available.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `input_values` - The typed input values to resolve from
/// * `all_expressions` - All expressions to process
/// * `accumulator` - The accumulator to collect results in
///
/// # Returns
///
/// Result indicating success or failure
fn process_direct_value_resolution_typed(
    parsed_chains: &[(String, ParsedChain)],
    input_values: &HashMap<String, HashMap<KeyValue, FieldValue>>,
    all_expressions: &[(String, String)],
    accumulator: &mut AggregationAccumulator,
) -> Result<(), SchemaError> {
    let mut row_fields: HashMap<String, Vec<JsonValue>> = HashMap::new();
    for (field_name, expression) in all_expressions {
        let field_value = resolve_field_value_typed(
            field_name,
            expression,
            parsed_chains,
            input_values,
        )?;
        row_fields.insert(field_name.clone(), vec![field_value]);
    }
    accumulator.raw_rows = HashMap::from([(String::from("_single_row"), row_fields)]);

    Ok(())
}

/// Processes execution result aggregation for different schema types.
///
/// # Arguments
///
/// * `schema` - The schema definition being processed
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result to process
/// * `input_values` - The typed input values for fallback
/// * `all_expressions` - All expressions to process
/// * `accumulator` - The accumulator to collect results in
///
/// # Returns
///
/// Result indicating success or failure
fn process_execution_result_aggregation_typed(
    schema: &DeclarativeSchemaDefinition,
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, HashMap<KeyValue, FieldValue>>,
    all_expressions: &[(String, String)],
    accumulator: &mut AggregationAccumulator,
) -> Result<(), SchemaError> {
    match &schema.schema_type {
        SchemaType::HashRange { .. } => {
            process_hash_range_aggregation(
                parsed_chains,
                execution_result,
                accumulator,
            )?;
        }
        _ => {
            process_single_range_aggregation_typed(
                parsed_chains,
                execution_result,
                input_values,
                all_expressions,
                accumulator,
            )?;
        }
    }

    Ok(())
}

/// Processes aggregation for HashRange schema types.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result to process
/// * `accumulator` - The accumulator to collect results in
///
/// # Returns
///
/// Result indicating success or failure
fn process_hash_range_aggregation(
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    accumulator: &mut AggregationAccumulator,
) -> Result<(), SchemaError> {
    // Group entries by row_id
    let mut rows: HashMap<String, HashMap<String, Vec<JsonValue>>> = HashMap::new();

    // Build a map from expression to output field name
    let mut expr_to_field: HashMap<String, String> = HashMap::new();
    for (field_name, parsed_chain) in parsed_chains.iter() {
        expr_to_field.insert(parsed_chain.expression.clone(), field_name.clone());
    }

    for entries in execution_result.index_entries.values() {
        for entry in entries {
            let row = rows.entry(entry.row_id.clone()).or_default();
            if let Some(field_name) = expr_to_field.get(&entry.expression) {
                row.entry(field_name.clone()).or_default().push(extract_optimal_field_value(entry));
            }
        }
    }

    // Ensure every row contains all fields by implementing bidirectional inheritance
    // This handles both parent-to-child and child-to-parent field propagation
    
    // Step 1: Child-to-parent inheritance (children inherit from parents)
    let mut row_ids: Vec<String> = rows.keys().cloned().collect();
    // Sort by depth (number of segments) ascending so parents come first
    row_ids.sort_by_key(|id| id.split('/').count());

    // Build a quick lookup to avoid multiple clones
    let rows_clone = rows.clone();
    for child_id in row_ids.iter() {
        let child_fields = rows.get_mut(child_id).unwrap();
        let segments: Vec<&str> = child_id.split('/').collect();
        if segments.len() <= 1 { continue; }
        
        // Try to inherit from all possible parent prefixes
        for prefix_len in (1..segments.len()).rev() {
            let prefix = segments[..prefix_len].join("/");
            if let Some(parent_fields) = rows_clone.get(&prefix) {
                for (fname, fvals) in parent_fields {
                    child_fields.entry(fname.clone()).or_insert_with(|| fvals.clone());
                }
            }
        }
        
        // Special case: also try to inherit from the root parent (first segment only)
        if segments.len() > 1 {
            let root_parent = segments[0].to_string();
            if let Some(parent_fields) = rows_clone.get(&root_parent) {
                for (fname, fvals) in parent_fields {
                    child_fields.entry(fname.clone()).or_insert_with(|| fvals.clone());
                }
            }
        }
    }

    // For HashRange schemas, filter out parent rows that have children
    // This prevents duplicates in word-splitting scenarios where parent rows are just containers
    let mut filtered_rows = HashMap::new();
    for (row_id, row_fields) in rows.iter() {
        let segments: Vec<&str> = row_id.split('/').collect();
        
        // Check if this is a parent row (single segment) that has children
        if segments.len() == 1 {
            let has_children = rows.keys().any(|id| id.starts_with(&format!("{}/", row_id)));
            
            if has_children {
                // This is a parent row with children - skip it as the children will inherit its fields
                // This prevents duplicates in word-splitting scenarios
                continue;
            }
        }
        
        // Keep this row (either child rows or parent rows without children)
        filtered_rows.insert(row_id.clone(), row_fields.clone());
    }
    
    rows = filtered_rows;

    // CRITICAL FIX: Ensure every row contains ALL declared transform fields
    // If a field did not produce a value for this row, throw an error
    for (row_id, row_fields) in rows.iter() {
        for declared_field in accumulator.expressions.keys() {
            if !row_fields.contains_key(declared_field) {
                return Err(SchemaError::InvalidData(format!(
                    "Missing required field '{}' in row '{}' for HashRange aggregation. All declared transform fields must be present in each row.",
                    declared_field, row_id
                )));
            }
        }
    }

    accumulator.raw_rows = rows;

    Ok(())
}

/// Processes aggregation for Single and Range schema types.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result to process
/// * `input_values` - The typed input values for fallback
/// * `all_expressions` - All expressions to process
/// * `accumulator` - The accumulator to collect results in
///
/// # Returns
///
/// Result indicating success or failure
fn process_single_range_aggregation_typed(
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, HashMap<KeyValue, FieldValue>>,
    all_expressions: &[(String, String)],
    accumulator: &mut AggregationAccumulator,
) -> Result<(), SchemaError> {
    // Build a single synthetic row for Single/Range schema types
    let mut row_fields: HashMap<String, Vec<JsonValue>> = HashMap::new();

    // Create a map of expression to entry for quick lookup
    let mut entries_by_expression: HashMap<String, &IndexEntry> = HashMap::new();
    for entries in execution_result.index_entries.values() {
        for entry in entries {
            entries_by_expression.insert(entry.expression.clone(), entry);
        }
    }

    for (field_name, expression) in all_expressions {
        let field_value = if let Some((_, parsed_chain)) = parsed_chains.iter().find(|(name, _)| name == field_name) {
            if let Some(entry) = entries_by_expression.get(&parsed_chain.expression) {
                extract_optimal_field_value(entry)
            } else {
                resolve_field_value_from_chain_typed(parsed_chain, input_values, field_name).unwrap_or(JsonValue::Null)
            }
        } else {
            resolve_field_value_typed(field_name, expression, parsed_chains, input_values).unwrap_or(JsonValue::Null)
        };

        if !field_name.starts_with('_') {
            row_fields.insert(field_name.clone(), vec![field_value]);
        }
    }

    // Use a constant row id since Single/Range produce one row
    accumulator.raw_rows = HashMap::from([(String::from("_single_row"), row_fields)]);

    Ok(())
}

/// Resolves a field value using the most appropriate method with typed input.
///
/// # Arguments
///
/// * `field_name` - The name of the field
/// * `expression` - The expression to resolve
/// * `parsed_chains` - Available parsed chains
/// * `input_values` - Typed input values for fallback
///
/// # Returns
///
/// The resolved field value
fn resolve_field_value_typed(
    field_name: &str,
    expression: &str,
    parsed_chains: &[(String, ParsedChain)],
    input_values: &HashMap<String, HashMap<KeyValue, FieldValue>>,
) -> Result<JsonValue, SchemaError> {
    if let Some((_, parsed_chain)) = parsed_chains.iter().find(|(name, _)| name == field_name) {
        resolve_field_value_from_chain_typed(parsed_chain, input_values, field_name)
    } else {
        // Convert typed input to JSON for dotted path resolution
        let json_input = convert_typed_input_to_json(input_values);
        crate::transform::shared_utilities::resolve_dotted_path(expression, &json_input)
    }
}

/// Resolves a field value from a parsed chain with typed input.
///
/// # Arguments
///
/// * `parsed_chain` - The parsed chain to resolve
/// * `input_values` - Typed input values
/// * `field_name` - The field name for error reporting
///
/// # Returns
///
/// The resolved field value
fn resolve_field_value_from_chain_typed(
    parsed_chain: &ParsedChain,
    input_values: &HashMap<String, HashMap<KeyValue, FieldValue>>,
    field_name: &str,
) -> Result<JsonValue, SchemaError> {
    // Convert typed input to JSON for chain resolution
    let json_input = convert_typed_input_to_json(input_values);
    resolve_field_value_from_chain(parsed_chain, &json_input, field_name)
}

/// Converts a JSON value to a string representation.
///
/// # Arguments
///
/// * `value` - The JSON value to convert
///
/// # Returns
///
/// String representation if convertible, None otherwise
fn convert_json_to_string(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(s) => Some(s.clone()),
        JsonValue::Number(n) => Some(n.to_string()),
        JsonValue::Bool(b) => Some(b.to_string()),
        JsonValue::Null => None,
        JsonValue::Array(arr) => arr.first().and_then(convert_json_to_string),
        JsonValue::Object(_) => Some(value.to_string()),
    }
}

/// Convert typed input HashMap<String, HashMap<KeyValue, FieldValue>> to JSON for fallback resolution paths
fn convert_typed_input_to_json(
    input_values: &HashMap<String, HashMap<KeyValue, FieldValue>>,
) -> HashMap<String, JsonValue> {
    let mut grouped: HashMap<String, serde_json::Map<String, JsonValue>> = HashMap::new();

    for (key, kv_map) in input_values.iter() {
        let (schema_name, field_name) = if let Some(dot) = key.find('.') {
            (&key[..dot], &key[dot + 1..])
        } else {
            (key.as_str(), "_root")
        };

        let values: Vec<JsonValue> = kv_map.values().map(|fv| fv.value.clone()).collect();
        let field_json = if values.len() == 1 { values[0].clone() } else { JsonValue::Array(values) };

        let entry = grouped.entry(schema_name.to_string()).or_default();
        entry.insert(field_name.to_string(), field_json);
    }

    grouped
        .into_iter()
        .map(|(schema, map)| (schema, JsonValue::Object(map)))
        .collect()
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::DeclarativeSchemaDefinition;
    use crate::schema::types::schema::SchemaType;
    use crate::transform::iterator_stack::chain_parser::{ChainOperation, ParsedChain};
    use crate::transform::result_types::{ExecutionResult, IndexEntry};
    use crate::schema::types::field::FieldValue;
    use crate::schema::types::key_value::KeyValue;
    use serde_json::json;
    use serde_json::Value as JsonValue;
    use std::collections::HashMap;

    /// Helper function to build a parsed chain for testing
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

    /// Helper function to build an index entry for testing
    fn build_index_entry(expression: &str, value: JsonValue) -> IndexEntry {
        IndexEntry {
            row_id: "row1".to_string(),
            value,
            atom_uuid: "test-uuid".to_string(),
            metadata: HashMap::new(),
            expression: expression.to_string(),
        }
    }


    #[test]
    fn test_aggregate_results_unified_empty_execution() {
        let mut transform_fields = HashMap::new();
        transform_fields.insert("field1".to_string(), "input.field1".to_string());

        let schema = DeclarativeSchemaDefinition::new(
            "single_schema".to_string(),
            SchemaType::Single,
            None,
            None,
            Some(transform_fields),
        );

        let parsed_chains = vec![
            (
                "field1".to_string(),
                build_parsed_chain("input.field1", &["input", "field1"]),
            )
        ];

        let execution_result = ExecutionResult {
            index_entries: HashMap::new(),
            warnings: HashMap::new(),
        };

        // typed input: key -> map<KeyValue, FieldValue>
        let mut typed_input: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();
        let mut field_map: HashMap<KeyValue, FieldValue> = HashMap::new();
        field_map.insert(KeyValue::new(None, None), FieldValue { value: json!("value1"), atom_uuid: "a1".to_string() });
        typed_input.insert("input.field1".to_string(), field_map);

        let all_expressions = vec![("field1".to_string(), "input.field1".to_string())];

        let result = aggregate_results_unified_typed(
            &schema,
            &parsed_chains,
            &execution_result,
            &typed_input,
            &all_expressions,
        );

        assert!(result.is_ok());
        let result_value = result.unwrap();
        let arr = result_value.as_array().expect("rows array");
        assert_eq!(arr.len(), 1);
        let row = arr[0].as_object().unwrap();
        assert!(row.contains_key("key"));
        assert_eq!(row["fields"]["field1"], json!("value1"));
    }

    #[test]
    fn test_aggregate_results_unified_with_execution() {
        let mut transform_fields = HashMap::new();
        transform_fields.insert("field1".to_string(), "input.field1".to_string());

        let schema = DeclarativeSchemaDefinition::new(
            "single_schema".to_string(),
            SchemaType::Single,
            None,
            None,
            Some(transform_fields),
        );

        let parsed_chains = vec![
            (
                "field1".to_string(),
                build_parsed_chain("input.field1", &["input", "field1"]),
            )
        ];

        let mut index_entries = HashMap::new();
        index_entries.insert("input.field1".to_string(), vec![build_index_entry("input.field1", json!("executed_value1"))]);
        let execution_result = ExecutionResult {
            index_entries,
            warnings: HashMap::new(),
        };

        let typed_input: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();

        let all_expressions = vec![("field1".to_string(), "input.field1".to_string())];

        let result = aggregate_results_unified_typed(
            &schema,
            &parsed_chains,
            &execution_result,
            &typed_input,
            &all_expressions,
        );

        assert!(result.is_ok());
        let result_value = result.unwrap();
        let arr = result_value.as_array().expect("rows array");
        assert_eq!(arr.len(), 1);
        let row = arr[0].as_object().unwrap();
        assert!(row.contains_key("key"));
        assert_eq!(row["fields"]["field1"], json!("executed_value1"));
    }

    #[test]
    fn test_hash_range_aggregation() {
        let mut transform_fields = HashMap::new();
        transform_fields.insert("field1".to_string(), "input.field1".to_string());

        let schema = DeclarativeSchemaDefinition::new(
            "hash_range_schema".to_string(),
            SchemaType::HashRange { 
                keyconfig: crate::schema::types::key_config::KeyConfig::new(None, None)
            },
            None,
            None,
            Some(transform_fields),
        );

        let parsed_chains = vec![
            (
                "field1".to_string(),
                build_parsed_chain("input.field1", &["input", "field1"]),
            )
        ];

        let mut index_entries = HashMap::new();
        index_entries.insert("input.field1".to_string(), vec![
            build_index_entry("input.field1", json!("value1")),
            build_index_entry("input.field1", json!("value2")),
        ]);
        let execution_result = ExecutionResult {
            index_entries,
            warnings: HashMap::new(),
        };

        let typed_input: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();
        let all_expressions = vec![("field1".to_string(), "input.field1".to_string())];

        let result = aggregate_results_unified_typed(
            &schema,
            &parsed_chains,
            &execution_result,
            &typed_input,
            &all_expressions,
        );

        assert!(result.is_ok());
        let result_value = result.unwrap();
        let arr = result_value.as_array().expect("rows array");
        assert_eq!(arr.len(), 1);
        let row = arr[0].as_object().unwrap();
        assert!(row.contains_key("key"));
        assert_eq!(row["fields"]["field1"], json!(["value1", "value2"]));
    }

    #[test]
    fn test_hash_range_aggregation_missing_field_throws_error() {
        let mut transform_fields = HashMap::new();
        transform_fields.insert("field1".to_string(), "input.field1".to_string());
        transform_fields.insert("field2".to_string(), "input.field2".to_string());

        let schema = DeclarativeSchemaDefinition::new(
            "hash_range_schema".to_string(),
            SchemaType::HashRange { 
                keyconfig: crate::schema::types::key_config::KeyConfig::new(None, None)
            },
            None,
            None,
            Some(transform_fields),
        );

        let parsed_chains = vec![
            (
                "field1".to_string(),
                build_parsed_chain("input.field1", &["input", "field1"]),
            )
        ];

        let mut index_entries = HashMap::new();
        index_entries.insert("input.field1".to_string(), vec![
            build_index_entry("input.field1", json!("value1")),
        ]);
        let execution_result = ExecutionResult {
            index_entries,
            warnings: HashMap::new(),
        };

        let typed_input: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();
        let all_expressions = vec![
            ("field1".to_string(), "input.field1".to_string()),
            ("field2".to_string(), "input.field2".to_string()),
        ];

        let result = aggregate_results_unified_typed(
            &schema,
            &parsed_chains,
            &execution_result,
            &typed_input,
            &all_expressions,
        );

        // Should fail because field2 is declared but not present in execution result
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Missing required field 'field2'"));
    }

    #[test]
    fn test_hash_range_aggregation_all_fields_present_success() {
        // Test that when all fields are properly inherited, aggregation succeeds
        let mut transform_fields = HashMap::new();
        transform_fields.insert("word".to_string(), "input.content.split_by_word()".to_string());
        transform_fields.insert("author".to_string(), "input.author".to_string());
        transform_fields.insert("title".to_string(), "input.title".to_string());

        let schema = DeclarativeSchemaDefinition::new(
            "hash_range_schema".to_string(),
            SchemaType::HashRange { 
                keyconfig: crate::schema::types::key_config::KeyConfig::new(None, None)
            },
            None,
            None,
            Some(transform_fields),
        );

        let parsed_chains = vec![
            (
                "word".to_string(),
                build_parsed_chain("input.content.split_by_word()", &["input", "content"]),
            ),
            (
                "author".to_string(),
                build_parsed_chain("input.author", &["input", "author"]),
            ),
            (
                "title".to_string(),
                build_parsed_chain("input.title", &["input", "title"]),
            ),
        ];

        // Simulate execution results with word splitting and field inheritance
        // All fields must be present in all rows for the aggregation to succeed
        let mut index_entries = HashMap::new();
        index_entries.insert("input.content.split_by_word()".to_string(), vec![
            build_index_entry_with_row_id("input.content.split_by_word()", json!("hello"), "0/0"),
            build_index_entry_with_row_id("input.content.split_by_word()", json!("world"), "0/1"),
        ]);
        index_entries.insert("input.author".to_string(), vec![
            build_index_entry_with_row_id("input.author", json!("John Doe"), "0/0"),
            build_index_entry_with_row_id("input.author", json!("John Doe"), "0/1"),
        ]);
        index_entries.insert("input.title".to_string(), vec![
            build_index_entry_with_row_id("input.title", json!("Test Post"), "0/0"),
            build_index_entry_with_row_id("input.title", json!("Test Post"), "0/1"),
        ]);

        let execution_result = ExecutionResult {
            index_entries,
            warnings: HashMap::new(),
        };

        let typed_input: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();
        let all_expressions = vec![
            ("word".to_string(), "input.content.split_by_word()".to_string()),
            ("author".to_string(), "input.author".to_string()),
            ("title".to_string(), "input.title".to_string()),
        ];

        let result = aggregate_results_unified_typed(
            &schema,
            &parsed_chains,
            &execution_result,
            &typed_input,
            &all_expressions,
        );

        // Should succeed because all fields are inherited properly
        assert!(result.is_ok());
        let result_value = result.unwrap();
        let arr = result_value.as_array().expect("rows array");
        assert_eq!(arr.len(), 2); // Two word rows
        
        // Check that both word rows have all fields inherited
        for row in arr {
            let row_obj = row.as_object().unwrap();
            let fields = row_obj.get("fields").unwrap().as_object().unwrap();
            assert!(fields.contains_key("word"));
            assert!(fields.contains_key("author"));
            assert!(fields.contains_key("title"));
            assert_eq!(fields.get("author").unwrap(), &json!("John Doe"));
            assert_eq!(fields.get("title").unwrap(), &json!("Test Post"));
        }
    }

    #[test]
    fn test_single_range_aggregation_missing_field_throws_error() {
        // Test that Single/Range aggregation also throws errors for missing fields
        let mut transform_fields = HashMap::new();
        transform_fields.insert("field1".to_string(), "input.field1".to_string());
        transform_fields.insert("field2".to_string(), "input.field2".to_string());

        let schema = DeclarativeSchemaDefinition::new(
            "single_schema".to_string(),
            SchemaType::Single,
            None,
            None,
            Some(transform_fields),
        );

        let parsed_chains = vec![
            (
                "field1".to_string(),
                build_parsed_chain("input.field1", &["input", "field1"]),
            )
        ];

        let execution_result = ExecutionResult {
            index_entries: HashMap::new(),
            warnings: HashMap::new(),
        };

        // typed input: key -> map<KeyValue, FieldValue>
        let mut typed_input: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();
        let mut field_map: HashMap<KeyValue, FieldValue> = HashMap::new();
        field_map.insert(KeyValue::new(None, None), FieldValue { value: json!("value1"), atom_uuid: "a1".to_string() });
        typed_input.insert("input.field1".to_string(), field_map);

        let all_expressions = vec![
            ("field1".to_string(), "input.field1".to_string()),
            ("field2".to_string(), "input.field2".to_string()),
        ];

        let result = aggregate_results_unified_typed(
            &schema,
            &parsed_chains,
            &execution_result,
            &typed_input,
            &all_expressions,
        );

        // Should fail because field2 is declared but not present
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        // The error occurs during field resolution, not aggregation
        assert!(error_msg.contains("field2") || error_msg.contains("Field 'field2' not found"));
    }

    #[test]
    fn test_direct_value_resolution_missing_field_throws_error() {
        // Test that direct value resolution also throws errors for missing fields
        let mut transform_fields = HashMap::new();
        transform_fields.insert("field1".to_string(), "input.field1".to_string());
        transform_fields.insert("field2".to_string(), "input.field2".to_string());

        let schema = DeclarativeSchemaDefinition::new(
            "single_schema".to_string(),
            SchemaType::Single,
            None,
            None,
            Some(transform_fields),
        );

        let parsed_chains = vec![
            (
                "field1".to_string(),
                build_parsed_chain("input.field1", &["input", "field1"]),
            )
        ];

        let execution_result = ExecutionResult {
            index_entries: HashMap::new(),
            warnings: HashMap::new(),
        };

        // Only provide field1 in input, missing field2
        let mut typed_input: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();
        let mut field1_map: HashMap<KeyValue, FieldValue> = HashMap::new();
        field1_map.insert(KeyValue::new(None, None), FieldValue { 
            value: json!("value1"), 
            atom_uuid: "test-uuid".to_string() 
        });
        typed_input.insert("input.field1".to_string(), field1_map);

        let all_expressions = vec![
            ("field1".to_string(), "input.field1".to_string()),
            ("field2".to_string(), "input.field2".to_string()),
        ];

        let result = aggregate_results_unified_typed(
            &schema,
            &parsed_chains,
            &execution_result,
            &typed_input,
            &all_expressions,
        );

        // Should fail because field2 is declared but not resolvable
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        // The error occurs during field resolution, not aggregation
        assert!(error_msg.contains("field2") || error_msg.contains("Field 'field2' not found"));
    }

    #[test]
    fn test_every_entry_has_every_field() {
        // Test that in a HashRange schema with word splitting, every entry has every field
        let mut transform_fields = HashMap::new();
        transform_fields.insert("word".to_string(), "input.content.split_by_word()".to_string());
        transform_fields.insert("author".to_string(), "input.author".to_string());
        transform_fields.insert("title".to_string(), "input.title".to_string());

        let schema = DeclarativeSchemaDefinition::new(
            "test_schema".to_string(),
            SchemaType::HashRange { 
                keyconfig: crate::schema::types::key_config::KeyConfig::new(None, None)
            },
            Some(crate::schema::types::key_config::KeyConfig::new(
                Some("word".to_string()),
                Some("author".to_string())
            )),
            None,
            Some(transform_fields),
        );

        let parsed_chains = vec![
            ("word".to_string(), build_parsed_chain("input.content.split_by_word()", &["input", "content", "split_by_word()"])),
            ("author".to_string(), build_parsed_chain("input.author", &["input", "author"])),
            ("title".to_string(), build_parsed_chain("input.title", &["input", "title"])),
        ];

        // Simulate execution results with word splitting and field inheritance
        let mut index_entries = HashMap::new();
        index_entries.insert("input.content.split_by_word()".to_string(), vec![
            build_index_entry_with_row_id("input.content.split_by_word()", json!("hello"), "0/0"),
            build_index_entry_with_row_id("input.content.split_by_word()", json!("world"), "0/1"),
        ]);
        index_entries.insert("input.author".to_string(), vec![
            build_index_entry_with_row_id("input.author", json!("John Doe"), "0"),
        ]);
        index_entries.insert("input.title".to_string(), vec![
            build_index_entry_with_row_id("input.title", json!("Test Post"), "0"),
        ]);

        let execution_result = ExecutionResult {
            index_entries,
            warnings: HashMap::new(),
        };

        let typed_input: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();
        let all_expressions = vec![
            ("word".to_string(), "input.content.split_by_word()".to_string()),
            ("author".to_string(), "input.author".to_string()),
            ("title".to_string(), "input.title".to_string()),
        ];

        let result = aggregate_results_unified_typed(
            &schema,
            &parsed_chains,
            &execution_result,
            &typed_input,
            &all_expressions,
        );

        // Should succeed and every row should have all fields
        assert!(result.is_ok());
        let result_value = result.unwrap();
        let arr = result_value.as_array().expect("rows array");
        
        
        // Should have 2 rows: only child rows "0/0", "0/1" (parent row "0" is filtered out to prevent duplicates)
        assert_eq!(arr.len(), 2);
        
        // Verify every row has all 3 fields
        for row in arr {
            let obj = row.as_object().expect("row obj");
            let fields = obj.get("fields").unwrap().as_object().unwrap();
            
            // Every row must have all declared fields
            assert!(fields.contains_key("word"), "Row missing 'word' field: {:?}", fields);
            assert!(fields.contains_key("author"), "Row missing 'author' field: {:?}", fields);
            assert!(fields.contains_key("title"), "Row missing 'title' field: {:?}", fields);
        }
    }

    #[test]
    fn test_hash_range_aggregation_inheritance_mechanism() {
        // Test that the inheritance mechanism works correctly for word splitting
        let mut transform_fields = HashMap::new();
        transform_fields.insert("word".to_string(), "input.content.split_by_word()".to_string());
        transform_fields.insert("author".to_string(), "input.author".to_string());

        let schema = DeclarativeSchemaDefinition::new(
            "hash_range_schema".to_string(),
            SchemaType::HashRange { 
                keyconfig: crate::schema::types::key_config::KeyConfig::new(None, None)
            },
            None,
            None,
            Some(transform_fields),
        );

        let parsed_chains = vec![
            (
                "word".to_string(),
                build_parsed_chain("input.content.split_by_word()", &["input", "content"]),
            ),
            (
                "author".to_string(),
                build_parsed_chain("input.author", &["input", "author"]),
            ),
        ];

        // Create execution results where both word and author have entries for the same rows
        // This ensures all required fields are present in each row
        let mut index_entries = HashMap::new();
        index_entries.insert("input.content.split_by_word()".to_string(), vec![
            build_index_entry_with_row_id("input.content.split_by_word()", json!("hello"), "0/0"),
            build_index_entry_with_row_id("input.content.split_by_word()", json!("world"), "0/1"),
        ]);
        index_entries.insert("input.author".to_string(), vec![
            build_index_entry_with_row_id("input.author", json!("John Doe"), "0/0"),
            build_index_entry_with_row_id("input.author", json!("John Doe"), "0/1"),
        ]);

        let execution_result = ExecutionResult {
            index_entries,
            warnings: HashMap::new(),
        };

        let typed_input: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();
        let all_expressions = vec![
            ("word".to_string(), "input.content.split_by_word()".to_string()),
            ("author".to_string(), "input.author".to_string()),
        ];

        let result = aggregate_results_unified_typed(
            &schema,
            &parsed_chains,
            &execution_result,
            &typed_input,
            &all_expressions,
        );

        // Should succeed because both word and author fields are present in each row
        assert!(result.is_ok());
        let result_value = result.unwrap();
        let arr = result_value.as_array().expect("rows array");
        assert_eq!(arr.len(), 2); // Two word rows
        
        // Verify all fields are present
        for row in arr {
            let row_obj = row.as_object().unwrap();
            let fields = row_obj.get("fields").unwrap().as_object().unwrap();
            assert!(fields.contains_key("word"));
            assert!(fields.contains_key("author"));
            assert_eq!(fields.get("author").unwrap(), &json!("John Doe"));
        }
    }

    /// Helper function to build an index entry with a specific row_id for testing
    fn build_index_entry_with_row_id(expression: &str, value: JsonValue, row_id: &str) -> IndexEntry {
        IndexEntry {
            row_id: row_id.to_string(),
            value,
            atom_uuid: "test-uuid".to_string(),
            metadata: HashMap::new(),
            expression: expression.to_string(),
        }
    }

    #[test]
    fn test_json_value_to_string_conversion() {
        assert_eq!(convert_json_to_string(&json!("hello")), Some("hello".to_string()));
        assert_eq!(convert_json_to_string(&json!(42)), Some("42".to_string()));
        assert_eq!(convert_json_to_string(&json!(true)), Some("true".to_string()));
        assert_eq!(convert_json_to_string(&json!(null)), None);
        assert_eq!(convert_json_to_string(&json!(["hello"])), Some("hello".to_string()));
    }
}