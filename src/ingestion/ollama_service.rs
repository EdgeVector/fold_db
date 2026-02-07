//! Ollama API service for AI-powered schema analysis

use super::ai_helpers::parse_ai_response;
use crate::ingestion::config::OllamaConfig;
use crate::ingestion::{AISchemaResponse, IngestionError, IngestionResult};
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
        let prompt = super::ai_helpers::analyze_and_build_prompt(sample_json)?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Sending request to Ollama API with model: {}",
            self.config.model
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
