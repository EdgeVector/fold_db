//! Face detection and embedding for photo discovery.
//!
//! Uses SCRFD for face detection and ArcFace for face embedding.
//! Both models run locally via ONNX Runtime.
//! Gated behind the `face-detection` cargo feature.

#[cfg(feature = "face-detection")]
mod detector;
#[cfg(feature = "face-detection")]
mod embedder;
#[cfg(feature = "face-detection")]
mod model;
#[cfg(feature = "face-detection")]
mod pipeline;

use serde::{Deserialize, Serialize};

use crate::schema::SchemaError;

/// A detected and embedded face from an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceEmbedding {
    /// 512-dimensional L2-normalized face embedding vector (ArcFace).
    pub embedding: Vec<f32>,
    /// Bounding box [x1, y1, x2, y2] normalized to [0, 1].
    pub bbox: [f32; 4],
    /// Detection confidence score.
    pub confidence: f32,
}

/// Trait for face detection and embedding.
pub trait FaceProcessor: Send + Sync {
    /// Detect faces in an image and return embeddings for each.
    fn detect_and_embed(&self, image_bytes: &[u8]) -> Result<Vec<FaceEmbedding>, SchemaError>;
}

#[cfg(feature = "face-detection")]
pub use pipeline::OnnxFaceProcessor;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_face_embedding_struct() {
        let fe = FaceEmbedding {
            embedding: vec![0.1; 512],
            bbox: [0.1, 0.2, 0.3, 0.4],
            confidence: 0.95,
        };
        assert_eq!(fe.embedding.len(), 512);
        assert!(fe.confidence > 0.9);
    }

    #[test]
    fn test_face_embedding_serialization() {
        let fe = FaceEmbedding {
            embedding: vec![0.5; 512],
            bbox: [0.0, 0.0, 1.0, 1.0],
            confidence: 0.99,
        };
        let json = serde_json::to_string(&fe).unwrap();
        let deserialized: FaceEmbedding = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.embedding.len(), 512);
        assert_eq!(deserialized.confidence, 0.99);
    }
}
