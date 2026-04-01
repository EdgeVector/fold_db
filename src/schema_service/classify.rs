//! Field sensitivity classification and interest category inference.
//!
//! Determines the (sensitivity_level, data_domain) and interest_category for
//! new canonical fields based on the field's description. The schema service
//! is the sole authority on both data classification and interest categories.
//!
//! Strategy for new fields without an existing canonical match:
//! 1. Caller-provided classification → use it
//! 2. LLM call using field description (Ollama by default, Anthropic with ANTHROPIC_API_KEY)
//! 3. No fallback — returns error. Incorrect classification is worse than no schema.

use crate::llm_registry::models;
use crate::llm_registry::prompts::classification::{
    build_classification_prompt, build_interest_category_prompt, INTEREST_CATEGORIES,
};
use crate::schema::types::data_classification::DataClassification;
use serde::{Deserialize, Serialize};

// ---- Provider resolution ----

enum ClassifyProvider {
    Anthropic,
    Ollama,
}

/// Determine which LLM provider to use for classification.
///
/// Precedence:
/// 1. `AI_PROVIDER` env var ("ollama" or "anthropic") — explicit override
/// 2. `ANTHROPIC_API_KEY` set and non-empty — use Anthropic
/// 3. Default — Ollama (local dev)
fn resolve_provider() -> ClassifyProvider {
    if let Ok(p) = std::env::var("AI_PROVIDER") {
        return match p.to_lowercase().as_str() {
            "anthropic" => ClassifyProvider::Anthropic,
            _ => ClassifyProvider::Ollama,
        };
    }
    if std::env::var("ANTHROPIC_API_KEY")
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false)
    {
        return ClassifyProvider::Anthropic;
    }
    ClassifyProvider::Ollama
}

// ---- Ollama wire types ----

#[derive(Debug, Serialize)]
struct OllamaClassifyRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaClassifyOptions,
}

#[derive(Debug, Serialize)]
struct OllamaClassifyOptions {
    num_ctx: u32,
    temperature: f32,
    num_predict: u32,
}

#[derive(Debug, Deserialize)]
struct OllamaClassifyResponse {
    response: String,
}

// ---- LLM call helpers ----

/// Call Ollama's `/api/generate` endpoint with a classification prompt.
/// Reads `OLLAMA_BASE_URL` and `OLLAMA_MODEL` from environment, with defaults.
async fn call_ollama(prompt: &str, field_name: &str) -> Result<String, String> {
    let base_url =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| models::OLLAMA_DEFAULT_URL.to_string());
    let model =
        std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| models::OLLAMA_DEFAULT.to_string());
    call_ollama_with(prompt, field_name, &base_url, &model).await
}

/// Inner implementation that accepts explicit base_url and model (testable without env vars).
async fn call_ollama_with(
    prompt: &str,
    field_name: &str,
    base_url: &str,
    model: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            models::TIMEOUT_CLASSIFICATION_OLLAMA,
        ))
        .no_proxy()
        .build()
        .map_err(|e| {
            format!(
                "Failed to create HTTP client for Ollama classification: {}",
                e
            )
        })?;

    let request = OllamaClassifyRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
        stream: false,
        options: OllamaClassifyOptions {
            num_ctx: 4096,
            temperature: models::TEMPERATURE_DETERMINISTIC,
            num_predict: models::MAX_TOKENS_CLASSIFICATION_OLLAMA,
        },
    };

    let url = format!("{}/api/generate", base_url);
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                format!(
                    "Schema service cannot classify field '{}': \
                     Ollama not reachable at {}. Ensure Ollama is running.",
                    field_name, base_url
                )
            } else if e.is_timeout() {
                format!(
                    "Classification via Ollama timed out for field '{}' (model: {})",
                    field_name, model
                )
            } else {
                format!(
                    "Classification via Ollama failed for field '{}': {}",
                    field_name, e
                )
            }
        })?;

    if !response.status().is_success() {
        return Err(format!(
            "Ollama classification returned status {} for field '{}' (model: {})",
            response.status(),
            field_name,
            model
        ));
    }

    let resp: OllamaClassifyResponse = response.json().await.map_err(|e| {
        format!(
            "Failed to parse Ollama response for field '{}': {}",
            field_name, e
        )
    })?;

    Ok(resp.response)
}

