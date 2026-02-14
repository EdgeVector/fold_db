//! MutationPreprocessor — client-side keyword extraction for ALL mutations.
//!
//! Enriches mutations with keyword `index_terms` before they reach the
//! MutationManager. In local FoldDB both preprocessor and storage run in
//! the same process; in exemem the preprocessor runs on the client.

use crate::fold_db_core::orchestration::keyword_extractor::normalize_keywords;
use crate::ingestion::ingestion_service::IngestionService;
use crate::log_feature;
use crate::logging::features::LogFeature;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Enriches mutations with keyword index_terms via LLM extraction.
#[derive(Clone)]
pub struct MutationPreprocessor {
    ingestion_service: Arc<IngestionService>,
}

impl MutationPreprocessor {
    pub fn new(ingestion_service: Arc<IngestionService>) -> Self {
        Self { ingestion_service }
    }

    /// Try to create a preprocessor from environment configuration.
    /// Returns `None` if no LLM API key is configured.
    pub fn from_env() -> Option<Self> {
        match IngestionService::from_env() {
            Ok(service) => Some(Self::new(Arc::new(service))),
            Err(_) => {
                log_feature!(
                    LogFeature::Database,
                    info,
                    "MutationPreprocessor: No LLM API key configured — keyword extraction disabled"
                );
                None
            }
        }
    }

    /// Enrich mutations with keyword index_terms (best-effort).
    /// Skips mutations that already have index_terms populated.
    pub async fn preprocess(
        &self,
        mutations: &mut [crate::schema::types::operations::Mutation],
    ) {
        for mutation in mutations.iter_mut() {
            if mutation.index_terms.is_some() {
                continue;
            }
            mutation.index_terms = self.extract_keywords(&mutation.fields_and_values).await;
        }
    }

    /// Extract keywords from field values via LLM call.
    async fn extract_keywords(
        &self,
        fields: &HashMap<String, Value>,
    ) -> Option<HashMap<String, Vec<String>>> {
        let prompt = build_keyword_prompt(fields);

        let raw_response = match self.ingestion_service.call_ai_raw(&prompt).await {
            Ok(resp) => resp,
            Err(e) => {
                log_feature!(
                    LogFeature::Ingestion,
                    warn,
                    "MutationPreprocessor: keyword extraction failed (LLM call): {}",
                    e
                );
                return None;
            }
        };

        let json_str =
            match crate::ingestion::ai_helpers::extract_json_from_response(&raw_response) {
                Ok(s) => s,
                Err(e) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        warn,
                        "MutationPreprocessor: keyword extraction failed (parse): {}",
                        e
                    );
                    return None;
                }
            };

        let per_field: HashMap<String, Vec<String>> = match serde_json::from_str(&json_str) {
            Ok(m) => m,
            Err(e) => {
                log_feature!(
                    LogFeature::Ingestion,
                    warn,
                    "MutationPreprocessor: keyword extraction failed (deserialize): {}",
                    e
                );
                return None;
            }
        };

        // Filter to only fields present in input, then normalize
        let filtered: HashMap<String, Vec<String>> = per_field
            .into_iter()
            .filter(|(k, _)| fields.contains_key(k))
            .collect();

        let normalized = normalize_all_keywords(filtered);

        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    }
}

/// Build a keyword extraction prompt for a set of fields.
fn build_keyword_prompt(fields: &HashMap<String, Value>) -> String {
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

/// Normalize all keywords in an index_terms map.
fn normalize_all_keywords(terms: HashMap<String, Vec<String>>) -> HashMap<String, Vec<String>> {
    terms
        .into_iter()
        .map(|(field, kws)| (field, normalize_keywords(kws)))
        .collect()
}
