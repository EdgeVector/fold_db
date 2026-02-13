use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;
use std::collections::HashSet;

use super::types::{IndexEntry, IndexResult, INDEX_ENTRY_PREFIX};
use super::NativeIndexManager;

/// Maximum number of entries returned from a single prefix scan.
/// Prevents unbounded memory growth on large indexes.
const SCAN_LIMIT: usize = 500;

impl NativeIndexManager {
    pub(super) fn normalize_search_term(term: &str) -> Option<String> {
        let lowered = term.trim().to_lowercase();
        if lowered.len() < 2 {
            return None;
        }
        Some(lowered)
    }

    /// Search all indexed keywords and return results
    pub async fn search_all_classifications(
        &self,
        term: &str,
    ) -> Result<Vec<IndexResult>, SchemaError> {
        log::debug!(
            "Native Index: search_all_classifications called for term: '{}'",
            term
        );

        let entries = self.search_all(term).await?;
        let results = self.entries_to_results(entries);

        log::info!(
            "Native Index: search_all_classifications for '{}' returned {} total results",
            term,
            results.len()
        );
        Ok(results)
    }

    /// Search for index entries matching a term.
    ///
    /// For multi-word queries like "alice johnson", tries the full phrase first
    /// (direct index match), then falls back to intersecting individual word results.
    pub async fn search(&self, term: &str) -> Result<Vec<IndexEntry>, SchemaError> {
        let Some(normalized) = Self::normalize_search_term(term) else {
            return Ok(Vec::new());
        };

        // Blind the full term and try as-is
        let blinded = self.blind_token(&normalized);
        let prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, blinded);
        let entries = self.scan_index_prefix(&prefix, Some(&normalized)).await?;
        if !entries.is_empty() || !normalized.contains(' ') {
            return Ok(entries);
        }

        // Multi-word with no direct match — intersect individual words
        let words: Vec<String> = term
            .split_whitespace()
            .filter_map(Self::normalize_search_term)
            .collect();

        if words.len() < 2 {
            return Ok(Vec::new());
        }

        // Fire all word scans concurrently
        let futures: Vec<_> = words
            .iter()
            .map(|word| {
                let blinded = self.blind_token(word);
                let prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, blinded);
                let word_clone = word.clone();
                async move { (word_clone, self.scan_index_prefix(&prefix, None).await) }
            })
            .collect();

        let scan_results = futures::future::join_all(futures).await;

        // First result set provides the candidates; remaining sets narrow via intersection
        let mut iter = scan_results.into_iter();
        let (_, first_result) = iter.next().unwrap();
        let candidates = first_result?;

        let mut required_keys: HashSet<(String, KeyValue)> = candidates
            .iter()
            .map(|e| (e.schema.clone(), e.key.clone()))
            .collect();

        for (_, result) in iter {
            let entries = result?;
            let keys: HashSet<(String, KeyValue)> = entries
                .into_iter()
                .map(|e| (e.schema.clone(), e.key.clone()))
                .collect();
            required_keys = required_keys.intersection(&keys).cloned().collect();
        }

        let mut seen = HashSet::new();
        let results: Vec<IndexEntry> = candidates
            .into_iter()
            .filter(|e| {
                let rk = (e.schema.clone(), e.key.clone());
                required_keys.contains(&rk) && seen.insert(rk)
            })
            .map(|mut e| {
                e.matched_term = Some(normalized.clone());
                e
            })
            .collect();

        Ok(results)
    }

    /// Search all indexed keywords and field names.
    /// Supports multi-word queries (phrase match first, then word intersection).
    pub async fn search_all(&self, term: &str) -> Result<Vec<IndexEntry>, SchemaError> {
        // Use search which handles multi-word intersection
        let (word_result, field_result) = tokio::join!(
            self.search(term),
            self.search_field_names(term)
        );

        let mut all_entries = Vec::new();
        let mut seen: HashSet<(String, KeyValue, String)> = HashSet::new();

        let entries = word_result?;
        for entry in entries {
            let dedup_key = (entry.schema.clone(), entry.key.clone(), entry.field.clone());
            if seen.insert(dedup_key) {
                all_entries.push(entry);
            }
        }

        let field_entries = field_result?;
        for entry in field_entries {
            let dedup_key = (entry.schema.clone(), entry.key.clone(), entry.field.clone());
            if seen.insert(dedup_key) {
                all_entries.push(entry);
            }
        }

        Ok(all_entries)
    }

    /// Search for field names in the index
    async fn search_field_names(
        &self,
        term: &str,
    ) -> Result<Vec<IndexEntry>, SchemaError> {
        let Some(normalized) = Self::normalize_search_term(term) else {
            return Ok(Vec::new());
        };

        let blinded = self.blind_token(&normalized);
        let prefix = format!("{}field:{}:", INDEX_ENTRY_PREFIX, blinded);
        self.scan_index_prefix(&prefix, Some(&normalized)).await
    }

    /// Scan index entries by prefix, setting `matched_term` on each result.
    /// Results are capped at `SCAN_LIMIT` to prevent unbounded memory growth.
    async fn scan_index_prefix(&self, prefix: &str, matched_term: Option<&str>) -> Result<Vec<IndexEntry>, SchemaError> {
        let raw = self
            .store
            .scan_prefix(prefix.as_bytes())
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to scan prefix: {}", e)))?;

        let total = raw.len();
        let mut entries = Vec::with_capacity(total.min(SCAN_LIMIT));
        for (_key, value) in raw.into_iter().take(SCAN_LIMIT) {
            match serde_json::from_slice::<IndexEntry>(&value) {
                Ok(mut entry) => {
                    entry.matched_term = matched_term.map(String::from);
                    entries.push(entry);
                }
                Err(e) => {
                    log::warn!("Failed to deserialize IndexEntry: {}", e);
                }
            }
        }

        if total > SCAN_LIMIT {
            log::warn!(
                "scan_index_prefix: truncated {} → {} entries for prefix '{}'",
                total, SCAN_LIMIT, prefix
            );
        }

        Ok(entries)
    }
}
