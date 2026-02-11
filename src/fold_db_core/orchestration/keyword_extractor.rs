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

    /// Extracts keywords from all fields of a record via one LLM call.
    /// Returns a flat Vec<String> of normalized keywords.
    pub async fn extract_keywords(
        &self,
        fields: &HashMap<String, Value>,
    ) -> Result<Vec<String>, FoldDbError> {
        if fields.is_empty() {
            return Ok(Vec::new());
        }

        let prompt = self.build_prompt(fields);

        let response = self
            .ingestion_service
            .call_ai_raw(&prompt)
            .await
            .map_err(|e| FoldDbError::Other(format!("LLM keyword extraction failed: {}", e)))?;

        let json_str = extract_json_from_response(&response)
            .map_err(|e| FoldDbError::Other(format!("Failed to parse LLM response: {}", e)))?;

        let keywords: Vec<String> = serde_json::from_str(&json_str)
            .map_err(|e| FoldDbError::Other(format!("LLM response not a string array: {}", e)))?;

        // Normalize: lowercase, deduplicate, filter empty/short.
        // For multi-word keywords, also index each individual word
        // so "alice johnson" is searchable as "alice johnson", "alice", and "johnson".
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

        info!(
            "KeywordExtractor: Extracted {} keywords from {} fields",
            normalized.len(),
            fields.len()
        );

        Ok(normalized)
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

        format!(
            "Extract search keywords from this data. Return a JSON array of lowercase strings.\n\
             Include: important words, normalized dates (YYYY-MM-DD), numbers,\n\
             named entities, and key phrases. Exclude stopwords and trivial terms.\n\n\
             Data:\n{}\n\
             Return ONLY a JSON array like: [\"keyword1\", \"keyword2\", ...]",
            data_section
        )
    }
}
