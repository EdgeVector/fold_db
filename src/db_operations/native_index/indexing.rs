use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;

use super::types::{BatchIndexOperation, IndexEntry};
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

    /// Batch index a record's fields by extracting terms and writing index entries.
    pub async fn batch_index(
        &self,
        index_operations: &[BatchIndexOperation],
    ) -> Result<(), SchemaError> {
        log::info!(
            "[NativeIndex] batch_index: Starting with {} operations",
            index_operations.len()
        );

        let mut index_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

        for (schema_name, field_name, key_value, value, _classifications) in index_operations {
            if !Self::should_index_field(field_name) {
                continue;
            }

            let words = self.extract_words(value);

            for word in &words {
                let entry = IndexEntry::new(
                    schema_name.clone(),
                    key_value.clone(),
                    field_name.clone(),
                    "word".to_string(),
                );

                let term = format!("word:{}", word);
                let storage_key = entry.storage_key(&term);
                let entry_bytes = serde_json::to_vec(&entry).map_err(|e| {
                    SchemaError::InvalidData(format!("Failed to serialize IndexEntry: {}", e))
                })?;

                index_entries.push((storage_key.into_bytes(), entry_bytes));
            }

            // Also index field name
            let field_entry = IndexEntry::new(
                schema_name.clone(),
                key_value.clone(),
                field_name.clone(),
                "field".to_string(),
            );
            let field_term = field_name.to_ascii_lowercase();
            let field_storage_key = field_entry.storage_key(&format!("field:{}", field_term));
            let field_entry_bytes = serde_json::to_vec(&field_entry).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to serialize field IndexEntry: {}", e))
            })?;
            index_entries.push((field_storage_key.into_bytes(), field_entry_bytes));
        }

        log::info!(
            "[NativeIndex] batch_index: Writing {} index entries",
            index_entries.len()
        );

        self.write_index_entries(index_entries).await?;

        log::info!("[NativeIndex] batch_index: Completed successfully");
        Ok(())
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
