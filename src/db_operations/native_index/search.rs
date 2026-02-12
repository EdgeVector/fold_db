use crate::db_operations::native_index_classification::ClassificationType;
use crate::schema::SchemaError;
use std::collections::HashSet;

use super::types::{IndexEntry, IndexResult, INDEX_ENTRY_PREFIX};
use super::NativeIndexManager;

impl NativeIndexManager {
    /// Search all indexed keywords and return results (async version)
    pub async fn search_all_classifications_async(
        &self,
        term: &str,
    ) -> Result<Vec<IndexResult>, SchemaError> {
        log::debug!(
            "Native Index: search_all_classifications_async called for term: '{}'",
            term
        );

        let entries = self.search_all(term).await?;
        let results = self.entries_to_results(entries);

        log::info!(
            "Native Index: search_all_classifications_async for '{}' returned {} total results",
            term,
            results.len()
        );
        Ok(results)
    }

    /// Search all indexed keywords and return results (sync version, Sled only)
    pub fn search_all_classifications(&self, term: &str) -> Result<Vec<IndexResult>, SchemaError> {
        log::debug!(
            "Native Index: search_all_classifications called for term: '{}'",
            term
        );

        // For Sled backend, use sync search
        if !self.is_async() {
            let entries = self.search_sync(term)?;
            let results = self.entries_to_results(entries);
            log::info!(
                "Native Index: search_all_classifications for '{}' returned {} total results",
                term,
                results.len()
            );
            return Ok(results);
        }

        // For async backends, create a new runtime
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            let entries = self.search_all(term).await?;
            let results = self.entries_to_results(entries);
            log::info!(
                "Native Index: search_all_classifications for '{}' returned {} total results",
                term,
                results.len()
            );
            Ok(results)
        })
    }

    /// Search for index entries matching a term.
    ///
    /// For multi-word queries like "alice johnson", tries the full phrase first
    /// (direct index match), then falls back to intersecting individual word results.
    pub async fn search(&self, term: &str) -> Result<Vec<IndexEntry>, SchemaError> {
        let Some(normalized) = self.normalize_search_term(term) else {
            return Ok(Vec::new());
        };

        // Try the full term as-is
        let prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, normalized);
        let entries = self.scan_index_prefix(&prefix).await?;
        if !entries.is_empty() || !normalized.contains(' ') {
            return Ok(entries);
        }

        // Multi-word with no direct match — intersect individual words
        let words: Vec<String> = term
            .split_whitespace()
            .filter_map(|w| self.normalize_search_term(w))
            .collect();

        if words.len() < 2 {
            return Ok(Vec::new());
        }

        // Search the first word, then filter to records that also match all other words
        let first_prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, words[0]);
        let candidates = self.scan_index_prefix(&first_prefix).await?;

        // Collect record keys that appear for every other word
        let mut required_keys: Option<HashSet<(String, crate::schema::types::key_value::KeyValue)>> = None;
        for word in &words[1..] {
            let p = format!("{}word:{}:", INDEX_ENTRY_PREFIX, word);
            let word_entries = self.scan_index_prefix(&p).await?;
            let keys: HashSet<(String, crate::schema::types::key_value::KeyValue)> = word_entries
                .into_iter()
                .map(|e| (e.schema.clone(), e.key.clone()))
                .collect();
            required_keys = Some(match required_keys {
                Some(existing) => existing.intersection(&keys).cloned().collect(),
                None => keys,
            });
        }

        let required_keys = required_keys.unwrap_or_default();
        let mut seen = HashSet::new();
        let results: Vec<IndexEntry> = candidates
            .into_iter()
            .filter(|e| {
                let rk = (e.schema.clone(), e.key.clone());
                required_keys.contains(&rk) && seen.insert(rk)
            })
            .collect();

        Ok(results)
    }

    /// Sync version of search (Sled only).
    pub fn search_sync(&self, term: &str) -> Result<Vec<IndexEntry>, SchemaError> {
        let Some(ref tree) = self.tree else {
            return Err(SchemaError::InvalidData(
                "Sync search only available with Sled backend".to_string(),
            ));
        };

        let Some(normalized) = self.normalize_search_term(term) else {
            return Ok(Vec::new());
        };

        let scan_sync = |prefix: &str| -> Vec<IndexEntry> {
            let mut entries = Vec::new();
            for result in tree.scan_prefix(prefix.as_bytes()) {
                match result {
                    Ok((_key, value)) => match serde_json::from_slice::<IndexEntry>(&value) {
                        Ok(entry) => entries.push(entry),
                        Err(e) => log::warn!("Failed to deserialize IndexEntry: {}", e),
                    },
                    Err(e) => log::warn!("Sled scan error: {}", e),
                }
            }
            entries
        };

        // Try the full term as-is
        let prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, normalized);
        let entries = scan_sync(&prefix);
        if !entries.is_empty() || !normalized.contains(' ') {
            return Ok(entries);
        }

        // Multi-word fallback — intersect individual words
        let words: Vec<String> = term
            .split_whitespace()
            .filter_map(|w| self.normalize_search_term(w))
            .collect();

        if words.len() < 2 {
            return Ok(Vec::new());
        }

        let first_prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, words[0]);
        let candidates = scan_sync(&first_prefix);

        let mut required_keys: Option<HashSet<(String, crate::schema::types::key_value::KeyValue)>> = None;
        for word in &words[1..] {
            let p = format!("{}word:{}:", INDEX_ENTRY_PREFIX, word);
            let word_entries = scan_sync(&p);
            let keys: HashSet<(String, crate::schema::types::key_value::KeyValue)> = word_entries
                .into_iter()
                .map(|e| (e.schema.clone(), e.key.clone()))
                .collect();
            required_keys = Some(match required_keys {
                Some(existing) => existing.intersection(&keys).cloned().collect(),
                None => keys,
            });
        }

        let required_keys = required_keys.unwrap_or_default();
        let mut seen = HashSet::new();
        let results: Vec<IndexEntry> = candidates
            .into_iter()
            .filter(|e| {
                let rk = (e.schema.clone(), e.key.clone());
                required_keys.contains(&rk) && seen.insert(rk)
            })
            .collect();

        Ok(results)
    }

    /// Search with classification using prefix scan
    pub async fn search_with_classification(
        &self,
        term: &str,
        classification: Option<ClassificationType>,
    ) -> Result<Vec<IndexEntry>, SchemaError> {
        let normalized = match classification {
            Some(ClassificationType::Word) | None => self.normalize_search_term(term),
            Some(_) => {
                let trimmed = term.trim().to_ascii_lowercase();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            }
        };

        let Some(normalized) = normalized else {
            return Ok(Vec::new());
        };

        let class_prefix = classification
            .map(|c| c.prefix())
            .unwrap_or_else(|| "word".to_string());

        // Build prefix: idx:{classification}:{normalized}:
        let prefix = format!("{}{}:{}:", INDEX_ENTRY_PREFIX, class_prefix, normalized);

        log::debug!(
            "[NativeIndex] search_with_classification: Searching with prefix '{}'",
            prefix
        );

        self.scan_index_prefix(&prefix).await
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
        let mut seen = HashSet::new();

        if let Ok(entries) = word_result {
            for entry in entries {
                let key = format!("{:?}:{:?}:{}", entry.schema, entry.key, entry.field);
                if seen.insert(key) {
                    all_entries.push(entry);
                }
            }
        }

        if let Ok(field_entries) = field_result {
            for entry in field_entries {
                let key = format!("{:?}:{:?}:{}", entry.schema, entry.key, entry.field);
                if seen.insert(key) {
                    all_entries.push(entry);
                }
            }
        }

        Ok(all_entries)
    }

    /// Search for field names in the index
    async fn search_field_names(
        &self,
        term: &str,
    ) -> Result<Vec<IndexEntry>, SchemaError> {
        let normalized = term.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        let prefix = format!("{}field:{}:", INDEX_ENTRY_PREFIX, normalized);
        self.scan_index_prefix(&prefix).await
    }

    /// Search for field names in the index (sync version, Sled only)
    pub fn search_field_names_sync(
        &self,
        term: &str,
    ) -> Result<Vec<IndexEntry>, SchemaError> {
        let Some(ref tree) = self.tree else {
            return Err(SchemaError::InvalidData(
                "Sync field name search only available with Sled backend.".to_string(),
            ));
        };

        let normalized = term.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        let prefix = format!("{}field:{}:", INDEX_ENTRY_PREFIX, normalized);

        log::debug!(
            "[NativeIndex] search_field_names_sync: Searching with prefix '{}'",
            prefix
        );

        let mut entries = Vec::new();
        for result in tree.scan_prefix(prefix.as_bytes()) {
            match result {
                Ok((_key, value)) => match serde_json::from_slice::<IndexEntry>(&value) {
                    Ok(entry) => entries.push(entry),
                    Err(e) => {
                        log::warn!("Failed to deserialize IndexEntry: {}", e);
                    }
                },
                Err(e) => {
                    log::warn!("Sled scan error: {}", e);
                }
            }
        }

        log::info!(
            "[NativeIndex] search_field_names_sync: Found {} field name entries for term '{}'",
            entries.len(),
            term
        );

        Ok(entries)
    }

    /// Scan index entries by prefix
    async fn scan_index_prefix(&self, prefix: &str) -> Result<Vec<IndexEntry>, SchemaError> {
        let results = if let Some(ref store) = self.store {
            store
                .scan_prefix(prefix.as_bytes())
                .await
                .map_err(|e| SchemaError::InvalidData(format!("Failed to scan prefix: {}", e)))?
        } else if let Some(ref tree) = self.tree {
            tree.scan_prefix(prefix.as_bytes())
                .filter_map(|r| r.ok())
                .map(|(k, v)| (k.to_vec(), v.to_vec()))
                .collect()
        } else {
            return Err(SchemaError::InvalidData(
                "NativeIndexManager not properly initialized".to_string(),
            ));
        };

        let mut entries = Vec::new();
        for (_key, value) in results {
            match serde_json::from_slice::<IndexEntry>(&value) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    log::warn!("Failed to deserialize IndexEntry: {}", e);
                }
            }
        }

        Ok(entries)
    }
}
