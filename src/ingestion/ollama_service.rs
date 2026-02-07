//! Ollama API service for AI-powered schema analysis

use super::ai_helpers::{create_prompt, parse_ai_response, pretty_json};
use crate::ingestion::config::OllamaConfig;
use crate::ingestion::{AISchemaResponse, IngestionError, IngestionResult, StructureAnalyzer};
use crate::log_feature;
use crate::logging::features::LogFeature;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

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
        let prompt = create_prompt(&superset_structure, is_array_input);

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
                format!("{}...[truncated]", &prompt[..1000])
            } else {
                prompt.clone()
            }
        );

        let response = self.call_ollama_api(&prompt).await?;

        parse_ai_response(&response)
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

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion::ai_helpers::{
        create_prompt, extract_json_from_response, validate_and_convert_response,
    };
    use crate::ingestion::prompts::{PROMPT_ACTIONS, PROMPT_HEADER};

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
        let response_direct =
            r###"{"new_schemas": null, "mutation_mappers": {}}"###;
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
        // Should be parseable
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

}
