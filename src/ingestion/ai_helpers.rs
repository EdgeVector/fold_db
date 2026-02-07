//! Shared helper functions for AI service implementations (OpenRouter, Ollama).

use super::prompts::{PROMPT_ACTIONS, PROMPT_HEADER};
use super::{AISchemaResponse, IngestionError, IngestionResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use serde_json::Value;
use std::collections::HashMap;

/// Pretty-print a JSON value.
pub fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "Invalid JSON".to_string())
}

/// Create the prompt for the AI from sample JSON and array context.
pub fn create_prompt(sample_json: &Value, is_array_input: bool) -> String {
    let array_note = if is_array_input {
        "\n\nIMPORTANT: The user provided a JSON ARRAY of multiple objects. You MUST create a Range schema with a range_key to store multiple entities."
    } else {
        ""
    };

    format!(
        "{header}\n\nSample JSON Data:\n{sample}{array_note}\n\n{actions}",
        header = PROMPT_HEADER,
        sample = pretty_json(sample_json),
        array_note = array_note,
        actions = PROMPT_ACTIONS
    )
}

/// Extract JSON from an AI response text that may contain markdown fences or extra text.
pub fn extract_json_from_response(response_text: &str) -> IngestionResult<String> {
    // First try to find a JSON block marker
    let text_to_parse = if let Some(start) = response_text.find("```json") {
        let search_start = start + 7; // Length of "```json"
        if let Some(end_offset) = response_text[search_start..].find("```") {
            let json_end = search_start + end_offset;
            &response_text[search_start..json_end]
        } else {
            &response_text[search_start..]
        }
    } else if let Some(start) = response_text.find('{') {
        &response_text[start..]
    } else {
        response_text
    };

    // Use serde_json stream deserializer to parse the first valid JSON value
    let deserialize_stream =
        serde_json::Deserializer::from_str(text_to_parse).into_iter::<Value>();

    for value in deserialize_stream {
        match value {
            Ok(v) => {
                // Valid JSON found, re-serialize it to ensure it's clean
                return serde_json::to_string(&v).map_err(|e| {
                    IngestionError::ai_response_validation_error(format!(
                        "Failed to serialize extracted JSON: {}",
                        e
                    ))
                });
            }
            Err(_) => continue,
        }
    }

    // Fallback: simpler extraction if stream parsing failed
    if let Some(start) = response_text.find('{') {
        if let Some(end) = response_text.rfind('}') {
            if end > start {
                let json_candidate = response_text[start..=end].to_string();
                if serde_json::from_str::<Value>(&json_candidate).is_ok() {
                    return Ok(json_candidate);
                }
            }
        }
    }

    // If all else fails, return trimmed text and let the caller try to parse it
    Ok(response_text.trim().to_string())
}

/// Validate that a schema has classifications on all primitive fields.
pub fn validate_schema_has_classifications(schema_val: &Value) -> IngestionResult<()> {
    let schema_obj = schema_val.as_object().ok_or_else(|| {
        IngestionError::ai_response_validation_error("Schema must be a JSON object")
    })?;

    let schema_name = schema_obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let field_topologies = schema_obj
        .get("field_topologies")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            IngestionError::ai_response_validation_error(format!(
                "Schema '{}' missing field_topologies",
                schema_name
            ))
        })?;

    // Check each field's topology for classifications
    for (field_name, topology_val) in field_topologies {
        let topology_obj = topology_val
            .as_object()
            .and_then(|obj| obj.get("root"))
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                IngestionError::ai_response_validation_error(format!(
                    "Schema '{}' field '{}' has invalid topology structure",
                    schema_name, field_name
                ))
            })?;

        validate_topology_node_classifications(schema_name, field_name, topology_obj)?;
    }

    Ok(())
}

/// Recursively validate that primitive nodes have classifications.
fn validate_topology_node_classifications(
    schema_name: &str,
    field_name: &str,
    node: &serde_json::Map<String, Value>,
) -> IngestionResult<()> {
    let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match node_type {
        "Primitive" => {
            let classifications = node.get("classifications").and_then(|v| v.as_array());

            match classifications {
                Some(arr) if !arr.is_empty() => Ok(()),
                _ => Err(IngestionError::ai_response_validation_error(format!(
                    "Schema '{}' field '{}' has a Primitive without classifications. \
                        AI must provide at least one classification (e.g., [\"word\"])",
                    schema_name, field_name
                ))),
            }
        }
        "Array" => {
            if let Some(value_obj) = node.get("value").and_then(|v| v.as_object()) {
                validate_topology_node_classifications(schema_name, field_name, value_obj)?;
            }
            Ok(())
        }
        "Object" => {
            if let Some(value_obj) = node.get("value").and_then(|v| v.as_object()) {
                for (_nested_field, nested_node) in value_obj {
                    if let Some(nested_obj) = nested_node.as_object() {
                        validate_topology_node_classifications(
                            schema_name,
                            field_name,
                            nested_obj,
                        )?;
                    }
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Validate and convert a parsed JSON value into an AISchemaResponse.
pub fn validate_and_convert_response(parsed: Value) -> IngestionResult<AISchemaResponse> {
    let obj = parsed.as_object().ok_or_else(|| {
        IngestionError::ai_response_validation_error("Response must be a JSON object")
    })?;

    // Parse new_schemas
    let new_schemas = obj.get("new_schemas").cloned();

    // Validate that new schemas have classifications on all primitive fields
    if let Some(schema_val) = &new_schemas {
        match schema_val {
            Value::Array(schemas) => {
                for schema in schemas {
                    validate_schema_has_classifications(schema)?;
                }
            }
            Value::Object(_) => {
                validate_schema_has_classifications(schema_val)?;
            }
            _ => {}
        }
    }

    // Parse mutation_mappers
    let mutation_mappers = match obj.get("mutation_mappers") {
        Some(Value::Object(map)) => {
            let mut result = HashMap::new();
            for (key, value) in map {
                if let Some(value_str) = value.as_str() {
                    result.insert(key.clone(), value_str.to_string());
                }
            }
            result
        }
        Some(Value::Null) | None => HashMap::new(),
        _ => {
            return Err(IngestionError::ai_response_validation_error(
                "mutation_mappers must be an object with string values",
            ))
        }
    };

    Ok(AISchemaResponse {
        new_schemas,
        mutation_mappers,
    })
}

/// Parse the raw AI response text into an AISchemaResponse.
pub fn parse_ai_response(response_text: &str) -> IngestionResult<AISchemaResponse> {
    let json_str = extract_json_from_response(response_text)?;
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Extracted JSON string: {}",
        json_str
    );

    let parsed: Value = serde_json::from_str(&json_str).map_err(|e| {
        IngestionError::ai_response_validation_error(format!(
            "Failed to parse AI response as JSON: {}. Response: {}",
            e, json_str
        ))
    })?;

    log_feature!(
        LogFeature::Ingestion,
        info,
        "Parsed JSON value: {}",
        pretty_json(&parsed)
    );

    let result = validate_and_convert_response(parsed)?;

    log_feature!(
        LogFeature::Ingestion,
        info,
        "=== FINAL PARSED AI RESPONSE ==="
    );
    log_feature!(
        LogFeature::Ingestion,
        info,
        "New schemas: {}",
        result
            .new_schemas
            .as_ref()
            .map(pretty_json)
            .unwrap_or_else(|| "None".to_string())
    );
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Mutation mappers: {:?}",
        result.mutation_mappers
    );
    log_feature!(
        LogFeature::Ingestion,
        info,
        "=== END PARSED AI RESPONSE ==="
    );

    Ok(result)
}
