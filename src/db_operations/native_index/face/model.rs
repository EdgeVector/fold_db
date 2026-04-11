use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::schema::SchemaError;

const MODELS_DIR: &str = "models";

/// Model pack URL (InsightFace buffalo_sc — smallest pack, ~15MB)
/// Contains det_500m.onnx (SCRFD, 2.5MB) and w600k_mbf.onnx (MobileFaceNet, 13MB)
const MODEL_PACK_URL: &str =
    "https://github.com/deepinsight/insightface/releases/download/v0.7/buffalo_sc.zip";

const SCRFD_FILENAME: &str = "scrfd_2.5g_bnkps.onnx";
const ARCFACE_FILENAME: &str = "arcface_r100.onnx";

// Names inside the zip
const SCRFD_ZIP_NAME: &str = "det_500m.onnx";
const ARCFACE_ZIP_NAME: &str = "w600k_mbf.onnx";

pub struct ModelManager {
    models_dir: PathBuf,
}

impl ModelManager {
    pub fn new(folddb_home: &Path) -> Self {
        Self {
            models_dir: folddb_home.join(MODELS_DIR),
        }
    }

    pub fn scrfd_path(&self) -> Result<PathBuf, SchemaError> {
        let path = self.models_dir.join(SCRFD_FILENAME);
        if path.exists() {
            return Ok(path);
        }
        self.download_model_pack()?;
        if path.exists() {
            Ok(path)
        } else {
            Err(SchemaError::InvalidData(
                "SCRFD model not found after download".to_string(),
            ))
        }
    }

    pub fn arcface_path(&self) -> Result<PathBuf, SchemaError> {
        let path = self.models_dir.join(ARCFACE_FILENAME);
        if path.exists() {
            return Ok(path);
        }
        self.download_model_pack()?;
        if path.exists() {
            Ok(path)
        } else {
            Err(SchemaError::InvalidData(
                "ArcFace model not found after download".to_string(),
            ))
        }
    }

    fn download_model_pack(&self) -> Result<(), SchemaError> {
        std::fs::create_dir_all(&self.models_dir).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to create models directory: {e}"))
        })?;

        log::info!("Downloading face models from {} ...", MODEL_PACK_URL);
        let response = reqwest::blocking::get(MODEL_PACK_URL)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to download models: {e}")))?;

        if !response.status().is_success() {
            return Err(SchemaError::InvalidData(format!(
                "Model download failed with status {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to read model bytes: {e}")))?;

        // Extract ONNX files from the zip
        let cursor = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to open model zip: {e}")))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| SchemaError::InvalidData(format!("Failed to read zip entry: {e}")))?;

            let name = file.name().to_string();
            if !name.ends_with(".onnx") {
                continue;
            }

            // Map zip filenames to our canonical names
            let dest_name = if name.contains(SCRFD_ZIP_NAME) || name.ends_with(SCRFD_ZIP_NAME) {
                SCRFD_FILENAME
            } else if name.contains(ARCFACE_ZIP_NAME) || name.ends_with(ARCFACE_ZIP_NAME) {
                ARCFACE_FILENAME
            } else {
                continue;
            };

            let dest = self.models_dir.join(dest_name);
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to extract {}: {e}", name))
            })?;
            std::fs::write(&dest, &contents).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to write {}: {e}", dest.display()))
            })?;
            log::info!("Extracted {} ({} bytes)", dest.display(), contents.len());
        }

        Ok(())
    }
}