/// Call Anthropic's Messages API with a classification prompt.
async fn call_anthropic(prompt: &str, field_name: &str) -> Result<String, String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
        "Schema service cannot classify new fields: ANTHROPIC_API_KEY not set. \
         Set the environment variable to enable automatic sensitivity classification."
            .to_string()
    })?;
    if api_key.trim().is_empty() {
        return Err(
            "Schema service cannot classify new fields: ANTHROPIC_API_KEY is empty".to_string(),
        );
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            models::TIMEOUT_CLASSIFICATION,
        ))
        .no_proxy()
        .build()
        .map_err(|e| format!("Failed to create HTTP client for classification: {}", e))?;

    let request_body = serde_json::json!({
        "model": models::ANTHROPIC_HAIKU,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": models::MAX_TOKENS_CLASSIFICATION,
        "temperature": models::TEMPERATURE_DETERMINISTIC
    });

    let response = client
        .post(format!("{}/v1/messages", models::ANTHROPIC_API_URL))
        .header("x-api-key", &api_key)
        .header("anthropic-version", models::ANTHROPIC_API_VERSION)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            format!(
                "Classification LLM call failed for field '{}': {}",
                field_name, e
            )
        })?;

    if !response.status().is_success() {
        return Err(format!(
            "Classification LLM call returned status {} for field '{}'",
            response.status(),
            field_name
        ));
    }

    let resp: serde_json::Value = response.json().await.map_err(|e| {
        format!(
            "Failed to parse LLM response for field '{}': {}",
            field_name, e
        )
    })?;

    let text = resp
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| {
            format!(
                "LLM response missing content text for field '{}'",
                field_name
            )
        })?;

    Ok(text.to_string())
}

/// Dispatch a classification prompt to the resolved LLM provider.
async fn call_llm(prompt: &str, field_name: &str) -> Result<String, String> {
    match resolve_provider() {
        ClassifyProvider::Anthropic => call_anthropic(prompt, field_name).await,
        ClassifyProvider::Ollama => call_ollama(prompt, field_name).await,
    }
}

// ---- JSON parsing helpers ----

/// Strip markdown code fences from LLM output, if present.
fn strip_markdown_fences(text: &str) -> &str {
    let trimmed = text.trim();
    trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .unwrap_or(trimmed)
        .trim()
}

/// Parse a DataClassification from LLM text output.
fn parse_classification_json(field_name: &str, text: &str) -> Result<DataClassification, String> {
    serde_json::from_str(text)
        .or_else(|_| serde_json::from_str(strip_markdown_fences(text)))
        .map_err(|e| {
            format!(
                "Failed to parse LLM classification for field '{}': {} (raw: {})",
                field_name, e, text
            )
        })
}

/// Parse an interest category from LLM text output.
/// Returns `Ok(None)` if the LLM returned null or an unrecognized category.
fn parse_interest_category_json(field_name: &str, text: &str) -> Result<Option<String>, String> {
    let parsed: serde_json::Value = serde_json::from_str(text)
        .or_else(|_| serde_json::from_str(strip_markdown_fences(text)))
        .map_err(|e| {
            format!(
                "Failed to parse LLM interest category for field '{}': {} (raw: {})",
                field_name, e, text
            )
        })?;

    let category = parsed
        .get("interest_category")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Validate against known categories
    let validated = category.filter(|cat| {
        INTEREST_CATEGORIES
            .iter()
            .any(|valid| valid.eq_ignore_ascii_case(cat))
    });

    Ok(validated)
}

// ---- Public API ----

/// Classify a field using LLM analysis of its description.
/// Returns a descriptive error string on failure.
pub async fn classify_with_llm(
    field_name: &str,
    description: &str,
) -> Result<DataClassification, String> {
    let prompt = build_classification_prompt(field_name, description);
    let text = call_llm(&prompt, field_name).await?;
    let classification = parse_classification_json(field_name, &text)?;

    crate::log_feature!(
        crate::logging::features::LogFeature::Schema,
        info,
        "LLM classified field '{}' as ({}, {})",
        field_name,
        classification.sensitivity_level,
        classification.data_domain
    );

    Ok(classification)
}

/// Classify a field's interest category using LLM analysis of its description.
/// Returns `Ok(None)` if the field doesn't map to any interest category (structural fields).
/// Returns `Err` only on LLM communication failures.
pub async fn classify_interest_category_with_llm(
    field_name: &str,
    description: &str,
) -> Result<Option<String>, String> {
    let prompt = build_interest_category_prompt(field_name, description);
    let text = call_llm(&prompt, field_name).await?;
    let validated = parse_interest_category_json(field_name, &text)?;

    if let Some(ref cat) = validated {
        crate::log_feature!(
            crate::logging::features::LogFeature::Schema,
            info,
            "LLM classified field '{}' interest category as '{}'",
            field_name,
            cat
        );
    }

    Ok(validated)
}

