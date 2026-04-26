use image::GenericImageView;
use once_cell::sync::OnceCell;

use crate::schema::SchemaError;

use super::detector::ScrfdDetector;
use super::embedder::ArcFaceEmbedder;
use super::model::ModelManager;
use super::{FaceEmbedding, FaceProcessor};

/// ONNX-based face detection and embedding pipeline.
/// Uses SCRFD for detection and ArcFace for embedding.
/// Models are downloaded on first use.
pub struct OnnxFaceProcessor {
    model_manager: ModelManager,
    detector: OnceCell<ScrfdDetector>,
    embedder: OnceCell<ArcFaceEmbedder>,
}

impl OnnxFaceProcessor {
    pub fn new(folddb_home: &std::path::Path) -> Self {
        Self {
            model_manager: ModelManager::new(folddb_home),
            detector: OnceCell::new(),
            embedder: OnceCell::new(),
        }
    }

    fn get_detector(&self) -> Result<&ScrfdDetector, SchemaError> {
        self.detector.get_or_try_init(|| {
            let path = self.model_manager.scrfd_path()?;
            ScrfdDetector::new(&path)
        })
    }

    fn get_embedder(&self) -> Result<&ArcFaceEmbedder, SchemaError> {
        self.embedder.get_or_try_init(|| {
            let path = self.model_manager.arcface_path()?;
            ArcFaceEmbedder::new(&path)
        })
    }
}

impl FaceProcessor for OnnxFaceProcessor {
    fn detect_and_embed(&self, image_bytes: &[u8]) -> Result<Vec<FaceEmbedding>, SchemaError> {
        let image = image::load_from_memory(image_bytes)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to decode image: {e}")))?;

        let detector = self.get_detector()?;
        let embedder = self.get_embedder()?;

        let detections = detector.detect(&image)?;

        let (orig_w, orig_h) = image.dimensions();

        let mut face_embeddings = Vec::new();
        for detection in &detections {
            match embedder.embed_face(&image, detection) {
                Ok(embedding) => {
                    face_embeddings.push(FaceEmbedding {
                        embedding,
                        bbox: [
                            detection.bbox[0] / orig_w as f32,
                            detection.bbox[1] / orig_h as f32,
                            detection.bbox[2] / orig_w as f32,
                            detection.bbox[3] / orig_h as f32,
                        ],
                        confidence: detection.confidence,
                    });
                }
                Err(e) => {
                    log::warn!("Failed to embed face: {}", e);
                }
            }
        }

        Ok(face_embeddings)
    }
}
