//! Shared helper functions for AI service implementations (OpenRouter, Ollama).

use super::{IngestionError, IngestionResult, StructureAnalyzer};
use crate::log_feature;
use crate::logging::features::LogFeature;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

// ---- AI response types ----

/// Parsed AI response for schema analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct AISchemaResponse {
    /// New schema definition created from the data structure
    pub new_schemas: Option<Value>,
    /// Mapping from JSON field paths to schema field paths
    pub mutation_mappers: HashMap<String, String>,
}

// ---- Prompt constants ----

/// Prompt header describing the response format, schema structure, and classification rules.
pub const PROMPT_HEADER: &str = r#"Create a schema for this sample json data. Return the value in this format:
{
  "new_schemas": <single_schema_definition>,
  "mutation_mappers": {json_field_name: schema_field_name}
}

Where:
- new_schemas is a single schema definition for the input data
- mutation_mappers maps ONLY TOP-LEVEL JSON field names to schema field names (e.g., {"id": "id", "user": "user"})

CRITICAL - Mutation Mappers:
- ONLY use top-level field names in mutation_mappers (e.g., "user", "comments", "id")
- DO NOT use nested paths (e.g., "user.name", "comments[*].content") - they will not work
- Nested objects and arrays will be stored as-is in their top-level field
- Example: if JSON has {"user": {"id": 1, "name": "Tom"}}, mapper should be {"user": "user"}, NOT {"user.id": "id"}

IMPORTANT - Schema Types:
- STRONGLY PREFER Range schemas over Single schemas. Most data benefits from a range key.
- For storing MULTIPLE entities/records, use "key": {"range_field": "field_name"}
- Only use Single (no "key" field) when the data is truly a single global config/settings object with no records
- If the user is providing an ARRAY of objects, you MUST use a Range schema with a "key"
- Even for single objects, if the data has a date/timestamp field, use a Range schema so future records can be added
- PREFER a date or timestamp field as the range_field (like "created_at", "date", "timestamp", "posted_at") so that data can be queried by time range
- If NO date/timestamp field exists, fall back to a unique identifier field (like "id", "name", "email")

IMPORTANT - Schema Name and Descriptive Name:
- You MUST include "name": use any simple name like "Schema" (it will be replaced automatically)
- ALWAYS include "descriptive_name": a clear, human-readable description of what this schema stores
- Example: "descriptive_name": "User Profile Information" or "Customer Order Records"

IMPORTANT - Field Classifications:
- EVERY field MUST have a "field_classifications" entry
- Analyze field semantic meaning and assign appropriate classification types
- Multiple classifications per field are encouraged (e.g., ["name:person", "word"])
- ALWAYS include "word" classification for any string field that contains searchable text
- Available classification types:
  * "word" - general text, split into words for search (MANDATORY for searchable text)
  * "name:person" - person names (kept whole: "Jennifer Liu")
  * "name:company" - company/organization names
  * "name:place" - location names (cities, countries, places)
  * "email" - email addresses
  * "phone" - phone numbers
  * "url" - URLs or domains
  * "date" - dates and timestamps
  * "hashtag" - hashtags (from social media)
  * "username" - usernames/handles
  * "number" - numeric values (amounts, counts, scores, percentages, quantities)
- "field_classifications" is a flat map from field name to list of classification strings

Example Range schema with date range_field (PREFERRED when data has timestamps):
{
  "name": "Schema",
  "descriptive_name": "Social Media Posts",
  "key": {"range_field": "created_at"},
  "fields": ["created_at", "author", "content"],
  "field_classifications": {
    "created_at": ["date"],
    "author": ["name:person", "word"],
    "content": ["word"]
  }
}

Example Range schema with ID range_field (only when NO date/timestamp field exists):
{
  "name": "Schema",
  "descriptive_name": "User Profile Information",
  "key": {"range_field": "id"},
  "fields": ["id", "name", "age"],
  "field_classifications": {
    "id": ["word"],
    "name": ["name:person", "word"],
    "age": ["number"]
  }
}

