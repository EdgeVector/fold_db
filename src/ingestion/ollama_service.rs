//! Ollama API service for AI-powered schema analysis

use crate::ingestion::config::OllamaConfig;
use crate::ingestion::{AISchemaResponse, IngestionError, IngestionResult, StructureAnalyzer};
use crate::log_feature;
use crate::logging::features::LogFeature;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use super::prompts::{PROMPT_ACTIONS, PROMPT_HEADER};
use serde_json::Value;
use std::time::Duration;

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "Invalid JSON".to_string())
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
    ) -> IngestionResult<AISchemaResponse> {
        // Extract minimal structure skeleton (flattened paths, no data values)
        let superset_structure = StructureAnalyzer::extract_structure_skeleton(sample_json);

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

        let is_array_input = sample_json.is_array();
        let prompt = self.create_prompt(&superset_structure, is_array_input);

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

        self.parse_ai_response(&response)
    }

    /// Create the prompt for the AI
    fn create_prompt(
        &self,
        sample_json: &Value,
        is_array_input: bool,
    ) -> String {
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
                Err(_) => continue, // Keep looking if parsing fails
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

        // If all else fails, return trimmed text
        Ok(response_text.trim().to_string())
    }

    /// Validate and convert the parsed response
    fn validate_and_convert_response(&self, parsed: Value) -> IngestionResult<AISchemaResponse> {
        let obj = parsed.as_object().ok_or_else(|| {
            IngestionError::ai_response_validation_error("Response must be a JSON object")
        })?;

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
{"new_schemas": {"name": "test"}, "mutation_mappers": {}}
```
That should work."###;

        let result = service.extract_json_from_response(response_with_markers);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("new_schemas"));

        // Test with direct JSON
        let response_direct =
            r###"{"new_schemas": null, "mutation_mappers": {}}"###;
        let result = service.extract_json_from_response(response_direct);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_json_with_trailing_brace() {
        let service = create_test_service();

        let response_trailing = r###"
        {
            "new_schemas": null,
            "mutation_mappers": {}
        }
        some extra text with a } closing brace
        "###;

        let result = service.extract_json_from_response(response_trailing);
        assert!(result.is_ok());
        let json = result.unwrap();
        // Should be parseable
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("new_schemas").is_some());
    }

    #[test]
    fn test_validate_and_convert_response() {
        let service = create_test_service();

        let test_json = serde_json::json!({
            "new_schemas": null,
            "mutation_mappers": {
                "field1": "schema.field1",
                "nested.field": "schema.nested_field"
            }
        });

        let result = service.validate_and_convert_response(test_json);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.mutation_mappers.len(), 2);
    }

    #[test]
    fn test_create_prompt_includes_sample() {
        let service = create_test_service();
        let sample = serde_json::json!({"a": 1});

        let prompt = service.create_prompt(&sample, false);
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

    fn create_test_service() -> OllamaService {
        let config = OllamaConfig {
            model: "test-model".to_string(),
            base_url: "http://localhost:11434".to_string(),
        };
        OllamaService::new(config, 10, 3).unwrap()
    }
}
