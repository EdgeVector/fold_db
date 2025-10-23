//! Ollama API service for AI-powered schema analysis

use crate::ingestion::config::OllamaConfig;
use crate::ingestion::{AISchemaResponse, IngestionError, IngestionResult, StructureAnalyzer};
use crate::log_feature;
use crate::logging::features::LogFeature;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Prompt header shared across requests
const PROMPT_HEADER: &str = r#"Tell me which of these schemas to use for this sample json data. If none are available, then create a new one. Return the value in this format:
{
  "existing_schemas": [<list_of_schema_names visualized>],
  "new_schemas": <single_schema_definition>,
  "mutation_mappers": {json_field_name: schema_field_name}
}

Where:
- existing_schemas is an array of schema names that match the input data
- new_schemas is a single schema definition if no existing schemas match
- mutation_mappers maps ONLY TOP-LEVEL JSON field names to schema field names (e.g., {"id": "id", "user": "user"})

CRITICAL - Mutation Mappers:
- ONLY use top-level field names in mutation_mappers (e.g., "user", "comments", "id")
- DO NOT use nested paths (e.g., "user.name", "comments[*].content") - they will not work
- Nested objects and arrays will be stored as-is in their top-level field
- Example: if JSON has {"user": {"id": 1, "name": "Tom"}}, mapper should be {"user": "user"}, NOT {"user.id": "id"}

IMPORTANT - Schema Types:
- For storing MULTIPLE entities/records, use "key": {"range_field": "field_name"}
- For storing ONE global value per field, omit the "key" field
- If the user is providing an ARRAY of objects, you MUST use a Range schema with a "key"
- The range_field should be a unique identifier field (like "name", "id", "email")

IMPORTANT - Schema Name and Descriptive Name:
- You MUST include "name": use any simple name like "Schema" (it will be replaced automatically)
- ALWAYS include "descriptive_name": a clear, human-readable description of what this schema stores
- Example: "descriptive_name": "User Profile Information" or "Customer Order Records"

IMPORTANT - Field Topologies with Classifications:
- EVERY Primitive leaf MUST include "classifications" array
- Analyze field semantic meaning and assign appropriate classification types
- Multiple classifications per field are encouraged (e.g., ["name:person", "word"])
- Available classification types:
  * "word" - general text, split into words for search
  * "name:person" - person names (kept whole: "Jennifer Liu")
  * "name:company" - company/organization names
  * "name:place" - location names (cities, countries, places)
  * "email" - email addresses
  * "phone" - phone numbers
  * "url" - URLs or domains
  * "date" - dates and timestamps
  * "hashtag" - hashtags (from social media)
  * "username" - usernames/handles
- Topology structure:
  * Primitives: {"type": "Primitive", "value": "String", "classifications": ["name:person", "word"]}
  * Objects: {"type": "Object", "value": {"field_name": {"type": "Primitive", "value": "String", "classifications": ["word"]}}}
  * Arrays of Primitives: {"type": "Array", "value": {"type": "Primitive", "value": "String", "classifications": ["hashtag", "word"]}}
  * Arrays of Objects: {"type": "Array", "value": {"type": "Object", "value": {"field_name": {"type": "Primitive", "value": "String", "classifications": ["word"]}}}}

Example Range schema (for multiple records):
{
  "name": "Schema",
  "descriptive_name": "User Profile Information",
  "key": {"range_field": "id"},
  "fields": ["id", "name", "age"],
  "field_topologies": {
    "id": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
    "name": {"root": {"type": "Primitive", "value": "String", "classifications": ["name:person", "word"]}},
    "age": {"root": {"type": "Primitive", "value": "Number", "classifications": ["word"]}}
  }
}

Example Single schema (for one global value):
{
  "name": "Schema",
  "descriptive_name": "Global Counter Statistics",
  "fields": ["count", "total"],
  "field_topologies": {
    "count": {"root": {"type": "Primitive", "value": "Number", "classifications": ["word"]}},
    "total": {"root": {"type": "Primitive", "value": "Number", "classifications": ["word"]}}
  }
}

Example with Arrays and Objects:
{
  "name": "Schema",
  "descriptive_name": "Social Media Post",
  "key": {"range_field": "post_id"},
  "fields": ["post_id", "content", "hashtags", "media"],
  "field_topologies": {
    "post_id": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
    "content": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
    "hashtags": {"root": {"type": "Array", "value": {"type": "Primitive", "value": "String", "classifications": ["hashtag", "word"]}}},
    "media": {"root": {"type": "Array", "value": {"type": "Object", "value": {"url": {"type": "Primitive", "value": "String", "classifications": ["url", "word"]}, "type": {"type": "Primitive", "value": "String", "classifications": ["word"]}}}}}
  }
}
"#;

/// Instructions appended to every prompt
const PROMPT_ACTIONS: &str = r#"Please analyze the sample data and either:
1. If existing schemas can handle this data, return their names in existing_schemas and provide mutation_mappers
2. If no existing schemas match, create a new schema definition in new_schemas and provide mutation_mappers

