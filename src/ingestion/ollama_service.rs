//! Ollama API service for AI-powered schema analysis

use crate::ingestion::config::OllamaConfig;
use crate::ingestion::{IngestionError, IngestionResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
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

    /// Call the Ollama API
    pub async fn call_ollama_api(&self, prompt: &str) -> IngestionResult<String> {
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        super::ai_helpers::call_with_retries(
            "Ollama API",
            self.max_retries,
            || IngestionError::ollama_error("All API attempts failed"),
            || self.make_api_request(&request),
        )
        .await
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
            .await
            .map_err(|e| crate::ingestion::error::classify_transport_error("Ollama", &e))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::ingestion::error::classify_llm_error("Ollama", status, &error_text));
        }

        let ollama_response: OllamaResponse = response.json().await?;

        Ok(ollama_response.response)
    }

}
