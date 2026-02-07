// OpenRouter API service for AI-powered schema analysis

use super::ai_helpers::{create_prompt, parse_ai_response, pretty_json};
use crate::ingestion::config::OpenRouterConfig;
use crate::ingestion::{AISchemaResponse, IngestionError, IngestionResult, StructureAnalyzer};
use crate::log_feature;
use crate::logging::features::LogFeature;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// OpenRouter API service
pub struct OpenRouterService {
    client: Client,
    config: OpenRouterConfig,
    max_retries: u32,
}

/// Request to OpenRouter API
#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

/// Message in OpenRouter request
#[derive(Debug, Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
}

/// Response from OpenRouter API
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
    usage: Option<OpenRouterUsage>,
}

/// Choice in OpenRouter response
#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterResponseMessage,
}

/// Response message from OpenRouter
#[derive(Debug, Deserialize)]
struct OpenRouterResponseMessage {
    content: String,
}

/// Usage information from OpenRouter
#[derive(Debug, Deserialize)]
struct OpenRouterUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

impl OpenRouterService {
    /// Create a new OpenRouter service
    pub fn new(
        config: OpenRouterConfig,
        timeout_seconds: u64,
        max_retries: u32,
    ) -> IngestionResult<Self> {
        config.validate()?;

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| {
                IngestionError::openrouter_error(format!("Failed to create HTTP client: {}", e))
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
            "=== SUPERSET STRUCTURE ANALYSIS ==="
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Raw JSON data structure (first 500 chars): {}",
            if let Ok(json_str) = serde_json::to_string_pretty(sample_json) {
                if json_str.len() > 500 {
                    format!("{}...[truncated]", &json_str[..500])
                } else {
                    json_str
                }
            } else {
                "Failed to serialize JSON".to_string()
            }
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Generated superset structure: {}",
            pretty_json(&superset_structure)
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "=== END SUPERSET STRUCTURE ANALYSIS ==="
        );
        let is_array_input = sample_json.is_array();
        let prompt = create_prompt(&superset_structure, is_array_input);

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Sending request to OpenRouter API with model: {}",
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

        let response = self.call_openrouter_api(&prompt).await?;

        parse_ai_response(&response)
    }

    /// Call the OpenRouter API
    pub async fn call_openrouter_api(&self, prompt: &str) -> IngestionResult<String> {
        let request = OpenRouterRequest {
            model: self.config.model.clone(),
            messages: vec![OpenRouterMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: Some(4000),
            temperature: Some(0.1),
        };

        let mut last_error = None;

        for attempt in 1..=self.max_retries {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "OpenRouter API attempt {} of {}",
                attempt,
                self.max_retries
            );

            let start_time = std::time::Instant::now();
            match self.make_api_request(&request).await {
                Ok(response) => {
                    let elapsed = start_time.elapsed();
                    log_feature!(
                        LogFeature::Ingestion,
                        info,
                        "OpenRouter API call successful on attempt {} (took {:.2?})",
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
                        "OpenRouter API attempt {} failed (took {:.2?}): {}",
                        attempt,
                        elapsed,
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

        Err(last_error
            .unwrap_or_else(|| IngestionError::openrouter_error("All API attempts failed")))
    }

    /// Make a single API request
    async fn make_api_request(&self, request: &OpenRouterRequest) -> IngestionResult<String> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/datafold/datafold")
            .header("X-Title", "DataFold Ingestion")
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IngestionError::openrouter_error(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }

        let openrouter_response: OpenRouterResponse = response.json().await?;

        if let Some(usage) = &openrouter_response.usage {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "OpenRouter API usage - Prompt tokens: {:?}, Completion tokens: {:?}, Total tokens: {:?}",
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.total_tokens
            );
        }

        if openrouter_response.choices.is_empty() {
            return Err(IngestionError::openrouter_error(
                "No choices in API response",
            ));
        }

        Ok(openrouter_response.choices[0].message.content.clone())
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
        // This failing case was reported by user: trailing characters with a brace
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

    #[tokio::test]
    async fn test_timeout_configuration() {
        // Find a random available port
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{}", port);

        // Spawn a thread that accepts a connection and sleeps longer than the timeout
        tokio::spawn(async move {
            std::thread::spawn(move || {
                let _ = listener.accept();
                std::thread::sleep(std::time::Duration::from_secs(5));
            });
        });

        // Config with 1 second timeout
        let config = OpenRouterConfig {
            api_key: "test-key".to_string(),
            base_url,
            ..Default::default()
        };

        // Create service with 1 second timeout, 0 retries to fail fast
        let service = OpenRouterService::new(config, 1, 0).unwrap();

        // Make a request - it should timeout
        let result = service.call_openrouter_api("test").await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.to_lowercase().contains("time") || error_msg.to_lowercase().contains("out"),
            "Error message '{}' did not contain 'time' or 'out' indicating a timeout",
            error_msg
        );
    }
}
