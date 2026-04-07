use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;
use crate::storage::traits::KvStore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::types::IndexResult;

pub(super) const EMB_PREFIX: &str = "emb:";

/// Entry stored in Sled for each indexed fragment.
/// Backward-compatible: old entries missing new fields will deserialize with defaults.
#[derive(Serialize, Deserialize)]
pub(super) struct StoredEmbedding {
    pub schema: String,
    pub key: KeyValue,
    /// Per-fragment: the specific field this fragment belongs to.
    #[serde(default)]
    pub field_name: String,
    /// Per-fragment: index within the field's fragment list.
    #[serde(default)]
    pub fragment_idx: usize,
    /// The original fragment text (needed for anonymity gate during discovery publish).
    #[serde(default)]
    pub fragment_text: Option<String>,
    pub embedding: Vec<f32>,
    /// Legacy: old format stored all field names here. Kept for deserialization compat.
    #[serde(default)]
    pub field_names: Vec<String>,
}

/// Entry held in the in-memory index.
#[allow(dead_code)] // fragment_text read by discovery publisher in Phase 5
pub(super) struct EmbeddingEntry {
    pub schema: String,
    pub key: KeyValue,
    pub field_name: String,
    pub fragment_idx: usize,
    pub fragment_text: Option<String>,
    pub embedding: Vec<f32>,
    /// Legacy field names from old-format entries (for backward-compat search expansion).
    pub legacy_field_names: Vec<String>,
}

impl EmbeddingEntry {
    /// New per-fragment storage key: emb:{schema}:{key_hash}:{field}:{fragment_idx}
    pub(super) fn fragment_storage_key(
        schema: &str,
        key: &KeyValue,
        field_name: &str,
        fragment_idx: usize,
    ) -> String {
        let key_hash = Self::key_hash(key);
        format!(
            "{}{}:{}:{}:{}",
            EMB_PREFIX, schema, key_hash, field_name, fragment_idx
        )
    }

    /// Legacy storage key: emb:{schema}:{key_hash}
    #[allow(dead_code)] // Used when migrating old-format entries
    pub(super) fn legacy_storage_key(schema: &str, key: &KeyValue) -> String {
        let key_hash = Self::key_hash(key);
        format!("{}{}:{}", EMB_PREFIX, schema, key_hash)
    }

    fn key_hash(key: &KeyValue) -> String {
        match (&key.hash, &key.range) {
            (Some(h), Some(r)) => format!("{}_{}", h, r),
            (Some(h), None) => h.clone(),
            (None, Some(r)) => format!("_{}", r),
            (None, None) => "empty".to_string(),
        }
    }

    /// Returns true if this is a legacy (pre-fragmentation) entry.
    pub(super) fn is_legacy(&self) -> bool {
        self.field_name.is_empty() && !self.legacy_field_names.is_empty()
    }
}

