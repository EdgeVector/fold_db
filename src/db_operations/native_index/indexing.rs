use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;

use super::types::IndexEntry;
use super::NativeIndexManager;

impl NativeIndexManager {
    /// Deduplicate index entries by key and write them via the KvStore.
    ///
    /// DynamoDB batch_write_item doesn't allow duplicate keys, and entries
    /// created within the same millisecond can collide.
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
        keywords: Vec<String>,
    ) -> Result<(), SchemaError> {
        log::info!(
            "[NativeIndex] batch_index_from_keywords: {} keywords for schema '{}'",
            keywords.len(),
            schema_name
        );

        let mut index_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

        for keyword in &keywords {
            let entry = IndexEntry::new(
                schema_name.to_string(),
                key_value.clone(),
                "llm_keyword".to_string(),
                "word".to_string(),
            );

            // Term is stored as "word:{keyword}" to match the search prefix format
            let term = format!("word:{}", keyword);
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

    /// Flush pending writes to durable storage.
    pub async fn flush(&self) -> Result<(), SchemaError> {
        self.store
            .flush()
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Flush failed: {}", e)))
    }
}