Example Single schema (for one global value):
{
  "name": "Schema",
  "descriptive_name": "Global Counter Statistics",
  "fields": ["count", "total"],
  "field_classifications": {
    "count": ["number"],
    "total": ["number"]
  }
}

Example with Arrays and Objects (note: date field used as range_field):
{
  "name": "Schema",
  "descriptive_name": "Social Media Post",
  "key": {"range_field": "posted_at"},
  "fields": ["posted_at", "content", "hashtags", "media"],
  "field_classifications": {
    "posted_at": ["date"],
    "content": ["word"],
    "hashtags": ["hashtag", "word"],
    "media": ["url", "word"]
  }
}

IMPORTANT - Transform Fields (DSL):
- You can add a "transform_fields" map to the schema to derive new fields from existing ones.
- SYNTAX: "SourceField.function().function()"
- IMPLICIT CARDINALITY:
  * The system automatically iterates over every record in the schema (1:N). You do NOT need a .map() token.
  * Iterator Functions (like split_by_word, split_array) INCREASE depth/cardinality (one row -> many rows).
  * Reducer Functions (like count, join, sum) DECREASE depth/cardinality (many rows -> one row).
- DEPRECATION: The ".map()" token is DEPRECATED. Do not use it.
- Examples:
  * Word Count: "content.split_by_word().count()" (Iterates content -> splits into words -> counts words per row)
  * Character Count: "content.slugify().len()"
  * Array Join: "hashtags.join(', ')" (Joins array elements into a string)
"#;

/// Instructions appended to every prompt.
pub const PROMPT_ACTIONS: &str = r#"Please analyze the sample data and create a new schema definition in new_schemas with mutation_mappers.

CRITICAL RULES:
- If the original input was a JSON array (multiple objects), you MUST create a Range schema with "key": {"range_field": "field_name"}
- PREFER a date/timestamp field as range_field (e.g., "created_at", "date", "timestamp") — this enables time-based queries. Only use an ID field if no date/timestamp exists.
- NEVER create a Single-type schema for array inputs - they will overwrite data
- AVOID Single schemas unless the data is truly a one-off global config. If any field looks like a date, timestamp, or unique ID, use Range instead.
- ALWAYS provide field_classifications for every field

The response must be valid JSON."#;