/// Bundle of fragment metadata for insert_fragment (avoids too-many-arguments).
pub(super) struct FragmentInfo<'a> {
    pub schema: &'a str,
    pub key: &'a KeyValue,
    pub field_name: &'a str,
    pub fragment_idx: usize,
    pub fragment_text: Option<String>,
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
    /// Handles both old (per-record) and new (per-fragment) formats.
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
                    entries.push(EmbeddingEntry {
                        schema: stored.schema,
                        key: stored.key,
                        field_name: stored.field_name,
                        fragment_idx: stored.fragment_idx,
                        fragment_text: stored.fragment_text,
                        embedding: stored.embedding,
                        legacy_field_names: stored.field_names,
                    });
                }
                Err(e) => log::warn!("Failed to deserialize StoredEmbedding: {}", e),
            }
        }

        log::info!("Loaded {} embeddings from store", entries.len());
        entries
    }

    /// Persist and upsert a single fragment embedding.
    pub(super) async fn insert_fragment(
        &self,
        store: &dyn KvStore,
        info: FragmentInfo<'_>,
        embedding: Vec<f32>,
    ) -> Result<(), SchemaError> {
        let stored = StoredEmbedding {
            schema: info.schema.to_string(),
            key: info.key.clone(),
            field_name: info.field_name.to_string(),
            fragment_idx: info.fragment_idx,
            fragment_text: info.fragment_text.clone(),
            embedding: embedding.clone(),
            field_names: Vec::new(),
        };

        let storage_key = EmbeddingEntry::fragment_storage_key(
            info.schema,
            info.key,
            info.field_name,
            info.fragment_idx,
        );
        let bytes = serde_json::to_vec(&stored)
            .map_err(|e| SchemaError::InvalidData(format!("Serialization failed: {}", e)))?;

        store
            .put(storage_key.as_bytes(), bytes)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Storage write failed: {}", e)))?;

        let schema = info.schema;
        let key = info.key;
        let field_name = info.field_name;
        let fragment_idx = info.fragment_idx;

        let new_entry = EmbeddingEntry {
            schema: schema.to_string(),
            key: key.clone(),
            field_name: field_name.to_string(),
            fragment_idx,
            fragment_text: info.fragment_text,
            embedding,
            legacy_field_names: Vec::new(),
        };

        let mut entries = self.entries.write().unwrap();
        if let Some(existing) = entries.iter_mut().find(|e| {
            e.schema == schema
                && e.key == *key
                && e.field_name == field_name
                && e.fragment_idx == fragment_idx
        }) {
            *existing = new_entry;
        } else {
            entries.push(new_entry);
        }

        Ok(())
    }

    /// Reload embeddings from the store, adding any entries not already in the in-memory index.
    /// Returns the number of newly added entries.
    pub(super) async fn reload_from_store(&self, store: &dyn KvStore) -> usize {
        let new_entries = Self::load_from_store(store).await;
        let mut current = self.entries.write().unwrap();
        let before = current.len();

        // Build a set of existing storage keys for deduplication
        let existing_keys: std::collections::HashSet<String> = current
            .iter()
            .map(|e| {
                EmbeddingEntry::fragment_storage_key(
                    &e.schema,
                    &e.key,
                    &e.field_name,
                    e.fragment_idx,
                )
            })
            .collect();

        for entry in new_entries {
            let key = EmbeddingEntry::fragment_storage_key(
                &entry.schema,
                &entry.key,
                &entry.field_name,
                entry.fragment_idx,
            );
            if !existing_keys.contains(&key) {
                current.push(entry);
            }
        }

        let added = current.len() - before;
        if added > 0 {
            log::info!("reload_from_store: added {} new embeddings to index", added);
        }
        added
    }

    /// Brute-force cosine similarity search. Returns up to `k` results sorted by score,
    /// deduplicated by (schema, key) — taking the highest-scoring fragment per record.
    pub(super) fn search(&self, query_vec: &[f32], k: usize) -> Vec<IndexResult> {
        let entries = self.entries.read().unwrap();

        // Score every entry
        let mut scored: Vec<(f32, usize)> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (cosine_similarity(query_vec, &e.embedding), i))
            .collect();

        scored.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Deduplicate by (schema, key): keep only the highest-scoring fragment per record.
        let mut seen: HashMap<(String, KeyValue), f32> = HashMap::new();
        let mut best_per_record: Vec<(f32, usize)> = Vec::new();

        for (score, idx) in &scored {
            let e = &entries[*idx];
            let record_key = (e.schema.clone(), e.key.clone());
            if let std::collections::hash_map::Entry::Vacant(entry) = seen.entry(record_key) {
                entry.insert(*score);
                best_per_record.push((*score, *idx));
                if best_per_record.len() >= k {
                    break;
                }
            }
        }

        // Expand each matched record to results.
        // For new per-fragment entries: one IndexResult with the matched field.
        // For legacy entries: one IndexResult per field_name (backward compat).
        best_per_record
            .into_iter()
            .flat_map(|(score, i)| {
                let e = &entries[i];
                if e.is_legacy() {
                    // Legacy: expand to one result per field
                    e.legacy_field_names
                        .iter()
                        .map(|field| IndexResult {
                            schema_name: e.schema.clone(),
                            schema_display_name: None,
                            field: field.clone(),
                            key_value: e.key.clone(),
                            value: Value::Null,
                            metadata: Some(
                                serde_json::json!({"score": score, "match_type": "semantic"}),
                            ),
                            molecule_versions: None,
                        })
                        .collect::<Vec<_>>()
                } else {
                    // Per-fragment: one result for the matched field+fragment
                    vec![IndexResult {
                        schema_name: e.schema.clone(),
                        schema_display_name: None,
                        field: e.field_name.clone(),
                        key_value: e.key.clone(),
                        value: Value::Null,
                        metadata: Some(serde_json::json!({
                            "score": score,
                            "match_type": "semantic",
                            "fragment_idx": e.fragment_idx,
                        })),
                        molecule_versions: None,
                    }]
                }
            })
            .collect()
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
