use regex::Regex;
use std::sync::OnceLock;

use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;

use super::types::{IndexClassification, IndexEntry};
use super::NativeIndexManager;

/// Check if a keyword looks like an email address.
fn is_email(keyword: &str) -> bool {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let re = PATTERN.get_or_init(|| {
        Regex::new(r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$").unwrap()
    });
    re.is_match(keyword)
}

/// Check if a keyword is a normalized date (YYYY-MM-DD).
fn is_date(keyword: &str) -> bool {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let re = PATTERN.get_or_init(|| {
        Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap()
    });
    re.is_match(keyword)
}

impl NativeIndexManager {
    /// Deduplicate index entries by key and write them via the KvStore.
    ///
    /// DynamoDB batch_write_item doesn't allow duplicate keys in a single
    /// request, and the extractor may return duplicate keywords in one batch.
    async fn write_index_entries(&self, entries: Vec<(Vec<u8>, Vec<u8>)>) -> Result<(), SchemaError> {
        let mut seen_keys = std::collections::HashSet::new();
        let deduped: Vec<(Vec<u8>, Vec<u8>)> = entries
            .into_iter()
            .filter(|(key, _)| seen_keys.insert(key.clone()))
            .collect();

        self.store.batch_put(deduped).await.map_err(|e| {
            SchemaError::InvalidData(format!("Failed to batch write index entries: {}", e))
        })
    }

    /// Index a record using extracted keywords.
    ///
    /// Takes a flat list of keywords (already normalized) and writes
    /// index entries + reverse mappings for each keyword.
    ///
    /// When `field_classifications` is provided (from the schema's AI-assigned
    /// classifications), classification is determined once for the whole field.
    /// Otherwise falls back to per-keyword regex detection.
    pub async fn batch_index_from_keywords(
        &self,
        schema_name: &str,
        key_value: &KeyValue,
        field_name: &str,
        keywords: Vec<String>,
        molecule_versions: Option<&std::collections::HashSet<u64>>,
        field_classifications: Option<&[String]>,
    ) -> Result<(), SchemaError> {
        log::info!(
            "[NativeIndex] batch_index_from_keywords: {} keywords for field '{}' in schema '{}'",
            keywords.len(),
            field_name,
            schema_name
        );

        let mut index_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

        // Determine classification strategy: schema-driven (once per field) or regex fallback (per keyword)
        let schema_classification = field_classifications.map(|cls| {
            let (classification, prefix) = if cls.iter().any(|c| c == "email") {
                (IndexClassification::Email, "email")
            } else if cls.iter().any(|c| c == "date") {
                (IndexClassification::Date, "date")
            } else {
                (IndexClassification::Word, "word")
            };
            log::debug!(
                "[NativeIndex] field '{}': schema classification → {}",
                field_name,
                prefix
            );
            (classification, prefix)
        });

        if schema_classification.is_none() {
            log::debug!(
                "[NativeIndex] field '{}': no schema classifications, regex fallback",
                field_name
            );
        }

        for keyword in &keywords {
            let (classification, prefix) = match &schema_classification {
                Some((cls, pfx)) => (cls.clone(), *pfx),
                None => {
                    // Regex fallback per keyword
                    if is_email(keyword) {
                        (IndexClassification::Email, "email")
                    } else if is_date(keyword) {
                        (IndexClassification::Date, "date")
                    } else {
                        (IndexClassification::Word, "word")
                    }
                }
            };
            let mut entry = IndexEntry::new(
                schema_name.to_string(),
                key_value.clone(),
                field_name.to_string(),
                classification,
            );
            entry.molecule_versions = molecule_versions.cloned();

            // Blind the keyword (HMAC if E2E key present, passthrough otherwise)
            let blinded = self.blind_token(keyword);
            let term = format!("{}:{}", prefix, blinded);
            let storage_key = entry.storage_key(&term);
            let entry_bytes = serde_json::to_vec(&entry).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to serialize IndexEntry: {}", e))
            })?;

            index_entries.push((storage_key.into_bytes(), entry_bytes));
        }

        self.write_index_entries(index_entries).await?;

        log::info!("[NativeIndex] batch_index_from_keywords: Completed successfully");
        Ok(())
    }

    /// Index field names for a record.
    ///
    /// Writes `idx:field:{field_name}:...` entries so that `search_field_names()`
    /// can find records by their field names.
    pub async fn batch_index_field_names(
        &self,
        schema_name: &str,
        key_value: &KeyValue,
        field_names: &[String],
        molecule_versions: Option<&std::collections::HashSet<u64>>,
    ) -> Result<(), SchemaError> {
        let indexable: Vec<&String> = field_names
            .iter()
            .filter(|f| Self::should_index_field(f))
            .collect();

        if indexable.is_empty() {
            return Ok(());
        }

        log::info!(
            "[NativeIndex] batch_index_field_names: {} fields for schema '{}'",
            indexable.len(),
            schema_name
        );

        let mut index_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

        for field_name in indexable {
            let normalized = field_name.to_ascii_lowercase();
            let mut entry = IndexEntry::new(
                schema_name.to_string(),
                key_value.clone(),
                field_name.clone(),
                IndexClassification::Field,
            );
            entry.molecule_versions = molecule_versions.cloned();

            let blinded = self.blind_token(&normalized);
            let term = format!("field:{}", blinded);
            let storage_key = entry.storage_key(&term);
            let entry_bytes = serde_json::to_vec(&entry).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to serialize IndexEntry: {}", e))
            })?;

            index_entries.push((storage_key.into_bytes(), entry_bytes));
        }

        self.write_index_entries(index_entries).await?;

        log::info!("[NativeIndex] batch_index_field_names: Completed successfully");
        Ok(())
    }

    /// Flush pending writes to durable storage.
    pub async fn flush(&self) -> Result<(), SchemaError> {
        self.store
            .flush()
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Flush failed: {}", e)))
    }
}
