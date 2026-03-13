pub mod anonymity;
mod embedding_index;
mod embedding_model;
pub mod fragmentation;
mod types;

#[cfg(test)]
mod tests;

pub use anonymity::{
    check_fragment_anonymity, default_privacy_class, FieldPrivacyClass, FragmentDecision,
};
pub use embedding_index::cosine_similarity;
pub use embedding_model::{Embedder, FastEmbedModel};
pub use fragmentation::{split_into_fragments, Fragment};
#[cfg(any(test, feature = "test-utils"))]
pub use embedding_model::{MockEmbeddingModel, ScriptedEmbeddingModel};
pub use types::IndexResult;

use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;
use crate::storage::traits::KvStore;
use embedding_index::EmbeddingIndex;
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

    /// Index each field of a record as an independent embedding.
    ///
    /// Each field value is embedded separately (and long text fields are further
    /// split into sentence-level fragments). This produces one embedding per
    /// fragment, enabling per-fragment discovery and anonymity checks.
    pub async fn index_record(
        &self,
        schema: &str,
        key: &KeyValue,
        fields_and_values: &HashMap<String, serde_json::Value>,
    ) -> Result<(), SchemaError> {
        for (field_name, value) in fields_and_values {
            let text = value_to_text(value);
            if text.trim().is_empty() {
                continue;
            }

            let fragments = split_into_fragments(&text);
            for fragment in &fragments {
                let embedding = self.embedding_model.embed_text(&fragment.text)?;
                self.embedding_index
                    .insert_fragment(
                        &*self.store,
                        schema,
                        key,
                        field_name,
                        fragment.index,
                        embedding,
                    )
                    .await?;
            }
        }
        Ok(())
    }

    /// Semantic search: embed the query then return top results by cosine similarity.
    ///
    /// Results are deduplicated by record key — if multiple fragments of the same
    /// record match, only the highest-scoring match is returned.
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

fn value_to_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Array(arr) => arr.iter().map(value_to_text).collect::<Vec<_>>().join(" "),
        serde_json::Value::Object(obj) => {
            obj.values().map(value_to_text).collect::<Vec<_>>().join(" ")
        }
        serde_json::Value::Null => String::new(),
    }
}
