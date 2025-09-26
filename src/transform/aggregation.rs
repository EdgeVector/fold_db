//! Result aggregation for transform execution.
//!
//! This module handles the aggregation of execution results from the ExecutionEngine
//! into the final output format for different schema types. It provides a unified
//! interface for processing both direct value resolution and execution result aggregation.

use crate::schema::types::{DeclarativeSchemaDefinition, SchemaError, KeyConfig, KeyValue};
use crate::schema::types::schema::SchemaType;
use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::execution_engine::{ExecutionResult, IndexEntry};
use crate::transform::shared_utilities::resolve_field_value_from_chain;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};

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

        for (_row_id, fields_map) in &self.raw_rows {
            let mut used_names: HashSet<String> = HashSet::new();
            let mut shaped_fields = serde_json::Map::new();

            for (field_name, values) in fields_map {
                // Preserve original declared field name for row shaping
                let unique_name = field_name.clone();
                used_names.insert(unique_name.clone());
                let value = self.format_field_value(values);
                shaped_fields.insert(unique_name, value);
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

    /// Determines the appropriate field key for a given field name.
    ///
    /// # Arguments
    ///
    /// * `field_name` - The original field name
    ///
    /// # Returns
    ///
    /// The sanitized field key
    fn determine_field_key(&self, field_name: &str) -> String {
        self.expressions
            .get(field_name)
            .and_then(|expr| extract_expression_final_segment(expr))
            .unwrap_or_else(|| sanitize_field_name(field_name))
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

    /// Ensures a unique field name by appending numbers if necessary.
    ///
    /// # Arguments
    ///
    /// * `base_name` - The base field name
    /// * `used_names` - Set of already used names
    ///
    /// # Returns
    ///
    /// A unique field name
    fn ensure_unique_name(&self, base_name: &str, used_names: &HashSet<String>) -> String {
        if !used_names.contains(base_name) {
            return base_name.to_string();
        }

        let mut index = 1;
        loop {
            let candidate = format!("{}_{}", base_name, index);
            if !used_names.contains(&candidate) {
                return candidate;
            }
            index += 1;
        }
    }

    /// Derives a key value from the collected field data.
    ///
    /// # Arguments
    ///
    /// * `field_name` - The field name to extract
    ///
    /// # Returns
    ///
    /// The first string value found, if any
    fn derive_key_value(&self, _field_name: &str) -> Option<String> { None }
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
pub fn aggregate_results_unified(
    schema: &DeclarativeSchemaDefinition,
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
) -> Result<JsonValue, SchemaError> {
    let mut accumulator = AggregationAccumulator::new(schema, all_expressions);

    if execution_result.index_entries.values().all(|entries| entries.is_empty()) {
        process_direct_value_resolution(
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

    accumulator.finalize()
}

/// Processes direct value resolution when no execution results are available.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `input_values` - The input values to resolve from
/// * `all_expressions` - All expressions to process
/// * `accumulator` - The accumulator to collect results in
///
/// # Returns
///
/// Result indicating success or failure
fn process_direct_value_resolution(
    parsed_chains: &[(String, ParsedChain)],
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
    accumulator: &mut AggregationAccumulator,
) -> Result<(), SchemaError> {
    let mut row_fields: HashMap<String, Vec<JsonValue>> = HashMap::new();
    for (field_name, expression) in all_expressions {
        let field_value = resolve_field_value(
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
/// * `input_values` - The input values for fallback
/// * `all_expressions` - All expressions to process
/// * `accumulator` - The accumulator to collect results in
///
/// # Returns
///
/// Result indicating success or failure
fn process_execution_result_aggregation(
    schema: &DeclarativeSchemaDefinition,
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
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
            process_single_range_aggregation(
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

    // Ensure every row contains all fields by inheriting from nearest ancestor prefixes
    let mut row_ids: Vec<String> = rows.keys().cloned().collect();
    // Sort by depth (number of segments) ascending so parents come first
    row_ids.sort_by_key(|id| id.split('/').count());

    // Build a quick lookup to avoid multiple clones
    let rows_clone = rows.clone();
    for child_id in row_ids.iter() {
        let child_fields = rows.get_mut(child_id).unwrap();
        let segments: Vec<&str> = child_id.split('/').collect();
        if segments.len() <= 1 { continue; }
        for prefix_len in (1..segments.len()).rev() {
            let prefix = segments[..prefix_len].join("/");
            if let Some(parent_fields) = rows_clone.get(&prefix) {
                for (fname, fvals) in parent_fields {
                    child_fields.entry(fname.clone()).or_insert_with(|| fvals.clone());
                }
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
/// * `input_values` - The input values for fallback
/// * `all_expressions` - All expressions to process
/// * `accumulator` - The accumulator to collect results in
///
/// # Returns
///
/// Result indicating success or failure
fn process_single_range_aggregation(
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
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
                resolve_field_value_from_chain(parsed_chain, input_values, field_name).unwrap_or(JsonValue::Null)
            }
        } else {
            crate::transform::shared_utilities::resolve_dotted_path(expression, input_values).unwrap_or(JsonValue::Null)
        };

        if !field_name.starts_with('_') {
            row_fields.insert(field_name.clone(), vec![field_value]);
        }
    }

    // Use a constant row id since Single/Range produce one row
    accumulator.raw_rows = HashMap::from([(String::from("_single_row"), row_fields)]);

    Ok(())
}

/// Resolves a field value using the most appropriate method.
///
/// # Arguments
///
/// * `field_name` - The name of the field
/// * `expression` - The expression to resolve
/// * `parsed_chains` - Available parsed chains
/// * `input_values` - Input values for fallback
///
/// # Returns
///
/// The resolved field value
fn resolve_field_value(
    field_name: &str,
    expression: &str,
    parsed_chains: &[(String, ParsedChain)],
    input_values: &HashMap<String, JsonValue>,
) -> Result<JsonValue, SchemaError> {
    if let Some((_, parsed_chain)) = parsed_chains.iter().find(|(name, _)| name == field_name) {
        resolve_field_value_from_chain(parsed_chain, input_values, field_name)
    } else {
        crate::transform::shared_utilities::resolve_dotted_path(expression, input_values)
    }
}

/// Extracts the final segment from an expression for use as a field name.
///
/// # Arguments
///
/// * `expression` - The expression to parse
///
/// # Returns
///
/// The final segment if valid, None otherwise
fn extract_expression_final_segment(expression: &str) -> Option<String> {
    expression.split('.').rev().find_map(|segment| {
        let trimmed = segment.trim();
        if trimmed.is_empty() || 
           trimmed.eq_ignore_ascii_case("input") || 
           trimmed.ends_with("()") {
            None
        } else {
            Some(trimmed.trim_matches(|c| "\"'".contains(c)).to_string())
        }
    })
}

/// Sanitizes a field name by removing leading underscores.
///
/// # Arguments
///
/// * `field_name` - The field name to sanitize
///
/// # Returns
///
/// The sanitized field name
fn sanitize_field_name(field_name: &str) -> String {
    let sanitized = field_name.trim_start_matches('_');
    if sanitized.is_empty() {
        field_name.to_string()
    } else {
        sanitized.to_string()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::DeclarativeSchemaDefinition;
    use crate::schema::types::schema::SchemaType;
    use crate::transform::iterator_stack::chain_parser::{ChainOperation, ParsedChain};
    use crate::transform::iterator_stack::execution_engine::{ExecutionResult, IndexEntry};
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

        let parsed_chains = vec![(
            "field1".to_string(),
            build_parsed_chain("input.field1", &["input", "field1"]),
        )];

        let execution_result = ExecutionResult {
            index_entries: HashMap::new(),
            warnings: HashMap::new(),
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

        let parsed_chains = vec![(
            "field1".to_string(),
            build_parsed_chain("input.field1", &["input", "field1"]),
        )];

        let mut index_entries = HashMap::new();
        index_entries.insert("input.field1".to_string(), vec![build_index_entry("input.field1", json!("executed_value1"))]);
        let execution_result = ExecutionResult {
            index_entries,
            warnings: HashMap::new(),
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

        let parsed_chains = vec![(
            "field1".to_string(),
            build_parsed_chain("input.field1", &["input", "field1"]),
        )];

        let mut index_entries = HashMap::new();
        index_entries.insert("input.field1".to_string(), vec![
            build_index_entry("input.field1", json!("value1")),
            build_index_entry("input.field1", json!("value2")),
        ]);
        let execution_result = ExecutionResult {
            index_entries,
            warnings: HashMap::new(),
        };

        let input_values = HashMap::new();
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
        let arr = result_value.as_array().expect("rows array");
        assert_eq!(arr.len(), 1);
        let row = arr[0].as_object().unwrap();
        assert!(row.contains_key("key"));
        assert_eq!(row["fields"]["field1"], json!(["value1", "value2"]));
    }

    #[test]
    fn test_field_name_sanitization() {
        assert_eq!(sanitize_field_name("normal_field"), "normal_field");
        assert_eq!(sanitize_field_name("_internal_field"), "internal_field");
        assert_eq!(sanitize_field_name("__double_underscore"), "double_underscore");
        assert_eq!(sanitize_field_name("_"), "_");
    }

    #[test]
    fn test_expression_final_segment_extraction() {
        assert_eq!(extract_expression_final_segment("input.field1"), Some("field1".to_string()));
        assert_eq!(extract_expression_final_segment("input.user.name"), Some("name".to_string()));
        assert_eq!(extract_expression_final_segment("input"), None);
        assert_eq!(extract_expression_final_segment("input."), None);
        assert_eq!(extract_expression_final_segment("input.func()"), None);
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