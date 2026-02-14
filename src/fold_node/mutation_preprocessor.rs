//! MutationPreprocessor — rules-based keyword extraction for ALL mutations.
//!
//! Enriches mutations with keyword `index_terms` before they reach the
//! MutationManager. Uses tokenization, stopword filtering, and stemming —
//! no LLM dependency.

use crate::fold_db_core::orchestration::keyword_extractor::extract_keywords_per_field;

/// Enriches mutations with keyword index_terms via rules-based extraction.
#[derive(Clone, Default)]
pub struct MutationPreprocessor;

impl MutationPreprocessor {
    pub fn new() -> Self {
        Self
    }

    /// Enrich mutations with keyword index_terms (deterministic, instant).
    /// Skips mutations that already have index_terms populated.
    pub fn preprocess(&self, mutations: &mut [crate::schema::types::operations::Mutation]) {
        for mutation in mutations.iter_mut() {
            if mutation.index_terms.is_some() {
                continue;
            }
            let keywords = extract_keywords_per_field(&mutation.fields_and_values);
            if !keywords.is_empty() {
                mutation.index_terms = Some(keywords);
            }
        }
    }
}