CRITICAL RULES:
- If the original input was a JSON array (multiple objects), you MUST create a NEW Range schema with "key": {"range_field": "unique_field"}
- NEVER recommend a Single-type schema for array inputs - they will overwrite data
- When user provides an array, ignore any existing Single schemas and create a new Range schema with the "key" field

The response must be valid JSON."#;

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "Invalid JSON".to_string())
}

fn pretty_json_or_empty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
}

/// Ollama API service
pub struct OllamaService {
    client: Client,
    config: OllamaConfig,
    max_retries: u32,
}

/// Request to Ollama API
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

/// Response from Ollama API
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

impl OllamaService {
    /// Create a new Ollama service
    pub fn new(
        config: OllamaConfig,
        timeout_seconds: u64,
        max_retries: u32,
    ) -> IngestionResult<Self> {
        config.validate()?;

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| {
                IngestionError::ollama_error(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            config,
            max_retries,
        })
    }

    /// Get schema recommendation from AI
    pub async fn get_schema_recommendation(
        &self,
        sample_json: &Value,
        available_schemas: &Value,
    ) -> IngestionResult<AISchemaResponse> {
        // Create superset structure from all top-level elements
        let superset_structure = StructureAnalyzer::create_superset_structure(sample_json);
        
        // Get analysis statistics for logging
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
                    "Cannot determine schema from empty JSON array".to_string()
                ));
            }
            
            log_feature!(
                LogFeature::Ingestion,
                info,
                "JSON data is an array with {} elements, created superset structure with {} fields",
                array.len(),
                stats.unique_fields
            );
            
            // Log field coverage information
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
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Available schemas: {}",
            pretty_json_or_empty(available_schemas)
        );

        let is_array_input = sample_json.is_array();
        let prompt = self.create_prompt(&superset_structure, available_schemas, is_array_input);

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Sending request to Ollama API with model: {}",
            self.config.model
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "AI Request Prompt (length: {} chars): {}",
            prompt.len(),
            if prompt.len() > 1000 {
                format!(
                    "{}
...[truncated]",
                    &prompt[..1000]
                )
            } else {
                prompt.clone()
            }
        );

        let response = self.call_ollama_api(&prompt).await?;
        log_feature!(LogFeature::Ingestion, info, "=== FULL AI RESPONSE ===");
        log_feature!(
            LogFeature::Ingestion,
            info,
            "AI Response (length: {} chars):\n{}",
            response.len(),
            response
        );
        log_feature!(LogFeature::Ingestion, info, "=== END AI RESPONSE ===");

        self.parse_ai_response(&response)
    }

    /// Create the prompt for the AI
    fn create_prompt(&self, sample_json: &Value, available_schemas: &Value, is_array_input: bool) -> String {
        let array_note = if is_array_input {
            "\n\nIMPORTANT: The user provided a JSON ARRAY of multiple objects. You MUST create or use a Range schema with a range_key to store multiple entities."
        } else {
            ""
        };
        
        format!(
            "{header}\n\nSample JSON Data:\n{sample}\n\nAvailable Schemas:\n{schemas}{array_note}\n\n{actions}",
            header = PROMPT_HEADER,
            sample = pretty_json(sample_json),
            schemas = pretty_json_or_empty(available_schemas),
            array_note = array_note,
            actions = PROMPT_ACTIONS
        )
    }

    /// Call the Ollama API
    pub async fn call_ollama_api(&self, prompt: &str) -> IngestionResult<String> {
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let mut last_error = None;

        for attempt in 1..=self.max_retries {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Ollama API attempt {} of {}",
                attempt,
                self.max_retries
            );

            match self.make_api_request(&request).await {
                Ok(response) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        info,
                        "Ollama API call successful on attempt {}",
                        attempt
                    );
                    return Ok(response);
                }
                Err(e) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        warn,
                        "Ollama API attempt {} failed: {}",
                        attempt,
                        e
                    );
                    last_error = Some(e);

                    if attempt < self.max_retries {
                        // Exponential backoff
                        let delay = Duration::from_secs(2_u64.pow(attempt - 1));
                        log_feature!(LogFeature::Ingestion, info, "Retrying in {:?}", delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| IngestionError::ollama_error("All API attempts failed")))
    }

    /// Make a single API request
    async fn make_api_request(&self, request: &OllamaRequest) -> IngestionResult<String> {
        let url = format!("{}/api/generate", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IngestionError::ollama_error(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }

        let ollama_response: OllamaResponse = response.json().await?;

        Ok(ollama_response.response)
    }

    /// Parse the AI response
    fn parse_ai_response(&self, response_text: &str) -> IngestionResult<AISchemaResponse> {
        log_feature!(LogFeature::Ingestion, info, "=== PARSING AI RESPONSE ===");
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Raw AI response text: {}",
            response_text
        );

        // Try to extract JSON from the response
        let json_str = self.extract_json_from_response(response_text)?;
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Extracted JSON string: {}",
            json_str
        );

        // Parse the JSON
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

        // Validate and convert to AISchemaResponse
        let result = self.validate_and_convert_response(parsed)?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "=== FINAL PARSED AI RESPONSE ==="
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Existing schemas: {:?}",
            result.existing_schemas
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

    /// Extract JSON from the AI response text
    fn extract_json_from_response(&self, response_text: &str) -> IngestionResult<String> {
        // Look for JSON block markers
        if let Some(start) = response_text.find("```json") {
            let search_start = start + 7; // Length of "```json"
            if let Some(end_offset) = response_text[search_start..].find("```") {
                let json_end = search_start + end_offset;
                return Ok(response_text[search_start..json_end].trim().to_string());
            }
        }

        // Look for direct JSON (starts with { and ends with })
        if let Some(start) = response_text.find('{') {
            if let Some(end) = response_text.rfind('}') {
                if end > start {
                    return Ok(response_text[start..=end].to_string());
                }
            }
        }

        // If no JSON found, try the entire response
        Ok(response_text.trim().to_string())
    }

    /// Validate and convert the parsed response
    fn validate_and_convert_response(&self, parsed: Value) -> IngestionResult<AISchemaResponse> {
        let obj = parsed.as_object().ok_or_else(|| {
            IngestionError::ai_response_validation_error("Response must be a JSON object")
        })?;

        // Parse existing_schemas
        let existing_schemas = match obj.get("existing_schemas") {
            Some(Value::Array(arr)) => arr
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            Some(Value::String(s)) => vec![s.clone()],
            Some(Value::Null) | None => vec![],
            _ => {
                return Err(IngestionError::ai_response_validation_error(
                    "existing_schemas must be an array of strings or null",
                ))
            }
        };

        // Parse new_schemas
        let new_schemas = obj.get("new_schemas").cloned();

        // Parse mutation_mappers
        let mutation_mappers = match obj.get("mutation_mappers") {
            Some(Value::Object(map)) => {
                let mut result = std::collections::HashMap::new();
                for (key, value) in map {
                    if let Some(value_str) = value.as_str() {
                        result.insert(key.clone(), value_str.to_string());
                    }
                }
                result
            }
            Some(Value::Null) | None => std::collections::HashMap::new(),
            _ => {
                return Err(IngestionError::ai_response_validation_error(
                    "mutation_mappers must be an object with string values",
                ))
            }
        };

        Ok(AISchemaResponse {
            existing_schemas,
            new_schemas,
            mutation_mappers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion::config::OllamaConfig;

    #[test]
    fn test_extract_json_from_response() {
        let service = create_test_service();

        // Test with JSON block markers
        let response_with_markers = r###"Here's the analysis:
```json
{"existing_schemas": ["test"], "new_schemas": null, "mutation_mappers": {}}
```
That should work."###;

        let result = service.extract_json_from_response(response_with_markers);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("existing_schemas"));

        // Test with direct JSON
        let response_direct =
            r###"{"existing_schemas": ["test"], "new_schemas": null, "mutation_mappers": {}}"###;
        let result = service.extract_json_from_response(response_direct);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_and_convert_response() {
        let service = create_test_service();

        let test_json = serde_json::json!({
            "existing_schemas": ["schema1", "schema2"],
            "new_schemas": null,
            "mutation_mappers": {
                "field1": "schema.field1",
                "nested.field": "schema.nested_field"
            }
        });

        let result = service.validate_and_convert_response(test_json);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.existing_schemas.len(), 2);
        assert_eq!(response.mutation_mappers.len(), 2);
    }

    #[test]
    fn test_create_prompt_includes_sample_and_schemas() {
        let service = create_test_service();
        let sample = serde_json::json!({"a": 1});
        let schemas = serde_json::json!({"test": {"field": "string"}});

        let prompt = service.create_prompt(&sample, &schemas, false);
        assert!(prompt.contains("Sample JSON Data:"));
        assert!(prompt.contains("\"a\": 1"));
        assert!(prompt.contains("Available Schemas:"));
        assert!(prompt.contains("\"test\""));
        assert!(prompt.contains(PROMPT_HEADER));
        assert!(prompt.contains(PROMPT_ACTIONS));
    }

    #[test]
    fn test_pretty_json_helpers() {
        let value = serde_json::json!({"x": 1});
        assert!(pretty_json(&value).contains("\"x\": 1"));
        assert!(pretty_json_or_empty(&value).contains("\"x\": 1"));
    }

    fn create_test_service() -> OllamaService {
        let config = OllamaConfig {
            model: "test-model".to_string(),
            base_url: "http://localhost:11434".to_string(),
        };
        OllamaService::new(config, 10, 3).unwrap()
    }
}
