use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use ort::session::Session;
use ort::value::Tensor;

use crate::schema::SchemaError;

pub struct FaceDetection {
    pub bbox: [f32; 4], // x1, y1, x2, y2 in original image pixels
    pub confidence: f32,
    /// 5 facial landmarks (used for face alignment in Phase 2).
    #[allow(dead_code)]
    pub landmarks: Option<[[f32; 2]; 5]>,
}

pub struct ScrfdDetector {
    session: Session,
    input_size: (u32, u32), // (640, 640)
    conf_threshold: f32,
    nms_threshold: f32,
}

impl ScrfdDetector {
    pub fn new(model_path: &std::path::Path) -> Result<Self, SchemaError> {
        let session = Session::builder()
            .map_err(|e| SchemaError::InvalidData(format!("ONNX session builder error: {e}")))?
            .with_intra_threads(2)
            .map_err(|e| SchemaError::InvalidData(format!("ONNX thread config error: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to load SCRFD model: {e}")))?;

        Ok(Self {
            session,
            input_size: (640, 640),
            conf_threshold: 0.5,
            nms_threshold: 0.4,
        })
    }

    pub fn detect(&self, image: &DynamicImage) -> Result<Vec<FaceDetection>, SchemaError> {
        let (orig_w, orig_h) = image.dimensions();
        let (input_w, input_h) = self.input_size;

        // Resize image to model input size
        let resized = image.resize_exact(input_w, input_h, FilterType::Lanczos3);

        // Convert to CHW float tensor [1, 3, H, W]
        let mut input_tensor = vec![0.0f32; (3 * input_h * input_w) as usize];
        for y in 0..input_h {
            for x in 0..input_w {
                let pixel = resized.get_pixel(x, y);
                let idx = (y * input_w + x) as usize;
                input_tensor[idx] = pixel[0] as f32; // R
                input_tensor[(input_h * input_w) as usize + idx] = pixel[1] as f32; // G
                input_tensor[2 * (input_h * input_w) as usize + idx] = pixel[2] as f32;
                // B
            }
        }

        // Run inference
        let input = Tensor::from_array((
            [1usize, 3, input_h as usize, input_w as usize],
            input_tensor.as_slice(),
        ))
        .map_err(|e| SchemaError::InvalidData(format!("Failed to create input tensor: {e}")))?;

        let inputs = ort::inputs![input]
            .map_err(|e| SchemaError::InvalidData(format!("Failed to create inputs: {e}")))?;

        let outputs = self
            .session
            .run(inputs)
            .map_err(|e| SchemaError::InvalidData(format!("SCRFD inference failed: {e}")))?;

        // Parse SCRFD outputs: the model has multiple stride outputs
        // For scrfd_2.5g_bnkps, outputs are grouped by stride (8, 16, 32):
        // score_8, bbox_8, kps_8, score_16, bbox_16, kps_16, score_32, bbox_32, kps_32
        let mut detections = Vec::new();
        let strides = [8u32, 16, 32];
        let num_outputs_per_stride = 3; // score, bbox, kps

        for (stride_idx, &stride) in strides.iter().enumerate() {
            let score_idx = stride_idx * num_outputs_per_stride;
            let bbox_idx = score_idx + 1;
            let kps_idx = score_idx + 2;

            if score_idx >= outputs.len() || bbox_idx >= outputs.len() {
                continue;
            }

            let scores = outputs[score_idx]
                .try_extract_tensor::<f32>()
                .map_err(|e| SchemaError::InvalidData(format!("Failed to extract scores: {e}")))?;
            let bboxes = outputs[bbox_idx]
                .try_extract_tensor::<f32>()
                .map_err(|e| SchemaError::InvalidData(format!("Failed to extract bboxes: {e}")))?;

            let scores_view = scores.view();
            let bboxes_view = bboxes.view();

            let feat_h = input_h / stride;
            let feat_w = input_w / stride;

            // SCRFD uses anchor-free detection with distance predictions
            for fy in 0..feat_h {
                for fx in 0..feat_w {
                    for anchor_idx in 0..2u32 {
                        // 2 anchors per position
                        let idx = ((fy * feat_w + fx) * 2 + anchor_idx) as usize;
                        if idx >= scores_view.len() {
                            continue;
                        }

                        let score = scores_view[[0, idx, 0]];
                        if score < self.conf_threshold {
                            continue;
                        }

                        // Center point
                        let cx = (fx as f32 + 0.5) * stride as f32;
                        let cy = (fy as f32 + 0.5) * stride as f32;

                        // Distance predictions (left, top, right, bottom from center)
                        let l = bboxes_view[[0, idx, 0]] * stride as f32;
                        let t = bboxes_view[[0, idx, 1]] * stride as f32;
                        let r = bboxes_view[[0, idx, 2]] * stride as f32;
                        let b = bboxes_view[[0, idx, 3]] * stride as f32;

                        let x1 = (cx - l).max(0.0) / input_w as f32 * orig_w as f32;
                        let y1 = (cy - t).max(0.0) / input_h as f32 * orig_h as f32;
                        let x2 = (cx + r).min(input_w as f32) / input_w as f32 * orig_w as f32;
                        let y2 = (cy + b).min(input_h as f32) / input_h as f32 * orig_h as f32;

                        // Extract landmarks if available
                        let landmarks = if kps_idx < outputs.len() {
                            let kps = outputs[kps_idx].try_extract_tensor::<f32>().ok();
                            kps.map(|k| {
                                let k_view = k.view();
                                let mut pts = [[0.0f32; 2]; 5];
                                for (i, pt) in pts.iter_mut().enumerate() {
                                    let kx_idx = i * 2;
                                    let ky_idx = i * 2 + 1;
                                    if kx_idx + 1 < k_view.shape()[2] {
                                        pt[0] = (cx + k_view[[0, idx, kx_idx]] * stride as f32)
                                            / input_w as f32
                                            * orig_w as f32;
                                        pt[1] = (cy + k_view[[0, idx, ky_idx]] * stride as f32)
                                            / input_h as f32
                                            * orig_h as f32;
                                    }
                                }
                                pts
                            })
                        } else {
                            None
                        };

                        detections.push(FaceDetection {
                            bbox: [x1, y1, x2, y2],
                            confidence: score,
                            landmarks,
                        });
                    }
                }
            }
        }

        // Apply NMS
        self.nms(&mut detections);

        Ok(detections)
    }

    fn nms(&self, detections: &mut Vec<FaceDetection>) {
        detections.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut keep = vec![true; detections.len()];
        for i in 0..detections.len() {
            if !keep[i] {
                continue;
            }
            for j in (i + 1)..detections.len() {
                if !keep[j] {
                    continue;
                }
                if self.iou(&detections[i].bbox, &detections[j].bbox) > self.nms_threshold {
                    keep[j] = false;
                }
            }
        }

        let mut idx = 0;
        detections.retain(|_| {
            let k = keep[idx];
            idx += 1;
            k
        });
    }

    fn iou(&self, a: &[f32; 4], b: &[f32; 4]) -> f32 {
        let x1 = a[0].max(b[0]);
        let y1 = a[1].max(b[1]);
        let x2 = a[2].min(b[2]);
        let y2 = a[3].min(b[3]);

        let inter = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
        let area_a = (a[2] - a[0]) * (a[3] - a[1]);
        let area_b = (b[2] - b[0]) * (b[3] - b[1]);
        let union = area_a + area_b - inter;

        if union <= 0.0 {
            0.0
        } else {
            inter / union
        }
    }
}
