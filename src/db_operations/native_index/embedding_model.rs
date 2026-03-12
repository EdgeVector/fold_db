use crate::schema::SchemaError;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use once_cell::sync::OnceCell;

/// Trait for embedding text into a fixed-dimension float vector.
pub trait Embedder: Send + Sync {
    fn embed_text(&self, text: &str) -> Result<Vec<f32>, SchemaError>;
}

/// Production embedder: all-MiniLM-L6-v2 via fastembed (ONNX).
/// Lazily initialized — model downloads on first call to embed_text.
pub struct FastEmbedModel {
    model: OnceCell<TextEmbedding>,
}

impl Default for FastEmbedModel {
    fn default() -> Self {
        Self { model: OnceCell::new() }
    }
}

impl FastEmbedModel {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Embedder for FastEmbedModel {
    fn embed_text(&self, text: &str) -> Result<Vec<f32>, SchemaError> {
        let model = self.model.get_or_try_init(|| {
            TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))
                .map_err(|e| SchemaError::InvalidData(format!("Failed to init embedding model: {}", e)))
        })?;

        let mut results = model
            .embed(vec![text], None)
            .map_err(|e| SchemaError::InvalidData(format!("Embedding failed: {}", e)))?;

        results
            .pop()
            .ok_or_else(|| SchemaError::InvalidData("No embedding returned".to_string()))
    }
}

/// Mock embedder for tests — deterministic, no download required.
#[cfg(any(test, feature = "test-utils"))]
pub struct MockEmbeddingModel;

#[cfg(any(test, feature = "test-utils"))]
impl Embedder for MockEmbeddingModel {
    fn embed_text(&self, text: &str) -> Result<Vec<f32>, SchemaError> {
        let mut vec = vec![0.0f32; 384];
        for (i, byte) in text.bytes().enumerate() {
            vec[i % 384] += byte as f32;
        }
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter_mut().for_each(|x| *x /= norm);
        }
        Ok(vec)
    }
}
