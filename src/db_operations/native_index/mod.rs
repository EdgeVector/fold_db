pub mod anonymity;
mod embedding_index;
mod embedding_model;
pub mod face;
pub mod fragmentation;
pub mod pseudonym;
mod types;

#[cfg(test)]
mod tests;

pub use embedding_index::cosine_similarity;
pub use embedding_index::FaceListEntry;
pub use embedding_model::{Embedder, FastEmbedModel};
#[cfg(any(test, feature = "test-utils"))]
pub use embedding_model::{MockEmbeddingModel, ScriptedEmbeddingModel};
pub use face::FaceEmbedding;
pub use types::IndexResult;

use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;
use crate::storage::traits::KvStore;
#[cfg(feature = "face-detection")]
use embedding_index::FACE_FIELD_NAME;
use embedding_index::{EmbeddingIndex, FragmentInfo};
use fragmentation::value_to_fragments;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "face-detection")]
use face::FaceProcessor;

#[derive(Clone)]
pub struct NativeIndexManager {
    store: Arc<dyn KvStore>,
    embedding_model: Arc<dyn Embedder>,
    embedding_index: Arc<EmbeddingIndex>,
    #[cfg(feature = "face-detection")]
    face_processor: once_cell::sync::OnceCell<Arc<dyn FaceProcessor>>,
}

impl NativeIndexManager {
    pub fn new(store: Arc<dyn KvStore>) -> Self {
        Self {
            store,
            embedding_model: Arc::new(FastEmbedModel::new()),
            embedding_index: Arc::new(EmbeddingIndex::new(Vec::new())),
            #[cfg(feature = "face-detection")]
            face_processor: once_cell::sync::OnceCell::new(),
        }
    }

    /// Load persisted embeddings from the store. Call this once after `new()` during node startup.
    pub async fn restore_from_store(&self) {
        let entries = EmbeddingIndex::load_from_store(&*self.store).await;
        *self.embedding_index.entries.write().unwrap() = entries;
    }

    #[allow(dead_code)]
    #[cfg(any(test, feature = "test-utils"))]
    pub(crate) fn with_model(store: Arc<dyn KvStore>, model: Arc<dyn Embedder>) -> Self {
        Self {
            store,
            embedding_model: model,
            embedding_index: Arc::new(EmbeddingIndex::new(Vec::new())),
            #[cfg(feature = "face-detection")]
            face_processor: once_cell::sync::OnceCell::new(),
        }
    }

    /// Index each field of a record independently, splitting into per-fragment embeddings.
    pub async fn index_record(
        &self,
        schema: &str,
        key: &KeyValue,
        fields_and_values: &HashMap<String, serde_json::Value>,
    ) -> Result<(), SchemaError> {
        for (field_name, value) in fields_and_values {
            let fragments = value_to_fragments(value);
            for (idx, fragment_text) in fragments.iter().enumerate() {
                if fragment_text.trim().is_empty() {
                    continue;
                }
                let embedding = self.embedding_model.embed_text(fragment_text)?;
                let info = FragmentInfo {
                    schema,
                    key,
                    field_name,
                    fragment_idx: idx,
                    fragment_text: Some(fragment_text.clone()),
                    face_bbox: None,
                    face_confidence: None,
                };
                self.embedding_index
                    .insert_fragment(&*self.store, info, embedding)
                    .await?;
            }
        }
        Ok(())
    }

    /// Get the underlying KV store (used by discovery publisher to scan embeddings).
    pub fn store(&self) -> &Arc<dyn KvStore> {
        &self.store
    }

    /// Get the embedding model (used by interest detection to embed seed phrases).
    pub fn embedder(&self) -> &Arc<dyn Embedder> {
        &self.embedding_model
    }

    /// Embed a text query into a vector. Used by discovery to generate search embeddings.
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>, SchemaError> {
        self.embedding_model.embed_text(text)
    }

    /// Reload embeddings from the persistent store into the in-memory index.
    /// Called after sync replays new native_index entries. Returns the count of
    /// newly added embeddings.
    pub async fn reload_embeddings(&self) -> usize {
        self.embedding_index.reload_from_store(&*self.store).await
    }

