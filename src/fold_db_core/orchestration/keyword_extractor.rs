//! LLM-powered keyword extraction for native indexing.
//!
//! Uses the existing IngestionService (OpenRouter/Ollama) to extract
//! semantically meaningful search keywords from record data.

use std::collections::HashMap;
use std::sync::Arc;

use log::info;
use serde_json::Value;

use crate::error::FoldDbError;
use crate::ingestion::ai_helpers::extract_json_from_response;
use crate::ingestion::ingestion_service::IngestionService;

/// Extracts search keywords from record data via LLM.
pub struct KeywordExtractor {
    ingestion_service: Arc<IngestionService>,
}

impl KeywordExtractor {
    pub fn new(ingestion_service: Arc<IngestionService>) -> Self {
        Self { ingestion_service }
    }

    /// Extracts keywords per field via one LLM call.
    /// Returns a map of field_name → normalized keywords.
    pub async fn extract_keywords_per_field(
        &self,
        fields: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Vec<String>>, FoldDbError> {
        if fields.is_empty() {
            return Ok(HashMap::new());
        }

        let prompt = self.build_prompt(fields);

        let response = self
            .ingestion_service
            .call_ai_raw(&prompt)
            .await
            .map_err(|e| FoldDbError::Other(format!("LLM keyword extraction failed: {}", e)))?;

        let json_str = extract_json_from_response(&response)
            .map_err(|e| FoldDbError::Other(format!("Failed to parse LLM response: {}", e)))?;

        let per_field: HashMap<String, Vec<String>> = serde_json::from_str(&json_str)
            .map_err(|e| FoldDbError::Other(format!("LLM response not a field→keywords map: {}", e)))?;

        // Normalize each field's keywords
        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        for (field_name, keywords) in per_field {
            // Only keep keywords for fields that were actually in the input
            if !fields.contains_key(&field_name) {
                continue;
            }
            let normalized = Self::normalize_keywords(keywords);
            if !normalized.is_empty() {
                result.insert(field_name, normalized);
            }
        }

        let total_kw: usize = result.values().map(|v| v.len()).sum();
        info!(
            "KeywordExtractor: Extracted {} keywords across {} fields",
            total_kw,
            result.len()
        );

        Ok(result)
    }

    /// Normalize a list of keywords: lowercase, deduplicate, split multi-word.
    fn normalize_keywords(keywords: Vec<String>) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut normalized: Vec<String> = Vec::new();
        for raw in keywords {
            let kw = raw.to_lowercase().trim().to_string();
            if kw.len() >= 2 && seen.insert(kw.clone()) {
                normalized.push(kw.clone());
                // Split multi-word keywords into parts
                if kw.contains(' ') {
                    for part in kw.split_whitespace() {
                        let part = part.to_string();
                        if part.len() >= 2 && seen.insert(part.clone()) {
                            normalized.push(part);
                        }
                    }
                }
            }
        }
        normalized
    }

    /// Visible for testing — returns the prompt that would be sent to the LLM.
    pub fn build_prompt_for_test(&self, fields: &HashMap<String, Value>) -> String {
        self.build_prompt(fields)
    }

    fn build_prompt(&self, fields: &HashMap<String, Value>) -> String {
        let mut data_section = String::new();
        for (field_name, value) in fields {
            let value_str = match value {
                Value::String(s) => s.clone(),
                Value::Array(arr) => serde_json::to_string(arr).unwrap_or_default(),
                other => other.to_string(),
            };
            data_section.push_str(&format!("{}: {}\n", field_name, value_str));
        }

        let field_names: Vec<&String> = fields.keys().collect();
        format!(
            "Extract search keywords from this data, grouped by field.\n\
             Return a JSON object mapping each field name to an array of lowercase keyword strings.\n\
             Include: important words, normalized dates (YYYY-MM-DD), numbers,\n\
             named entities, and key phrases. Exclude stopwords and trivial terms.\n\n\
             Data:\n{}\n\
             Return ONLY a JSON object with these keys: {:?}\n\
             Example: {{\"field1\": [\"keyword1\", \"keyword2\"], \"field2\": [\"keyword3\"]}}",
            data_section, field_names
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Run with: cargo test test_extract_keywords_per_field_live -- --ignored --nocapture
    /// Requires FOLD_OPENROUTER_API_KEY environment variable.
    #[tokio::test]
    #[ignore]
    async fn test_extract_keywords_per_field_live() {
        let service = IngestionService::from_env()
            .expect("IngestionService::from_env() failed — is FOLD_OPENROUTER_API_KEY set?");
        let extractor = KeywordExtractor::new(Arc::new(service));

        let fields = HashMap::from([
            (
                "content".to_string(),
                json!("Rust is a systems programming language focused on safety and performance"),
            ),
            (
                "author".to_string(),
                json!("Alice Johnson"),
            ),
            (
                "published_date".to_string(),
                json!("2024-03-15"),
            ),
        ]);

        println!("\n=== Prompt ===\n{}", extractor.build_prompt_for_test(&fields));

        let result = extractor.extract_keywords_per_field(&fields).await
            .expect("extract_keywords_per_field failed");

        println!("\n=== Results ===");
        for (field, keywords) in &result {
            println!("  {}: {:?}", field, keywords);
        }

        // Every input field should have at least one keyword
        assert!(!result.is_empty(), "Should extract at least some keywords");
        for field_name in fields.keys() {
            if let Some(kws) = result.get(field_name) {
                assert!(!kws.is_empty(), "Field '{}' should have keywords", field_name);
            }
        }
    }

    /// Run with: cargo test test_extract_keywords_tweet_sample -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn test_extract_keywords_tweet_sample() {
        let service = IngestionService::from_env()
            .expect("IngestionService::from_env() failed — is FOLD_OPENROUTER_API_KEY set?");
        let extractor = KeywordExtractor::new(Arc::new(service));

        let fields = HashMap::from([
            (
                "text".to_string(),
                json!("Just shipped v2.0 of our app! New features include dark mode, offline sync, and end-to-end encryption. Thanks to the team @acme_eng 🚀"),
            ),
            (
                "username".to_string(),
                json!("@devlead42"),
            ),
            (
                "created_at".to_string(),
                json!("2024-01-20T14:30:00Z"),
            ),
        ]);

        println!("\n=== Prompt ===\n{}", extractor.build_prompt_for_test(&fields));

        let result = extractor.extract_keywords_per_field(&fields).await
            .expect("extract_keywords_per_field failed");

        println!("\n=== Results ===");
        for (field, keywords) in &result {
            println!("  {}: {:?}", field, keywords);
        }

        assert!(!result.is_empty(), "Should extract keywords from tweet");
    }

    #[test]
    fn test_normalize_keywords_basic() {
        let input = vec![
            "Rust".to_string(),
            "alice johnson".to_string(),
            "a".to_string(),  // too short
            "rust".to_string(),  // duplicate
            "".to_string(),  // empty
        ];
        let result = KeywordExtractor::normalize_keywords(input);
        assert_eq!(result, vec!["rust", "alice johnson", "alice", "johnson"]);
    }
}
