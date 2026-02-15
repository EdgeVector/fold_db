// OpenRouter API service for AI-powered schema analysis

use crate::ingestion::config::OpenRouterConfig;
use crate::ingestion::{IngestionError, IngestionResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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

        super::ai_helpers::call_with_retries(
            "OpenRouter API",
            self.max_retries,
            || IngestionError::openrouter_error("All API attempts failed"),
            || self.make_api_request(&request),
        )
        .await
    }

    /// Make a single API request
    async fn make_api_request(&self, request: &OpenRouterRequest) -> IngestionResult<String> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/shiba4life/fold_db")
            .header("X-Title", "FoldDB Ingestion")
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