    /// Purge all org embeddings from both Sled and the in-memory index.
    /// Embedding keys use format `emb:{org_hash}:{schema}:...`, so we scan for `emb:{org_hash}:`.
    /// In-memory entries have schema = `{org_hash}:...`, so we retain entries where schema
    /// does NOT start with `{org_hash}:`.
    /// Returns the total number of entries removed (Sled + in-memory).
    pub async fn purge_org_embeddings(&self, org_hash: &str) -> Result<usize, SchemaError> {
        // 1. Delete from Sled store
        let emb_prefix = format!("emb:{}:", org_hash);
        let raw = self
            .store
            .scan_prefix(emb_prefix.as_bytes())
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to scan native_index: {}", e)))?;

        let sled_count = raw.len();
        for (key, _) in &raw {
            self.store.delete(key).await.map_err(|e| {
                SchemaError::InvalidData(format!("Failed to delete embedding key: {}", e))
            })?;
        }

        // 2. Purge in-memory index
        let mem_count = self.embedding_index.purge_org(org_hash);

        let total = sled_count.max(mem_count);
        if total > 0 {
            log::info!(
                "purge_org_embeddings: deleted {} from store, {} from memory for org {}",
                sled_count,
                mem_count,
                &org_hash[..12.min(org_hash.len())]
            );
        }
        Ok(total)
    }

    /// Semantic search: embed the query then return top-50 results by cosine similarity.
    pub async fn search_all_classifications(
        &self,
        query: &str,
    ) -> Result<Vec<IndexResult>, SchemaError> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let query_vec = self.embedding_model.embed_text(query)?;
        Ok(self.embedding_index.search(&query_vec, 50))
    }

    /// Returns true if a face processor is available for face detection/embedding.
    pub fn has_face_processor(&self) -> bool {
        #[cfg(feature = "face-detection")]
        {
            self.face_processor.get().is_some()
        }
        #[cfg(not(feature = "face-detection"))]
        {
            false
        }
    }

    /// Set the face processor for face detection and embedding.
    #[cfg(feature = "face-detection")]
    pub fn set_face_processor(&self, processor: Arc<dyn FaceProcessor>) {
        let _ = self.face_processor.set(processor);
    }

    /// Detect faces and return embeddings without mutating the index. Callers that also want the index updated should use `index_faces`.
    #[cfg(feature = "face-detection")]
    pub fn detect_faces(&self, image_bytes: &[u8]) -> Result<Vec<FaceEmbedding>, SchemaError> {
        let processor = self
            .face_processor
            .get()
            .ok_or_else(|| SchemaError::InvalidData("No face processor configured".to_string()))?;
        processor.detect_and_embed(image_bytes)
    }

    /// Detect faces in an image, embed each face, and store the embeddings.
    /// Returns the number of faces detected and indexed.
    #[cfg(feature = "face-detection")]
    pub async fn index_faces(
        &self,
        schema: &str,
        key: &KeyValue,
        image_bytes: &[u8],
    ) -> Result<usize, SchemaError> {
        let processor = self
            .face_processor
            .get()
            .ok_or_else(|| SchemaError::InvalidData("No face processor configured".to_string()))?;

        let faces = processor.detect_and_embed(image_bytes)?;
        let count = faces.len();

        for (idx, face) in faces.into_iter().enumerate() {
            let info = FragmentInfo {
                schema,
                key,
                field_name: FACE_FIELD_NAME,
                fragment_idx: idx,
                fragment_text: None,
                face_bbox: Some(face.bbox),
                face_confidence: Some(face.confidence),
            };
            self.embedding_index
                .insert_fragment(&*self.store, info, face.embedding)
                .await?;
        }

        if count > 0 {
            log::info!(
                "index_faces: indexed {} face(s) for schema={} key={:?}",
                count,
                schema,
                key
            );
        }

        Ok(count)
    }

    /// Search face embeddings by similarity to a query face vector.
    /// Returns up to `k` matching records sorted by cosine similarity.
    pub fn search_faces(&self, query_face_vec: &[f32], k: usize) -> Vec<IndexResult> {
        self.embedding_index.search_faces(query_face_vec, k)
    }

    /// List all face embeddings stored for a specific record. Each entry
    /// carries fragment_idx, embedding, optional bbox + confidence (None for
    /// pre-bbox-plumbing entries).
    pub fn list_faces(&self, schema: &str, key: &KeyValue) -> Vec<FaceListEntry> {
        self.embedding_index.list_faces(schema, key)
    }
}
