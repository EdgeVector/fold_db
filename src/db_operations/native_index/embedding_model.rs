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
/// Uses a hash-based approach to assign each unique input a near-orthogonal direction,
/// ensuring that different field names produce low cosine similarity (< 0.5).
/// This prevents false semantic matches in tests that don't care about field matching.
#[cfg(any(test, feature = "test-utils"))]
pub struct MockEmbeddingModel;

/// Scripted embedder for tests — returns pre-configured vectors for specific inputs.
/// Use this when you need to control exact similarity values between fields.
///
/// Example: to make "the creator of the Art" and "the artist of the Art" produce
/// cosine similarity of 0.95, give them nearly-identical vectors.
#[cfg(any(test, feature = "test-utils"))]
pub struct ScriptedEmbeddingModel {
    /// Map from input text → embedding vector. Falls back to MockEmbeddingModel for unknown inputs.
    pub responses: std::collections::HashMap<String, Vec<f32>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl ScriptedEmbeddingModel {
    pub fn new(responses: std::collections::HashMap<String, Vec<f32>>) -> Self {
        Self { responses }
    }

    /// Helper: create a unit vector pointing in the given direction index (out of 384 dims).
    /// Two vectors with nearby direction indices will have high cosine similarity.
    pub fn unit_vec(direction: usize) -> Vec<f32> {
        let mut vec = vec![0.0f32; 384];
        vec[direction % 384] = 1.0;
        vec
    }

    /// Helper: create a vector that is a blend of two directions.
    /// blend=0.0 gives pure dir_a, blend=1.0 gives pure dir_b.
    pub fn blended_vec(dir_a: usize, dir_b: usize, blend: f32) -> Vec<f32> {
        let mut vec = vec![0.0f32; 384];
        vec[dir_a % 384] += 1.0 - blend;
        vec[dir_b % 384] += blend;
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter_mut().for_each(|x| *x /= norm);
        }
        vec
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Embedder for ScriptedEmbeddingModel {
    fn embed_text(&self, text: &str) -> Result<Vec<f32>, SchemaError> {
        if let Some(vec) = self.responses.get(text) {
            return Ok(vec.clone());
        }
        // Fall back to MockEmbeddingModel behavior for unscripted inputs
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

#[cfg(any(test, feature = "test-utils"))]
impl Embedder for MockEmbeddingModel {
    fn embed_text(&self, text: &str) -> Result<Vec<f32>, SchemaError> {
        // Use a simple hash to pick a primary direction, plus spread some energy
        // to nearby dimensions. This makes different texts near-orthogonal while
        // keeping identical texts identical.
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        let mut vec = vec![0.0f32; 384];
        // Spread energy across 4 dimensions derived from the hash
        for i in 0..4 {
            let idx = ((hash >> (i * 16)) & 0xFF) as usize % 384;
            let sign = if (hash >> (i * 8 + 4)) & 1 == 0 { 1.0 } else { -1.0 };
            vec[idx] += sign * (4.0 - i as f32); // decreasing weight
        }
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter_mut().for_each(|x| *x /= norm);
        }
        Ok(vec)
    }
}
