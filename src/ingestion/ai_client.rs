//! Unified AI backend abstraction for OpenRouter and Ollama.

use crate::ingestion::config::{AIProvider, IngestionConfig, OllamaConfig, OpenRouterConfig};
use crate::ingestion::{IngestionError, IngestionResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Trait implemented by each AI provider backend.
#[async_trait]
pub trait AiBackend: Send + Sync {
    async fn call(&self, prompt: &str) -> IngestionResult<String>;
}

// ---- OpenRouter ----

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
    usage: Option<OpenRouterUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

pub struct OpenRouterBackend {
    client: Client,
    config: OpenRouterConfig,
    max_retries: u32,
}

impl OpenRouterBackend {
    pub fn new(config: OpenRouterConfig, timeout_seconds: u64, max_retries: u32) -> IngestionResult<Self> {
        config.validate()?;
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .no_proxy()
            .build()
            .map_err(|e| IngestionError::openrouter_error(format!("Failed to create HTTP client: {}", e)))?;
        Ok(Self { client, config, max_retries })
    }

    async fn make_request(&self, request: &OpenRouterRequest) -> IngestionResult<String> {
        let url = format!("{}/chat/completions", self.config.base_url);
        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/shiba4life/fold_db")
            .header("X-Title", "FoldDB Ingestion")
            .json(request)
            .send()
            .await
            .map_err(|e| crate::ingestion::error::classify_transport_error("OpenRouter", &e))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_else(|e| {
                log::warn!("Failed to read OpenRouter error response body: {}", e);
                "Unknown error (response body unreadable)".to_string()
            });
            return Err(crate::ingestion::error::classify_llm_error("OpenRouter", status, &error_text));
        }

        let resp: OpenRouterResponse = response.json().await?;
        if let Some(usage) = &resp.usage {
            log_feature!(
                LogFeature::Ingestion, info,
                "OpenRouter usage - prompt: {:?}, completion: {:?}, total: {:?}",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
            );
        }
        if resp.choices.is_empty() {
            return Err(IngestionError::openrouter_error("No choices in API response"));
        }
        Ok(resp.choices[0].message.content.clone())
    }
}

#[async_trait]
impl AiBackend for OpenRouterBackend {
    async fn call(&self, prompt: &str) -> IngestionResult<String> {
        let request = OpenRouterRequest {
            model: self.config.model.clone(),
            messages: vec![OpenRouterMessage { role: "user".to_string(), content: prompt.to_string() }],
            max_tokens: Some(16000),
            temperature: Some(0.1),
        };
        super::ai_helpers::call_with_retries(
            "OpenRouter API",
            self.max_retries,
            || IngestionError::openrouter_error("All API attempts failed"),
            || self.make_request(&request),
        ).await
    }
}

// ---- Ollama ----

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

pub struct OllamaBackend {
    client: Client,
    config: OllamaConfig,
    max_retries: u32,
}

impl OllamaBackend {
    pub fn new(config: OllamaConfig, timeout_seconds: u64, max_retries: u32) -> IngestionResult<Self> {
        config.validate()?;
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .no_proxy()
            .build()
            .map_err(|e| IngestionError::ollama_error(format!("Failed to create HTTP client: {}", e)))?;
        Ok(Self { client, config, max_retries })
    }

    async fn make_request(&self, request: &OllamaRequest) -> IngestionResult<String> {
        let url = format!("{}/api/generate", self.config.base_url);
        let response = self.client.post(&url)
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| crate::ingestion::error::classify_transport_error("Ollama", &e))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::ingestion::error::classify_llm_error("Ollama", status, &error_text));
        }

        let resp: OllamaResponse = response.json().await?;
        Ok(resp.response)
    }
}

#[async_trait]
impl AiBackend for OllamaBackend {
    async fn call(&self, prompt: &str) -> IngestionResult<String> {
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };
        super::ai_helpers::call_with_retries(
            "Ollama API",
            self.max_retries,
            || IngestionError::ollama_error("All API attempts failed"),
            || self.make_request(&request),
        ).await
    }
}

// ---- Factory ----

/// Build the correct backend from an `IngestionConfig`.
///
/// Returns `Ok(None)` when the configured provider fails validation so that
/// `IngestionService` can still be constructed and report status.
pub fn build_backend(config: &IngestionConfig) -> (Option<Arc<dyn AiBackend>>, Option<String>) {
    match config.provider {
        AIProvider::OpenRouter => match OpenRouterBackend::new(
            config.openrouter.clone(), config.timeout_seconds, config.max_retries,
        ) {
            Ok(b) => (Some(Arc::new(b)), None),
            Err(e) => {
                let msg = format!("OpenRouter init failed: {}", e);
                log::warn!("{}", msg);
                (None, Some(msg))
            }
        },
        AIProvider::Ollama => match OllamaBackend::new(
            config.ollama.clone(), config.timeout_seconds, config.max_retries,
        ) {
            Ok(b) => (Some(Arc::new(b)), None),
            Err(e) => {
                let msg = format!("Ollama init failed: {}", e);
                log::warn!("{}", msg);
                (None, Some(msg))
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timeout_configuration() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{}", port);

        let _handle = std::thread::spawn(move || {
            if let Ok((_stream, _addr)) = listener.accept() {
                std::thread::sleep(std::time::Duration::from_secs(10));
            }
        });

        let config = OpenRouterConfig {
            api_key: "test-key".to_string(),
            base_url,
            ..Default::default()
        };

        let backend = OpenRouterBackend::new(config, 1, 1).unwrap();
        let result = backend.call("test").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, IngestionError::TimeoutError { .. }),
            "Expected TimeoutError, got: {:?}",
            err
        );
    }
}
