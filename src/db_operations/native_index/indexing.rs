use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;

use super::types::{BatchIndexOperation, IndexEntry};
use super::NativeIndexManager;

impl NativeIndexManager {
    /// Batch index a record's fields by extracting terms and writing index entries.
    pub async fn batch_index(
        &self,
        index_operations: &[BatchIndexOperation],
    ) -> Result<(), SchemaError> {
        log::info!(
            "[NativeIndex] batch_index: Starting with {} operations",
            index_operations.len()
        );

        if self.tree.is_none() && self.store.is_none() {
            return Err(SchemaError::InvalidData(
                "NativeIndexManager not properly initialized".to_string(),
            ));
        }

        let mut index_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

        for (schema_name, field_name, key_value, value, classifications) in index_operations {
            if !Self::should_index_field(field_name) {
                continue;
            }

            let classifications = classifications.clone().unwrap_or_default();
            let effective_classifications = if classifications.is_empty() {
                vec!["word".to_string()]
            } else {
                classifications
            };

            // Extract terms and create index entries
            let terms_with_classification =
                self.extract_terms(&effective_classifications, value);

            for (term, classification) in &terms_with_classification {
                let entry = IndexEntry::new(
                    schema_name.clone(),
                    key_value.clone(),
                    field_name.clone(),
                    classification.clone(),
                );

                let storage_key = entry.storage_key(term);
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

        // Write all entries using batch operations
        if let Some(ref store) = self.store {
            // Deduplicate by key - DynamoDB batch_write_item doesn't allow duplicate keys
            // This can happen when entries are created within the same millisecond
            let mut seen_keys = std::collections::HashSet::new();
            let deduped_entries: Vec<(Vec<u8>, Vec<u8>)> = index_entries
                .into_iter()
                .filter(|(key, _)| seen_keys.insert(key.clone()))
                .collect();

            log::info!(
                "[NativeIndex] batch_index: After dedup: {} entries",
                deduped_entries.len()
            );

            store.batch_put(deduped_entries).await.map_err(|e| {
                SchemaError::InvalidData(format!("Failed to batch write index entries: {}", e))
            })?;
        } else if let Some(ref tree) = self.tree {
            let mut batch = sled::Batch::default();
            for (key, value) in index_entries {
                batch.insert(key, value);
            }
            tree.apply_batch(batch)
                .map_err(|e| SchemaError::InvalidData(format!("Batch apply failed: {}", e)))?;
        }

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

        if self.tree.is_none() && self.store.is_none() {
            return Err(SchemaError::InvalidData(
                "NativeIndexManager not properly initialized".to_string(),
            ));
        }

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

        // Write all entries
        if let Some(ref store) = self.store {
            let mut seen_keys = std::collections::HashSet::new();
            let deduped_entries: Vec<(Vec<u8>, Vec<u8>)> = index_entries
                .into_iter()
                .filter(|(key, _)| seen_keys.insert(key.clone()))
                .collect();

            store.batch_put(deduped_entries).await.map_err(|e| {
                SchemaError::InvalidData(format!("Failed to batch write keyword entries: {}", e))
            })?;
        } else if let Some(ref tree) = self.tree {
            let mut batch = sled::Batch::default();
            for (key, value) in index_entries {
                batch.insert(key, value);
            }
            tree.apply_batch(batch)
                .map_err(|e| SchemaError::InvalidData(format!("Batch apply failed: {}", e)))?;
        }

        log::info!("[NativeIndex] batch_index_from_keywords: Completed successfully");
        Ok(())
    }

    /// Explicitly flush the index tree to disk
    ///
    /// This should only be called for non-batch operations.
    /// Batch operations handle flushing internally.
    pub fn flush(&self) -> Result<(), SchemaError> {
        if let Some(ref tree) = self.tree {
            tree.flush()
                .map_err(|e| SchemaError::InvalidData(format!("Flush failed: {}", e)))?;
        }
        Ok(())
    }
}