/// Infer interest category for a new canonical field.
/// Returns `Ok(None)` for structural fields or when the LLM is unreachable.
/// Interest category is best-effort — missing it doesn't block schema creation.
pub async fn infer_interest_category(field_name: &str, description: &str) -> Option<String> {
    match classify_interest_category_with_llm(field_name, description).await {
        Ok(category) => category,
        Err(e) => {
            crate::log_feature!(
                crate::logging::features::LogFeature::Schema,
                warn,
                "Interest category classification failed for field '{}': {} (non-blocking)",
                field_name,
                e
            );
            None
        }
    }
}

/// Infer classification for a new canonical field.
/// Returns an error if classification cannot be determined — no silent fallbacks.
///
/// ```text
/// caller-provided? ──yes──▶ use it
///       │ no
///       ▼
/// LLM call (Ollama or Anthropic) ──success──▶ use it
///       │ failure
///       ▼
/// ERROR: schema service cannot classify
/// ```
pub async fn infer_classification(
    field_name: &str,
    description: &str,
    caller_provided: Option<&DataClassification>,
) -> Result<DataClassification, String> {
    if let Some(c) = caller_provided {
        return Ok(c.clone());
    }

    classify_with_llm(field_name, description).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn resolve_provider_defaults_to_ollama() {
        // When no env vars are set, should default to Ollama
        // (This test assumes CI doesn't set AI_PROVIDER or ANTHROPIC_API_KEY)
        temp_env::with_vars_unset(["AI_PROVIDER", "ANTHROPIC_API_KEY"], || {
            assert!(matches!(resolve_provider(), ClassifyProvider::Ollama));
        });
    }

    #[test]
    #[serial]
    fn resolve_provider_respects_ai_provider_env() {
        temp_env::with_vars(
            [
                ("AI_PROVIDER", Some("ollama")),
                ("ANTHROPIC_API_KEY", Some("sk-test")),
            ],
            || {
                // AI_PROVIDER=ollama should win even if ANTHROPIC_API_KEY is set
                assert!(matches!(resolve_provider(), ClassifyProvider::Ollama));
            },
        );
        temp_env::with_vars(
            [
                ("AI_PROVIDER", Some("anthropic")),
                ("ANTHROPIC_API_KEY", None::<&str>),
            ],
            || {
                assert!(matches!(resolve_provider(), ClassifyProvider::Anthropic));
            },
        );
    }

    #[test]
    #[serial]
    fn resolve_provider_uses_anthropic_when_key_set() {
        temp_env::with_vars(
            [
                ("AI_PROVIDER", None::<&str>),
                ("ANTHROPIC_API_KEY", Some("sk-test")),
            ],
            || {
                assert!(matches!(resolve_provider(), ClassifyProvider::Anthropic));
            },
        );
    }

    #[test]
    fn strip_markdown_fences_works() {
        assert_eq!(strip_markdown_fences(r#"{"a": 1}"#), r#"{"a": 1}"#);
        assert_eq!(
            strip_markdown_fences("```json\n{\"a\": 1}\n```"),
            "{\"a\": 1}"
        );
        assert_eq!(strip_markdown_fences("```\n{\"a\": 1}\n```"), "{\"a\": 1}");
    }

    #[test]
    fn parse_classification_json_valid() {
        let text = r#"{"sensitivity_level": 3, "data_domain": "identity"}"#;
        let result = parse_classification_json("test_field", text).unwrap();
        assert_eq!(result.sensitivity_level, 3);
        assert_eq!(result.data_domain, "identity");
    }

    #[test]
    fn parse_classification_json_with_fences() {
        let text = "```json\n{\"sensitivity_level\": 1, \"data_domain\": \"general\"}\n```";
        let result = parse_classification_json("test_field", text).unwrap();
        assert_eq!(result.sensitivity_level, 1);
        assert_eq!(result.data_domain, "general");
    }

    #[test]
    fn parse_interest_category_json_valid() {
        let text = r#"{"interest_category": "Photography"}"#;
        let result = parse_interest_category_json("photo_album", text).unwrap();
        assert_eq!(result.as_deref(), Some("Photography"));
    }

    #[test]
    fn parse_interest_category_json_null() {
        let text = r#"{"interest_category": null}"#;
        let result = parse_interest_category_json("id_field", text).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn parse_interest_category_json_invalid_category() {
        let text = r#"{"interest_category": "NotACategory"}"#;
        let result = parse_interest_category_json("test_field", text).unwrap();
        assert!(result.is_none());
    }

    // ---- call_ollama tests ----

    /// Spawn a minimal HTTP server on a background OS thread (not a tokio task)
    /// to avoid scheduling interference from the test runtime under load.
    /// Accepts one request, returns the given status + body, then exits.
    fn mock_http_server(status: u16, body: &str) -> (String, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{}", port);
        let body = body.to_string();
        let handle = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut stream, _) = listener.accept().unwrap();
            // Read the full HTTP request (headers + body).
            let mut buf = Vec::new();
            let mut tmp = [0u8; 1024];
            loop {
                let n = stream.read(&mut tmp).unwrap_or(0);
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                if let Some(header_end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&buf[..header_end]);
                    let content_length = headers
                        .lines()
                        .find(|l| l.to_lowercase().starts_with("content-length:"))
                        .and_then(|l| l.split(':').nth(1))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    let body_received = buf.len() - (header_end + 4);
                    if body_received >= content_length {
                        break;
                    }
                }
            }
            let response = format!(
                "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status,
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        (url, handle)
    }

    /// RAII guard that sets env vars on creation and restores originals on drop.
    /// Needed because `temp_env::with_vars` takes a sync closure, which can't
    /// contain `.await` calls.
    struct EnvGuard {
        originals: Vec<(String, Option<String>)>,
    }

    impl EnvGuard {
        fn set(vars: &[(&str, Option<&str>)]) -> Self {
            let mut originals = Vec::new();
            for (key, val) in vars {
                originals.push((key.to_string(), std::env::var(key).ok()));
                match val {
                    Some(v) => std::env::set_var(key, v),
                    None => std::env::remove_var(key),
                }
            }
            Self { originals }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, val) in &self.originals {
                match val {
                    Some(v) => std::env::set_var(key, v),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    #[tokio::test]
    async fn call_ollama_success() {
        let response_body =
            r#"{"response": "{\"sensitivity_level\": 2, \"data_domain\": \"financial\"}"}"#;
        let (url, handle) = mock_http_server(200, response_body);

        let result = call_ollama_with("classify this field", "salary", &url, "test-model").await;
        let text = result.unwrap();
        assert!(text.contains("sensitivity_level"));
        assert!(text.contains("financial"));
        handle.join().unwrap();
    }

    #[tokio::test]
    async fn call_ollama_non_success_status() {
        let (url, handle) = mock_http_server(404, r#"{"error": "model not found"}"#);

        let result = call_ollama_with("test prompt", "test_field", &url, "nonexistent-model").await;
        let err = result.unwrap_err();
        assert!(err.contains("status 404"), "got: {}", err);
        assert!(err.contains("test_field"), "got: {}", err);
        handle.join().unwrap();
    }

    #[tokio::test]
    async fn call_ollama_connection_refused() {
        let result = call_ollama_with(
            "test prompt",
            "test_field",
            "http://127.0.0.1:1",
            "test-model",
        )
        .await;
        let err = result.unwrap_err();
        assert!(
            err.contains("not reachable") || err.contains("Ollama"),
            "got: {}",
            err
        );
    }

    #[tokio::test]
    async fn call_ollama_timeout() {
        // Start a server that accepts but never responds
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{}", port);

        let _hold = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;
            drop(stream);
        });

        // Build a client with a very short timeout to avoid a slow test
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .no_proxy()
            .build()
            .unwrap();

        let request = OllamaClassifyRequest {
            model: "test-model".to_string(),
            prompt: "test".to_string(),
            stream: false,
            options: OllamaClassifyOptions {
                num_ctx: 4096,
                temperature: 0.0,
                num_predict: 256,
            },
        };

        let result = client
            .post(format!("{}/api/generate", url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_timeout(), "expected timeout, got: {:?}", err);
    }

    // ---- call_anthropic tests ----

    #[tokio::test]
    #[serial]
    async fn call_anthropic_missing_key() {
        let _env = EnvGuard::set(&[("ANTHROPIC_API_KEY", None)]);

        let result = call_anthropic("test prompt", "test_field").await;
        let err = result.unwrap_err();
        assert!(err.contains("ANTHROPIC_API_KEY not set"), "got: {}", err);
    }

    #[tokio::test]
    #[serial]
    async fn call_anthropic_empty_key() {
        let _env = EnvGuard::set(&[("ANTHROPIC_API_KEY", Some("   "))]);

        let result = call_anthropic("test prompt", "test_field").await;
        let err = result.unwrap_err();
        assert!(err.contains("ANTHROPIC_API_KEY is empty"), "got: {}", err);
    }

    // ---- call_llm dispatch tests ----

    #[tokio::test]
    #[serial]
    async fn call_llm_dispatches_to_ollama_by_default() {
        let _env = EnvGuard::set(&[
            ("AI_PROVIDER", None),
            ("ANTHROPIC_API_KEY", None),
            ("OLLAMA_BASE_URL", Some("http://127.0.0.1:1")),
        ]);

        let result = call_llm("test prompt", "test_field").await;
        let err = result.unwrap_err();
        assert!(err.contains("Ollama"), "got: {}", err);
        assert!(!err.contains("ANTHROPIC_API_KEY"), "got: {}", err);
    }

    #[tokio::test]
    #[serial]
    async fn call_llm_dispatches_to_anthropic_when_requested() {
        let _env = EnvGuard::set(&[
            ("AI_PROVIDER", Some("anthropic")),
            ("ANTHROPIC_API_KEY", None),
        ]);

        let result = call_llm("test prompt", "test_field").await;
        let err = result.unwrap_err();
        assert!(err.contains("ANTHROPIC_API_KEY"), "got: {}", err);
    }

    // ---- resolve_provider edge cases ----

    #[test]
    #[serial]
    fn resolve_provider_empty_anthropic_key_falls_to_ollama() {
        temp_env::with_vars(
            [
                ("AI_PROVIDER", None::<&str>),
                ("ANTHROPIC_API_KEY", Some("   ")),
            ],
            || {
                assert!(matches!(resolve_provider(), ClassifyProvider::Ollama));
            },
        );
    }

    // ---- parse error paths ----

    #[test]
    fn parse_classification_json_invalid() {
        let result = parse_classification_json("field", "not valid json");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("field"), "got: {}", err);
        assert!(err.contains("raw: not valid json"), "got: {}", err);
    }

    #[test]
    fn parse_interest_category_json_invalid() {
        let result = parse_interest_category_json("field", "not valid json");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("field"), "got: {}", err);
    }

    // ---- end-to-end via mock server ----

    #[tokio::test]
    async fn classify_ollama_end_to_end() {
        // Tests the full flow: call_ollama_with → parse_classification_json
        let ollama_response =
            r#"{"response": "{\"sensitivity_level\": 3, \"data_domain\": \"identity\"}"}"#;
        let (url, handle) = mock_http_server(200, ollama_response);

        let text = call_ollama_with("classify this field", "ssn", &url, "test-model")
            .await
            .unwrap();
        let classification = parse_classification_json("ssn", &text).unwrap();
        assert_eq!(classification.sensitivity_level, 3);
        assert_eq!(classification.data_domain, "identity");
        handle.join().unwrap();
    }

    #[tokio::test]
    async fn classify_interest_category_ollama_end_to_end() {
        // Tests the full flow: call_ollama_with → parse_interest_category_json
        let ollama_response = r#"{"response": "{\"interest_category\": \"Photography\"}"}"#;
        let (url, handle) = mock_http_server(200, ollama_response);

        let text = call_ollama_with("classify this field", "photo_album", &url, "test-model")
            .await
            .unwrap();
        let result = parse_interest_category_json("photo_album", &text).unwrap();
        assert_eq!(result.as_deref(), Some("Photography"));
        handle.join().unwrap();
    }

    #[tokio::test]
    async fn infer_uses_caller_provided_first() {
        let caller = DataClassification::new(4, "medical").unwrap();
        let result = infer_classification("diagnosis", "patient diagnosis", Some(&caller)).await;
        let c = result.unwrap();
        assert_eq!(c.sensitivity_level, 4);
        assert_eq!(c.data_domain, "medical");
    }

    #[tokio::test]
    #[serial]
    async fn infer_without_caller_uses_llm_or_errors() {
        let result = infer_classification("salary", "employee annual salary", None).await;
        match result {
            Ok(c) => {
                assert!(c.sensitivity_level <= 4);
                assert!(!c.data_domain.is_empty());
            }
            Err(e) => {
                // Could be Anthropic key missing or Ollama not reachable
                assert!(
                    e.contains("ANTHROPIC_API_KEY") || e.contains("Ollama"),
                    "got: {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn infer_interest_category_returns_none_without_llm() {
        // Without a running LLM, should return None (non-blocking)
        let result = infer_interest_category("photo_album", "the album containing the photo").await;
        // Either returns a valid category (if LLM is available) or None
        if let Some(ref cat) = result {
            assert!(
                INTEREST_CATEGORIES
                    .iter()
                    .any(|valid| valid.eq_ignore_ascii_case(cat)),
                "Invalid category: {}",
                cat
            );
        }
    }
}
