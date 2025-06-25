//! OpenRouter API service for AI-powered schema analysis

use crate::ingestion::{IngestionConfig, IngestionError, IngestionResult};
use crate::logging::features::LogFeature;
use crate::log_feature;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Prompt header shared across requests
const PROMPT_HEADER: &str = r#"Tell me which of these schemas to use for this sample json data. If none are available, then create a new one. Return the value in this format:
{
  \"existing_schemas\": [<list_of_schema_names>],
  \"new_schemas\": <single_schema_definition>,
  \"mutation_mappers\": {json_field_path: schema_field_path}
}

Where:
- existing_schemas is an array of schema names that match the input data
- new_schemas is a single schema definition if no existing schemas match
- mutation_mappers maps JSON paths (like \"path.field[0]\") to schema paths (like \"schema.field[\\\"key\\\"]\")
"#;

/// Instructions appended to every prompt
const PROMPT_ACTIONS: &str = r#"Please analyze the sample data and either:
1. If existing schemas can handle this data, return their names in existing_schemas and provide mutation_mappers
2. If no existing schemas match, create a new schema definition in new_schemas and provide mutation_mappers

The response must be valid JSON."#;

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "Invalid JSON".to_string())
}

fn pretty_json_or_empty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
}

/// OpenRouter API service
pub struct OpenRouterService {
    client: Client,
    config: IngestionConfig,
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

/// Parsed AI response for schema analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct AISchemaResponse {
    /// List of existing schema names that match the input data
    pub existing_schemas: Vec<String>,
    /// New schema definition if no existing schemas match
    pub new_schemas: Option<Value>,
    /// Mapping from JSON field paths to schema field paths
    pub mutation_mappers: std::collections::HashMap<String, String>,
}

impl OpenRouterService {
    /// Create a new OpenRouter service
    pub fn new(config: IngestionConfig) -> IngestionResult<Self> {
        config.validate()?;

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| {
                IngestionError::openrouter_error(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client, config })
    }

    /// Get schema recommendation from AI
    pub async fn get_schema_recommendation(
        &self,
        sample_json: &Value,
        available_schemas: &Value,
    ) -> IngestionResult<AISchemaResponse> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Sample JSON data: {}",
            pretty_json(sample_json)
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Available schemas: {}",
            pretty_json_or_empty(available_schemas)
        );

        let prompt = self.create_prompt(sample_json, available_schemas);

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Sending request to OpenRouter API with model: {}",
            self.config.openrouter_model
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
    fn create_prompt(&self, sample_json: &Value, available_schemas: &Value) -> String {
        format!(
            "{header}\n\nSample JSON Data:\n{sample}\n\nAvailable Schemas:\n{schemas}\n\n{actions}",
            header = PROMPT_HEADER,
            sample = pretty_json(sample_json),
            schemas = pretty_json_or_empty(available_schemas),
            actions = PROMPT_ACTIONS
        )
    }

    /// Call the OpenRouter API
    async fn call_openrouter_api(&self, prompt: &str) -> IngestionResult<String> {
        let request = OpenRouterRequest {
            model: self.config.openrouter_model.clone(),
            messages: vec![OpenRouterMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: Some(4000),
            temperature: Some(0.1),
        };

        let mut last_error = None;

        for attempt in 1..=self.config.max_retries {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "OpenRouter API attempt {} of {}",
                attempt,
                self.config.max_retries
            );

            match self.make_api_request(&request).await {
                Ok(response) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        info,
                        "OpenRouter API call successful on attempt {}",
                        attempt
                    );
                    return Ok(response);
                }
                Err(e) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        warn,
                        "OpenRouter API attempt {} failed: {}",
                        attempt,
                        e
                    );
                    last_error = Some(e);

                    if attempt < self.config.max_retries {
                        // Exponential backoff
                        let delay = Duration::from_secs(2_u64.pow(attempt - 1));
                        log_feature!(LogFeature::Ingestion, info, "Retrying in {:?}", delay);
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
        let url = format!("{}/chat/completions", self.config.openrouter_base_url);

        let response = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.openrouter_api_key),
            )
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

    /// Parse the AI response
    fn parse_ai_response(&self, response_text: &str) -> IngestionResult<AISchemaResponse> {
        log_feature!(LogFeature::Ingestion, info, "=== PARSING AI RESPONSE ===");
        log_feature!(LogFeature::Ingestion, info, "Raw AI response text: {}", response_text);

        // Try to extract JSON from the response
        let json_str = self.extract_json_from_response(response_text)?;
        log_feature!(LogFeature::Ingestion, info, "Extracted JSON string: {}", json_str);

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

        log_feature!(LogFeature::Ingestion, info, "=== FINAL PARSED AI RESPONSE ===");
        log_feature!(LogFeature::Ingestion, info, "Existing schemas: {:?}", result.existing_schemas);
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
        log_feature!(LogFeature::Ingestion, info, "Mutation mappers: {:?}", result.mutation_mappers);
        log_feature!(LogFeature::Ingestion, info, "=== END PARSED AI RESPONSE ===");

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

    #[test]
    fn test_extract_json_from_response() {
        let service = create_test_service();

        // Test with JSON block markers
        let response_with_markers = r#"Here's the analysis:
```json
{"existing_schemas": ["test"], "new_schemas": null, "mutation_mappers": {}}
```
That should work."#;

        let result = service.extract_json_from_response(response_with_markers);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("existing_schemas"));

        // Test with direct JSON
        let response_direct =
            r#"{"existing_schemas": ["test"], "new_schemas": null, "mutation_mappers": {}}"#;
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

        let prompt = service.create_prompt(&sample, &schemas);
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

    fn create_test_service() -> OpenRouterService {
        let config = IngestionConfig {
            openrouter_api_key: "test-key".to_string(),
            ..Default::default()
        };
        OpenRouterService::new(config).unwrap()
    }
}