/// Call an async function with retries and exponential backoff.
///
/// Logs each attempt and backs off exponentially (1s, 2s, 4s, ...) on failure.
/// `label` is used in log messages (e.g. "OpenRouter API" or "Ollama API").
/// `fallback_error` is called if all attempts fail without producing an error.
pub async fn call_with_retries<F, Fut>(
    label: &str,
    max_retries: u32,
    fallback_error: fn() -> IngestionError,
    mut call_fn: F,
) -> IngestionResult<String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = IngestionResult<String>>,
{
    let mut last_error = None;

    for attempt in 1..=max_retries {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "{} attempt {} of {}",
            label,
            attempt,
            max_retries
        );

        let start_time = std::time::Instant::now();
        match call_fn().await {
            Ok(response) => {
                let elapsed = start_time.elapsed();
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "{} call successful on attempt {} (took {:.2?})",
                    label,
                    attempt,
                    elapsed
                );
                return Ok(response);
            }
            Err(e) => {
                let elapsed = start_time.elapsed();
                log_feature!(
                    LogFeature::Ingestion,
                    warn,
                    "{} attempt {} failed (took {:.2?}): {}",
                    label,
                    attempt,
                    elapsed,
                    e
                );
                last_error = Some(e);

                if attempt < max_retries {
                    let delay = Duration::from_secs(2_u64.pow(attempt - 1));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(fallback_error))
}

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

/// Analyze JSON data and build the AI prompt for schema recommendation.
///
/// Shared between OpenRouter and Ollama services.  Returns the prompt string
/// ready to be sent to the AI backend.
pub fn analyze_and_build_prompt(sample_json: &Value) -> IngestionResult<String> {
    let superset_structure = StructureAnalyzer::extract_structure_skeleton(sample_json);
    let stats = StructureAnalyzer::get_analysis_stats(sample_json);

    log_feature!(
        LogFeature::Ingestion,
        info,
        "Analyzed JSON structure: {} elements, {} unique fields",
        stats.total_elements,
        stats.unique_fields
    );

    if let Some(array) = sample_json.as_array() {
        if array.is_empty() {
            return Err(IngestionError::ai_response_validation_error(
                "Cannot determine schema from empty JSON array".to_string(),
            ));
        }

        log_feature!(
            LogFeature::Ingestion,
            info,
            "JSON data is an array with {} elements, created superset structure with {} fields",
            array.len(),
            stats.unique_fields
        );

        let common_fields = stats.get_common_fields();
        let partial_fields = stats.get_partial_fields();

        if !common_fields.is_empty() {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Common fields (100% coverage): {:?}",
                common_fields
            );
        }

        if !partial_fields.is_empty() {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Partial fields (not in all elements): {:?}",
                partial_fields
            );
        }
    }

    log_feature!(
        LogFeature::Ingestion,
        info,
        "Superset structure: {}",
        pretty_json(&superset_structure)
    );

    let is_array_input = sample_json.is_array();
    let prompt = create_prompt(&superset_structure, is_array_input);

    log_feature!(
        LogFeature::Ingestion,
        info,
        "AI Request Prompt (length: {} chars): {}",
        prompt.len(),
        if prompt.len() > 1000 {
            format!("{}...[truncated]", &prompt[..1000])
        } else {
            prompt.clone()
        }
    );

    Ok(prompt)
}

/// Extract JSON from an AI response text that may contain markdown fences or extra text.
///
/// Handles both JSON objects (`{...}`) and arrays (`[...]`), with support for
/// markdown code blocks and surrounding prose.
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
    } else {
        // Find the first '{' or '[' — whichever comes first
        let obj_start = response_text.find('{');
        let arr_start = response_text.find('[');
        match (obj_start, arr_start) {
            (Some(o), Some(a)) => &response_text[o.min(a)..],
            (Some(o), None) => &response_text[o..],
            (None, Some(a)) => &response_text[a..],
            (None, None) => response_text,
        }
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

    // Fallback: try brace-matching for objects
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

    // Fallback: try bracket-matching for arrays
    if let Some(start) = response_text.find('[') {
        if let Some(end) = response_text.rfind(']') {
            if end > start {
                let json_candidate = response_text[start..=end].to_string();
                if serde_json::from_str::<Value>(&json_candidate).is_ok() {
                    return Ok(json_candidate);
                }
            }
        }
    }

    // All extraction strategies failed — return an error instead of passing garbage
    let preview = if response_text.len() > 200 {
        format!("{}...", &response_text[..200])
    } else {
        response_text.to_string()
    };
    Err(IngestionError::ai_response_validation_error(format!(
        "Could not extract valid JSON from AI response: {}",
        preview
    )))
}

