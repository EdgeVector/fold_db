use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;
use crate::storage::traits::KvStore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::types::IndexResult;

pub(super) const EMB_PREFIX: &str = "emb:";

/// Entry stored in Sled for each indexed fragment.
#[derive(Serialize, Deserialize)]
pub(super) struct StoredEmbedding {
    pub schema: String,
    pub key: KeyValue,
    pub field_name: String,
    pub fragment_index: usize,
    pub embedding: Vec<f32>,
    // Legacy: populated only for old-format entries (pre per-field split)
    #[serde(default)]
    pub field_names: Option<Vec<String>>,
}

/// Entry held in the in-memory index.
pub(super) struct EmbeddingEntry {
    pub schema: String,
    pub key: KeyValue,
    pub field_name: String,
    pub fragment_index: usize,
    pub embedding: Vec<f32>,
}

impl EmbeddingEntry {
    /// Storage key for a per-field fragment.
    pub(super) fn fragment_storage_key(
        schema: &str,
        key: &KeyValue,
        field_name: &str,
        fragment_index: usize,
    ) -> String {
        let key_hash = key_hash_string(key);
        format!("{}{}:{}:{}:{}", EMB_PREFIX, schema, key_hash, field_name, fragment_index)
    }

    /// Unique record identifier for dedup (schema + key hash).
    fn record_id(&self) -> String {
        let key_hash = key_hash_string(&self.key);
        format!("{}:{}", self.schema, key_hash)
    }
}

fn key_hash_string(key: &KeyValue) -> String {
    match (&key.hash, &key.range) {
        (Some(h), Some(r)) => format!("{}_{}", h, r),
        (Some(h), None) => h.clone(),
        (None, Some(r)) => format!("_{}", r),
        (None, None) => "empty".to_string(),
    }
}

/// In-memory nearest-neighbour index backed by Sled for persistence.
pub(super) struct EmbeddingIndex {
    pub entries: std::sync::RwLock<Vec<EmbeddingEntry>>,
}

impl EmbeddingIndex {
    pub(super) fn new(entries: Vec<EmbeddingEntry>) -> Self {
        Self {
            entries: std::sync::RwLock::new(entries),
        }
    }

    /// Load all persisted embeddings from the KV store into memory.
    /// Handles both legacy (whole-record) and new (per-field) formats.
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
                Ok(stored) => {
                    if stored.field_names.is_some() && stored.field_name.is_empty() {
                        // Legacy format: whole-record embedding with field_names list.
                        // Convert to a single entry with field_name="*" for backward compat.
                        entries.push(EmbeddingEntry {
                            schema: stored.schema,
                            key: stored.key,
                            field_name: "*".to_string(),
                            fragment_index: 0,
                            embedding: stored.embedding,
                        });
                    } else {
                        entries.push(EmbeddingEntry {
                            schema: stored.schema,
                            key: stored.key,
                            field_name: stored.field_name,
                            fragment_index: stored.fragment_index,
                            embedding: stored.embedding,
                        });
                    }
                }
                Err(e) => log::warn!("Failed to deserialize StoredEmbedding: {}", e),
            }
        }

        log::info!("Loaded {} embeddings from store", entries.len());
        entries
    }

    /// Persist and upsert a single field fragment embedding.
    pub(super) async fn insert_fragment(
        &self,
        store: &dyn KvStore,
        schema: &str,
        key: &KeyValue,
        field_name: &str,
        fragment_index: usize,
        embedding: Vec<f32>,
    ) -> Result<(), SchemaError> {
        let stored = StoredEmbedding {
            schema: schema.to_string(),
            key: key.clone(),
            field_name: field_name.to_string(),
            fragment_index,
            embedding: embedding.clone(),
            field_names: None,
        };

        let storage_key =
            EmbeddingEntry::fragment_storage_key(schema, key, field_name, fragment_index);
        let bytes = serde_json::to_vec(&stored)
            .map_err(|e| SchemaError::InvalidData(format!("Serialization failed: {}", e)))?;

        store
            .put(storage_key.as_bytes(), bytes)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Storage write failed: {}", e)))?;

        let new_entry = EmbeddingEntry {
            schema: schema.to_string(),
            key: key.clone(),
            field_name: field_name.to_string(),
            fragment_index,
            embedding,
        };

        let mut entries = self.entries.write().unwrap();
        if let Some(existing) = entries.iter_mut().find(|e| {
            EmbeddingEntry::fragment_storage_key(&e.schema, &e.key, &e.field_name, e.fragment_index)
                == storage_key
        }) {
            *existing = new_entry;
        } else {
            entries.push(new_entry);
        }

        Ok(())
    }

    /// Cosine similarity search with per-record deduplication.
    ///
    /// Multiple fragments of the same record may match. We keep only the
    /// highest-scoring fragment per record and return it as the representative.
    pub(super) fn search(&self, query_vec: &[f32], k: usize) -> Vec<IndexResult> {
        let entries = self.entries.read().unwrap();

        // Score all entries
        let mut scored: Vec<(f32, usize)> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (cosine_similarity(query_vec, &e.embedding), i))
            .collect();

        scored.sort_unstable_by(|a, b| {
            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Dedup by record key, keeping highest score per record
        let mut seen: HashMap<String, f32> = HashMap::new();
        let mut results: Vec<IndexResult> = Vec::new();

        for (score, i) in scored {
            if results.len() >= k {
                break;
            }
            let e = &entries[i];
            let record_id = e.record_id();

            // Skip if we already have a higher-scoring fragment for this record
            if let Some(&existing_score) = seen.get(&record_id) {
                if existing_score >= score {
                    continue;
                }
            }
            seen.insert(record_id, score);

            results.push(IndexResult {
                schema_name: e.schema.clone(),
                field: e.field_name.clone(),
                key_value: e.key.clone(),
                value: Value::Null,
                metadata: Some(serde_json::json!({"score": score, "match_type": "semantic"})),
                molecule_versions: None,
            });
        }

        results
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
