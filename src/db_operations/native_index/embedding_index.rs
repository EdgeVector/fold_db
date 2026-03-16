use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;
use crate::storage::traits::KvStore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::types::IndexResult;

pub(super) const EMB_PREFIX: &str = "emb:";

/// Entry stored in Sled for each indexed record.
#[derive(Serialize, Deserialize)]
pub(super) struct StoredEmbedding {
    pub schema: String,
    pub key: KeyValue,
    pub field_names: Vec<String>,
    pub embedding: Vec<f32>,
}

/// Entry held in the in-memory index.
pub(super) struct EmbeddingEntry {
    pub schema: String,
    pub key: KeyValue,
    pub field_names: Vec<String>,
    pub embedding: Vec<f32>,
}

impl EmbeddingEntry {
    pub(super) fn storage_key(schema: &str, key: &KeyValue) -> String {
        let key_hash = match (&key.hash, &key.range) {
            (Some(h), Some(r)) => format!("{}_{}", h, r),
            (Some(h), None) => h.clone(),
            (None, Some(r)) => format!("_{}", r),
            (None, None) => "empty".to_string(),
        };
        format!("{}{}:{}", EMB_PREFIX, schema, key_hash)
    }
}

/// In-memory nearest-neighbour index backed by Sled for persistence.
pub(super) struct EmbeddingIndex {
    pub entries: std::sync::RwLock<Vec<EmbeddingEntry>>,
}

impl EmbeddingIndex {
    pub(super) fn new(entries: Vec<EmbeddingEntry>) -> Self {
        Self { entries: std::sync::RwLock::new(entries) }
    }

    /// Load all persisted embeddings from the KV store into memory.
    pub(super) async fn load_from_store(store: &dyn KvStore) -> Vec<EmbeddingEntry> {
        let raw = match store.scan_prefix(EMB_PREFIX.as_bytes()).await {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Failed to scan embedding index from store: {}", e);
                return Vec::new();
            }
        };

        let mut entries = Vec::with_capacity(raw.len());
        for (_key, value) in raw {
            match serde_json::from_slice::<StoredEmbedding>(&value) {
                Ok(stored) => entries.push(EmbeddingEntry {
                    schema: stored.schema,
                    key: stored.key,
                    field_names: stored.field_names,
                    embedding: stored.embedding,
                }),
                Err(e) => log::warn!("Failed to deserialize StoredEmbedding: {}", e),
            }
        }

        log::info!("Loaded {} embeddings from store", entries.len());
        entries
    }

    /// Persist and upsert a record embedding.
    pub(super) async fn insert(
        &self,
        store: &dyn KvStore,
        schema: &str,
        key: &KeyValue,
        field_names: Vec<String>,
        embedding: Vec<f32>,
    ) -> Result<(), SchemaError> {
        let stored = StoredEmbedding {
            schema: schema.to_string(),
            key: key.clone(),
            field_names: field_names.clone(),
            embedding: embedding.clone(),
        };

        let storage_key = EmbeddingEntry::storage_key(schema, key);
        let bytes = serde_json::to_vec(&stored)
            .map_err(|e| SchemaError::InvalidData(format!("Serialization failed: {}", e)))?;

        store
            .put(storage_key.as_bytes(), bytes)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Storage write failed: {}", e)))?;

        let new_entry = EmbeddingEntry {
            schema: schema.to_string(),
            key: key.clone(),
            field_names,
            embedding,
        };

        let mut entries = self.entries.write().unwrap();
        if let Some(existing) = entries
            .iter_mut()
            .find(|e| EmbeddingEntry::storage_key(&e.schema, &e.key) == storage_key)
        {
            *existing = new_entry;
        } else {
            entries.push(new_entry);
        }

        Ok(())
    }

    /// Brute-force cosine similarity search. Returns up to `k` results sorted by score.
    pub(super) fn search(&self, query_vec: &[f32], k: usize) -> Vec<IndexResult> {
        let entries = self.entries.read().unwrap();

        let mut scored: Vec<(f32, usize)> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (cosine_similarity(query_vec, &e.embedding), i))
            .collect();

        scored.sort_unstable_by(|a, b| {
            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
        });

        scored
            .into_iter()
            .take(k)
            .flat_map(|(score, i)| {
                let e = &entries[i];
                e.field_names.iter().map(move |field| IndexResult {
                    schema_name: e.schema.clone(),
                    field: field.clone(),
                    key_value: e.key.clone(),
                    value: Value::Null,
                    metadata: Some(serde_json::json!({"score": score, "match_type": "semantic"})),
                    molecule_versions: None,
                })
            })
            .collect()
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

/// Convert a map of field values to a single text string for embedding.
pub(super) fn fields_to_text(fields: &HashMap<String, Value>) -> String {
    fields.values().map(value_to_text).collect::<Vec<_>>().join(" ")
}

fn value_to_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => arr.iter().map(value_to_text).collect::<Vec<_>>().join(" "),
        Value::Object(obj) => obj.values().map(value_to_text).collect::<Vec<_>>().join(" "),
        Value::Null => String::new(),
    }
}
