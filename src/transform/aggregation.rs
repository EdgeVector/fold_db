//! Result aggregation for transform execution.
//!
//! This module handles the aggregation of execution results from the ExecutionEngine
//! into the final output format for different schema types. It provides a unified
//! interface for processing both direct value resolution and execution result aggregation.

use crate::schema::types::{DeclarativeSchemaDefinition, SchemaError};
use crate::schema::types::schema::SchemaType;
use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::execution_engine::{ExecutionResult, IndexEntry};
use crate::transform::shared_utilities::resolve_field_value_from_chain;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};

/// Constants for field naming and structure
const HASH_FIELD_PREFIX: &str = "_hash_field";
const RANGE_FIELD_PREFIX: &str = "_range_field";
const HASH_KEY: &str = "hash";
const RANGE_KEY: &str = "range";
const FIELDS_KEY: &str = "fields";

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
fn extract_optimal_field_value(entry: &IndexEntry) -> JsonValue {
    serde_json::to_value(&entry.hash_value).unwrap_or(JsonValue::Null)
}

/// Aggregation accumulator that builds the final result structure.
///
/// This struct encapsulates the logic for collecting field values and
/// constructing the final hash->range->fields structure.
struct AggregationAccumulator<'a> {
    #[allow(dead_code)]
    schema: &'a DeclarativeSchemaDefinition,
    expressions: HashMap<String, String>,
    raw_fields: HashMap<String, Vec<JsonValue>>,
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

        Self {
            schema,
            expressions: expression_map,
            raw_fields: HashMap::new(),
        }
    }

    /// Finalizes the aggregation and returns the structured result.
    ///
    /// # Returns
    ///
    /// The final aggregated result in hash->range->fields format
    fn finalize(self) -> Result<JsonValue, SchemaError> {
        let mut shape_payload = serde_json::Map::new();
        let mut used_names: HashSet<String> = HashSet::new();

        // Process all collected field values
        for (field_name, values) in &self.raw_fields {
            let field_key = self.determine_field_key(field_name);
            let unique_name = self.ensure_unique_name(&field_key, &used_names);
            used_names.insert(unique_name.clone());

            let value = self.format_field_value(values);
            shape_payload.insert(unique_name, value);
        }

        // Create the structured result
        let hash_value = self.derive_key_value(HASH_FIELD_PREFIX);
        let range_value = self.derive_key_value(RANGE_FIELD_PREFIX);
        let shaped_input = JsonValue::Object(shape_payload);
        
        let mut result = serde_json::Map::new();
        result.insert(HASH_KEY.to_string(), JsonValue::String(hash_value.unwrap_or_default()));
        result.insert(RANGE_KEY.to_string(), JsonValue::String(range_value.unwrap_or_default()));
        result.insert(FIELDS_KEY.to_string(), shaped_input);

        Ok(JsonValue::Object(result))
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
    fn format_field_value(&self, values: &[JsonValue]) -> JsonValue {
        if values.len() == 1 {
            values[0].clone()
        } else {
            JsonValue::Array(values.to_vec())
        }
    }

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
    fn derive_key_value(&self, field_name: &str) -> Option<String> {
        self.raw_fields
            .get(field_name)
            .and_then(|values| values.iter().find_map(convert_json_to_string))
    }
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
    for (field_name, expression) in all_expressions {
        let field_value = resolve_field_value(
            field_name,
            expression,
            parsed_chains,
            input_values,
        )?;

        accumulator.raw_fields
            .insert(field_name.clone(), vec![field_value]);
    }

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
    let mut field_arrays: HashMap<String, Vec<JsonValue>> = HashMap::new();
    
    // Initialize field arrays
    for (field_name, _) in parsed_chains.iter() {
        field_arrays.insert(field_name.clone(), Vec::new());
    }

    // Group entries by expression
    let mut entries_by_expression: HashMap<String, Vec<&IndexEntry>> = HashMap::new();
    for entries in execution_result.index_entries.values() {
        for entry in entries {
            entries_by_expression
                .entry(entry.expression.clone())
                .or_default()
                .push(entry);
        }
    }

    // Collect values for each field
    for (field_name, parsed_chain) in parsed_chains.iter() {
        if let Some(entries) = entries_by_expression.get(&parsed_chain.expression) {
            if let Some(values) = field_arrays.get_mut(field_name) {
                for entry in entries {
                    values.push(extract_optimal_field_value(entry));
                }
            }
        }
    }

    // Transfer collected values to accumulator
    for (field_name, values) in field_arrays {
        accumulator.raw_fields.insert(field_name, values);
    }

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
    // Create a map of expression to entry for quick lookup
    let mut entries_by_expression: HashMap<String, &IndexEntry> = HashMap::new();
    for entries in execution_result.index_entries.values() {
        for entry in entries {
            entries_by_expression.insert(entry.expression.clone(), entry);
        }
    }

    // Process each expression
    for (field_name, expression) in all_expressions {
        let field_value = if let Some((_, parsed_chain)) =
            parsed_chains.iter().find(|(name, _)| name == field_name)
        {
            if let Some(entry) = entries_by_expression.get(&parsed_chain.expression) {
                extract_optimal_field_value(entry)
            } else {
                resolve_field_value_from_chain(parsed_chain, input_values, field_name)
                    .unwrap_or(JsonValue::Null)
            }
        } else {
            crate::transform::shared_utilities::resolve_dotted_path(expression, input_values)
                .unwrap_or(JsonValue::Null)
        };

        // Only include non-internal fields
        if !field_name.starts_with('_') {
            accumulator.raw_fields
                .insert(field_name.clone(), vec![field_value]);
        }
    }

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
            expression: expression.to_string(),
            hash_value: value,
            range_value: JsonValue::Null,
            atom_uuid: "test-uuid".to_string(),
            metadata: HashMap::new(),
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
        assert_eq!(result_value[HASH_KEY], json!(""));
        assert_eq!(result_value[RANGE_KEY], json!(""));
        assert_eq!(result_value[FIELDS_KEY]["field1"], json!("value1"));
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
        assert_eq!(result_value[HASH_KEY], json!(""));
        assert_eq!(result_value[RANGE_KEY], json!(""));
        assert_eq!(result_value[FIELDS_KEY]["field1"], json!("executed_value1"));
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
        assert_eq!(result_value[FIELDS_KEY]["field1"], json!(["value1", "value2"]));
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