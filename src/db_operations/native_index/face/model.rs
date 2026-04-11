use std::path::{Path, PathBuf};

use crate::schema::SchemaError;

const MODELS_DIR: &str = "models";

/// URLs for ONNX model downloads
const SCRFD_URL: &str =
    "https://github.com/nicken/insightface-onnx/raw/refs/heads/master/scrfd_2.5g_bnkps.onnx";
const ARCFACE_URL: &str =
    "https://github.com/nicken/insightface-onnx/raw/refs/heads/master/arcface_r100.onnx";

const SCRFD_FILENAME: &str = "scrfd_2.5g_bnkps.onnx";
const ARCFACE_FILENAME: &str = "arcface_r100.onnx";

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
        self.download(SCRFD_URL, &path)?;
        Ok(path)
    }

    pub fn arcface_path(&self) -> Result<PathBuf, SchemaError> {
        let path = self.models_dir.join(ARCFACE_FILENAME);
        if path.exists() {
            return Ok(path);
        }
        self.download(ARCFACE_URL, &path)?;
        Ok(path)
    }

    fn download(&self, url: &str, dest: &Path) -> Result<(), SchemaError> {
        std::fs::create_dir_all(&self.models_dir).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to create models directory: {e}"))
        })?;

        log::info!("Downloading face model from {} ...", url);
        let response = reqwest::blocking::get(url).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to download model from {url}: {e}"))
        })?;

        if !response.status().is_success() {
            return Err(SchemaError::InvalidData(format!(
                "Model download failed with status {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to read model bytes: {e}")))?;

        std::fs::write(dest, &bytes).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to write model to {}: {e}", dest.display()))
        })?;

        log::info!("Model saved to {}", dest.display());
        Ok(())
    }
}
