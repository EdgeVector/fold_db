use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;

use super::types::IndexEntry;
use super::NativeIndexManager;

impl NativeIndexManager {
    /// Deduplicate index entries by key and write them via the KvStore.
    ///
    /// DynamoDB batch_write_item doesn't allow duplicate keys in a single
    /// request, and the LLM may return duplicate keywords in one batch.
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

    /// Index a record using LLM-extracted keywords.
    ///
    /// Takes a flat list of keywords (already normalized by the LLM) and writes
    /// index entries + reverse mappings for each keyword.
    pub async fn batch_index_from_keywords(
        &self,
        schema_name: &str,
        key_value: &KeyValue,
        field_name: &str,
        keywords: Vec<String>,
        molecule_versions: Option<&Vec<u64>>,
    ) -> Result<(), SchemaError> {
        log::info!(
            "[NativeIndex] batch_index_from_keywords: {} keywords for field '{}' in schema '{}'",
            keywords.len(),
            field_name,
            schema_name
        );

        let mut index_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

        for keyword in &keywords {
            let mut entry = IndexEntry::new(
                schema_name.to_string(),
                key_value.clone(),
                field_name.to_string(),
                "word".to_string(),
            );
            entry.molecule_versions = molecule_versions.cloned();

            // Blind the keyword (HMAC if E2E key present, passthrough otherwise)
            let blinded = self.blind_token(keyword);
            let term = format!("word:{}", blinded);
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

    /// Index field names for a record (no LLM needed).
    ///
    /// Writes `idx:field:{field_name}:...` entries so that `search_field_names()`
    /// can find records by their field names.
    pub async fn batch_index_field_names(
        &self,
        schema_name: &str,
        key_value: &KeyValue,
        field_names: &[String],
        molecule_versions: Option<&Vec<u64>>,
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
                "field".to_string(),
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
