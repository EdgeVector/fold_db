use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use ort::session::Session;
use ort::value::Tensor;

use crate::schema::SchemaError;

pub struct FaceDetection {
    pub bbox: [f32; 4], // x1, y1, x2, y2 in original image pixels
    pub confidence: f32,
    /// 5 facial landmarks (used for face alignment).
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

        let resized = image.resize_exact(input_w, input_h, FilterType::Lanczos3);

        // Convert to CHW float tensor [1, 3, H, W] with the standard SCRFD
        // input preprocessing: subtract mean 127.5 and scale by 1/128 so each
        // channel ends up roughly in [-1, 1]. This matches what insightface
        // does for the buffalo_sc / scrfd_2.5g_bnkps weights:
        //
        //   cv2.dnn.blobFromImage(img, 1.0/128.0, input_size,
        //                         (127.5, 127.5, 127.5), swapRB=True)
        //
        // Without normalization the network sees raw 0–255 pixels, its
        // activations are wildly off, and it returns a sea of low-confidence
        // (0.5–0.7) detections at semi-random feature-map positions instead
        // of the prominent face. Downstream that manifests as bboxes that
        // are ~0.02–0.05 of the image width regardless of how big the face
        // actually is, AND face-search across photos of the same person
        // fails because the embeddings are computed from noise crops, not
        // actual faces. The cv2 reference also does swapRB=True because cv2
        // reads BGR; the `image` crate already gives us RGB so we skip the
        // swap.
        let mut input_tensor = vec![0.0f32; (3 * input_h * input_w) as usize];
        let plane = (input_h * input_w) as usize;
        for y in 0..input_h {
            for x in 0..input_w {
                let pixel = resized.get_pixel(x, y);
                let idx = (y * input_w + x) as usize;
                input_tensor[idx] = (pixel[0] as f32 - 127.5) / 128.0;
                input_tensor[plane + idx] = (pixel[1] as f32 - 127.5) / 128.0;
                input_tensor[2 * plane + idx] = (pixel[2] as f32 - 127.5) / 128.0;
            }
        }

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

        // SCRFD det_500m outputs 9 tensors (3 strides × 3 types):
        //   [0] scores_8  [N8, 1]    [3] bboxes_8  [N8, 4]    [6] kps_8  [N8, 10]
        //   [1] scores_16 [N16, 1]   [4] bboxes_16 [N16, 4]   [7] kps_16 [N16, 10]
        //   [2] scores_32 [N32, 1]   [5] bboxes_32 [N32, 4]   [8] kps_32 [N32, 10]
        // where N_s = (640/s)^2 * 2 anchors
        let mut detections = Vec::new();
        let strides = [8u32, 16, 32];

        for (stride_idx, &stride) in strides.iter().enumerate() {
            let score_idx = stride_idx; // 0, 1, 2
            let bbox_idx = stride_idx + 3; // 3, 4, 5
            let kps_idx = stride_idx + 6; // 6, 7, 8

            if bbox_idx >= outputs.len() {
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

            for fy in 0..feat_h {
                for fx in 0..feat_w {
                    for anchor in 0..2u32 {
                        let idx = ((fy * feat_w + fx) * 2 + anchor) as usize;
                        if idx >= scores_view.shape()[0] {
                            continue;
                        }

                        let score = scores_view[[idx, 0]];
                        if score < self.conf_threshold {
                            continue;
                        }

                        let cx = (fx as f32 + 0.5) * stride as f32;
                        let cy = (fy as f32 + 0.5) * stride as f32;

                        let l = bboxes_view[[idx, 0]] * stride as f32;
                        let t = bboxes_view[[idx, 1]] * stride as f32;
                        let r = bboxes_view[[idx, 2]] * stride as f32;
                        let b = bboxes_view[[idx, 3]] * stride as f32;

                        let x1 = (cx - l).max(0.0) / input_w as f32 * orig_w as f32;
                        let y1 = (cy - t).max(0.0) / input_h as f32 * orig_h as f32;
                        let x2 = (cx + r).min(input_w as f32) / input_w as f32 * orig_w as f32;
                        let y2 = (cy + b).min(input_h as f32) / input_h as f32 * orig_h as f32;

                        let landmarks = if kps_idx < outputs.len() {
                            outputs[kps_idx].try_extract_tensor::<f32>().ok().map(|k| {
                                let k_view = k.view();
                                let mut pts = [[0.0f32; 2]; 5];
                                for (i, pt) in pts.iter_mut().enumerate() {
                                    let kx = i * 2;
                                    let ky = i * 2 + 1;
                                    if ky < k_view.shape()[1] {
                                        pt[0] = (cx + k_view[[idx, kx]] * stride as f32)
                                            / input_w as f32
                                            * orig_w as f32;
                                        pt[1] = (cy + k_view[[idx, ky]] * stride as f32)
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