/// Validate that a schema has classifications for its fields.
pub fn validate_schema_has_classifications(schema_val: &Value) -> IngestionResult<()> {
    let schema_obj = schema_val.as_object().ok_or_else(|| {
        IngestionError::ai_response_validation_error("Schema must be a JSON object")
    })?;

    let schema_name = schema_obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // field_classifications is optional — AI may not always provide them and defaults will be applied later
    let field_classifications = match schema_obj.get("field_classifications").and_then(|v| v.as_object()) {
        Some(fc) => fc,
        None => return Ok(()),
    };

    // Check each field's classifications are non-empty arrays
    for (field_name, classifications_val) in field_classifications {
        match classifications_val.as_array() {
            Some(arr) if !arr.is_empty() => {}
            _ => {
                return Err(IngestionError::ai_response_validation_error(format!(
                    "Schema '{}' field '{}' has empty or invalid classifications. \
                        AI must provide at least one classification (e.g., [\"word\"])",
                    schema_name, field_name
                )));
            }
        }
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_response() {
        // Test with JSON block markers
        let response_with_markers = r###"Here's the analysis:
```json
{"new_schemas": {"name": "test"}, "mutation_mappers": {}}
```
That should work."###;

        let result = extract_json_from_response(response_with_markers);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("new_schemas"));

        // Test with direct JSON
        let response_direct = r###"{"new_schemas": null, "mutation_mappers": {}}"###;
        let result = extract_json_from_response(response_direct);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_json_with_trailing_brace() {
        let response_trailing = r###"
        {
            "new_schemas": null,
            "mutation_mappers": {}
        }
        some extra text with a } closing brace
        "###;

        let result = extract_json_from_response(response_trailing);
        assert!(result.is_ok());
        let json = result.unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("new_schemas").is_some());
    }

    #[test]
    fn test_validate_and_convert_response() {
        let test_json = serde_json::json!({
            "new_schemas": null,
            "mutation_mappers": {
                "field1": "schema.field1",
                "nested.field": "schema.nested_field"
            }
        });

        let result = validate_and_convert_response(test_json);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.mutation_mappers.len(), 2);
    }

    #[test]
    fn test_create_prompt_includes_sample() {
        let sample = serde_json::json!({"a": 1});

        let prompt = create_prompt(&sample, false);
        assert!(prompt.contains("Sample JSON Data:"));
        assert!(prompt.contains("\"a\": 1"));
        assert!(!prompt.contains("Available Schemas:"));
        assert!(prompt.contains(PROMPT_HEADER));
        assert!(prompt.contains(PROMPT_ACTIONS));
    }

    #[test]
    fn test_pretty_json_helpers() {
        let value = serde_json::json!({"x": 1});
        assert!(pretty_json(&value).contains("\"x\": 1"));
    }

    // ---- extract_json_from_response edge cases ----

    #[test]
    fn test_extract_json_array_in_markdown_fence() {
        let response = r#"Here are the results:
```json
[{"path": "a.txt", "should_ingest": true}]
```
Done."#;
        let result = extract_json_from_response(response).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_extract_nested_json_with_trailing_prose() {
        let response = r#"{"outer": {"inner": {"value": 42}}} And here is some extra explanation."#;
        let result = extract_json_from_response(response).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["outer"]["inner"]["value"], 42);
    }

    #[test]
    fn test_extract_first_json_object_from_multiple() {
        let response = r#"{"first": 1} {"second": 2}"#;
        let result = extract_json_from_response(response).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["first"], 1);
    }

    #[test]
    fn test_extract_only_prose_returns_error() {
        let response = "This response contains no JSON at all, just plain text.";
        let result = extract_json_from_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_empty_string_returns_error() {
        let result = extract_json_from_response("");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_json_with_unicode() {
        let response = r#"{"name": "日本語テスト", "emoji": "🎉"}"#;
        let result = extract_json_from_response(response).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "日本語テスト");
        assert_eq!(parsed["emoji"], "🎉");
    }

    #[test]
    fn test_extract_unclosed_markdown_fence() {
        let response = "```json\n{\"key\": \"value\"}\nsome trailing text without closing fence";
        let result = extract_json_from_response(response).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["key"], "value");
    }

    #[test]
    fn test_extract_brace_matching_fallback() {
        // JSON with invalid trailing content that stream parser may choke on,
        // but brace-matching should rescue
        let response = "prefix {\"a\": 1, \"b\": [2, 3]} suffix with } brace";
        let result = extract_json_from_response(response).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["a"], 1);
    }
}
