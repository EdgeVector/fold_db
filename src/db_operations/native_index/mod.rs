mod embedding_index;
mod embedding_model;
mod types;

#[cfg(test)]
mod tests;

pub use types::IndexResult;

use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;
use crate::storage::traits::KvStore;
use embedding_index::{fields_to_text, EmbeddingIndex};
use embedding_model::{Embedder, FastEmbedModel};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct NativeIndexManager {
    store: Arc<dyn KvStore>,
    embedding_model: Arc<dyn Embedder>,
    embedding_index: Arc<EmbeddingIndex>,
}

impl NativeIndexManager {
    pub fn new(store: Arc<dyn KvStore>) -> Self {
        Self {
            store,
            embedding_model: Arc::new(FastEmbedModel::new()),
            embedding_index: Arc::new(EmbeddingIndex::new(Vec::new())),
        }
    }

    /// Load persisted embeddings from the store. Call this once after `new()` during node startup.
    pub async fn restore_from_store(&self) {
        let entries = EmbeddingIndex::load_from_store(&*self.store).await;
        *self.embedding_index.entries.write().unwrap() = entries;
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub(crate) fn with_model(store: Arc<dyn KvStore>, model: Arc<dyn Embedder>) -> Self {
        Self {
            store,
            embedding_model: model,
            embedding_index: Arc::new(EmbeddingIndex::new(Vec::new())),
        }
    }

    /// Index all fields of a record as a single document embedding.
    pub async fn index_record(
        &self,
        schema: &str,
        key: &KeyValue,
        fields_and_values: &HashMap<String, serde_json::Value>,
    ) -> Result<(), SchemaError> {
        let text = fields_to_text(fields_and_values);
        if text.trim().is_empty() {
            return Ok(());
        }

        let embedding = self.embedding_model.embed_text(&text)?;
        let field_names: Vec<String> = fields_and_values.keys().cloned().collect();

        self.embedding_index
            .insert(&*self.store, schema, key, field_names, embedding)
            .await
    }

    /// Get the underlying KV store (used by discovery publisher to scan embeddings).
    pub fn store(&self) -> &Arc<dyn KvStore> {
        &self.store
    }

    /// Embed a text query into a vector. Used by discovery to generate search embeddings.
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>, SchemaError> {
        self.embedding_model.embed_text(text)
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
}
