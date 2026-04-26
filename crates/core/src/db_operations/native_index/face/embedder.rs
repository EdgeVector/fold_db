use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use ort::session::Session;
use ort::value::Tensor;

use crate::schema::SchemaError;

use super::detector::FaceDetection;

pub struct ArcFaceEmbedder {
    session: Session,
    input_size: (u32, u32), // (112, 112)
}

impl ArcFaceEmbedder {
    pub fn new(model_path: &std::path::Path) -> Result<Self, SchemaError> {
        let session = Session::builder()
            .map_err(|e| SchemaError::InvalidData(format!("ONNX session builder error: {e}")))?
            .with_intra_threads(2)
            .map_err(|e| SchemaError::InvalidData(format!("ONNX thread config error: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to load ArcFace model: {e}")))?;

        Ok(Self {
            session,
            input_size: (112, 112),
        })
    }

    /// Embed a face by cropping from the original image using the detection bbox.
    pub fn embed_face(
        &self,
        image: &DynamicImage,
        detection: &FaceDetection,
    ) -> Result<Vec<f32>, SchemaError> {
        let (img_w, img_h) = image.dimensions();

        // Expand bbox slightly (10% padding) for better alignment
        let pad = 0.1;
        let bw = detection.bbox[2] - detection.bbox[0];
        let bh = detection.bbox[3] - detection.bbox[1];
        let x1 = (detection.bbox[0] - bw * pad).max(0.0) as u32;
        let y1 = (detection.bbox[1] - bh * pad).max(0.0) as u32;
        let x2 = ((detection.bbox[2] + bw * pad) as u32).min(img_w);
        let y2 = ((detection.bbox[3] + bh * pad) as u32).min(img_h);

        let crop_w = x2.saturating_sub(x1);
        let crop_h = y2.saturating_sub(y1);
        if crop_w < 10 || crop_h < 10 {
            return Err(SchemaError::InvalidData("Face crop too small".to_string()));
        }

        // Crop and resize to 112x112
        let face_crop = image.crop_imm(x1, y1, crop_w, crop_h);
        let resized =
            face_crop.resize_exact(self.input_size.0, self.input_size.1, FilterType::Lanczos3);

        // Convert to CHW float tensor, normalized to [-1, 1]
        let (w, h) = self.input_size;
        let mut input_tensor = vec![0.0f32; (3 * h * w) as usize];
        for y in 0..h {
            for x in 0..w {
                let pixel = resized.get_pixel(x, y);
                let idx = (y * w + x) as usize;
                // Normalize: (pixel / 127.5) - 1.0 maps [0, 255] to [-1, 1]
                input_tensor[idx] = (pixel[0] as f32 - 127.5) / 128.0; // R
                input_tensor[(h * w) as usize + idx] = (pixel[1] as f32 - 127.5) / 128.0; // G
                input_tensor[2 * (h * w) as usize + idx] = (pixel[2] as f32 - 127.5) / 128.0;
                // B
            }
        }

        // Run inference
        let input =
            Tensor::from_array(([1usize, 3, h as usize, w as usize], input_tensor.as_slice()))
                .map_err(|e| {
                    SchemaError::InvalidData(format!("Failed to create input tensor: {e}"))
                })?;

        let inputs = ort::inputs![input]
            .map_err(|e| SchemaError::InvalidData(format!("Failed to create inputs: {e}")))?;

        let outputs = self
            .session
            .run(inputs)
            .map_err(|e| SchemaError::InvalidData(format!("ArcFace inference failed: {e}")))?;

        // Extract 512-dim embedding
        let embedding_tensor = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to extract embedding: {e}")))?;

        let embedding_view = embedding_tensor.view();
        let mut embedding: Vec<f32> = embedding_view.iter().copied().collect();

        // L2 normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut embedding {
                *v /= norm;
            }
        }

        Ok(embedding)
    }
}
