//! Field sensitivity classification inference.
//!
//! Determines the (sensitivity_level, data_domain) for new canonical fields
//! based on the field's description. The schema service is the sole authority
//! on data classification.
//!
//! Strategy for new fields without an existing canonical match:
//! 1. Caller-provided classification → use it
//! 2. LLM call using field description (requires ANTHROPIC_API_KEY)
//! 3. No fallback — returns error. Incorrect classification is worse than no schema.

use crate::llm_registry::models;
use crate::llm_registry::prompts::classification::build_classification_prompt;
use crate::schema::types::data_classification::DataClassification;

/// Classify a field using LLM analysis of its description.
/// Returns a descriptive error string on failure.
pub async fn classify_with_llm(
    field_name: &str,
    description: &str,
) -> Result<DataClassification, String> {
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

    let prompt = build_classification_prompt(field_name, description);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(models::TIMEOUT_CLASSIFICATION))
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

    let resp: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse LLM response for field '{}': {}", field_name, e))?;

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

    // Parse the JSON response — try raw text first, then extract from markdown fence
    let classification: DataClassification = serde_json::from_str(text)
        .or_else(|_| {
            let trimmed = text.trim();
            let json_str = trimmed
                .strip_prefix("```json")
                .or_else(|| trimmed.strip_prefix("```"))
                .and_then(|s| s.strip_suffix("```"))
                .unwrap_or(trimmed)
                .trim();
            serde_json::from_str(json_str)
        })
        .map_err(|e| {
            format!(
                "Failed to parse LLM classification for field '{}': {} (raw: {})",
                field_name, e, text
            )
        })?;

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

/// Infer classification for a new canonical field.
/// Returns an error if classification cannot be determined — no silent fallbacks.
///
/// ```text
/// caller-provided? ──yes──▶ use it
///       │ no
///       ▼
/// LLM call (ANTHROPIC_API_KEY) ──success──▶ use it
///       │ no key / failure
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

    #[tokio::test]
    async fn infer_uses_caller_provided_first() {
        let caller = DataClassification::new(4, "medical").unwrap();
        let result = infer_classification("diagnosis", "patient diagnosis", Some(&caller)).await;
        let c = result.unwrap();
        assert_eq!(c.sensitivity_level, 4);
        assert_eq!(c.data_domain, "medical");
    }

    #[tokio::test]
    async fn infer_without_caller_uses_llm_or_errors() {
        let result = infer_classification("salary", "employee annual salary", None).await;
        match result {
            Ok(c) => {
                assert!(c.sensitivity_level <= 4);
                assert!(!c.data_domain.is_empty());
            }
            Err(e) => {
                assert!(e.contains("ANTHROPIC_API_KEY"), "got: {}", e);
            }
        }
    }
}
