//! Model manager for the face-detection pipeline.
//!
//! Exposes paths to two ONNX models — SCRFD (detector) and ArcFace
//! (embedder). The bytes are embedded in the binary at compile time
//! (see `build.rs`, which downloads the InsightFace `buffalo_sc` pack
//! and drops the extracted files in `OUT_DIR`). At runtime we materialize
//! them to a per-`FOLDDB_HOME` directory the first time they're
//! requested, because `ort` (ONNX Runtime) takes a filesystem path, not
//! a byte slice.
//!
//! Why bundle instead of download-on-first-use:
//! - The previous implementation pulled the pack from GitHub on demand,
//!   which broke the E2E harness (90-second timeout vs. network fetch +
//!   extract + model load) and penalized every ephemeral environment.
//! - The E2E symptom: `ERROR: ingestion wrote 1 records but face
//!   detection produced 0 faces after 90s`. See CI run 24618304745.
//! - The bundled bytes add ~15MB to the binary, which is immaterial
//!   against the `folddb_server` debug binary size.

use std::path::{Path, PathBuf};

use crate::schema::SchemaError;

const MODELS_DIR: &str = "models";

const SCRFD_FILENAME: &str = "scrfd_2.5g_bnkps.onnx";
const ARCFACE_FILENAME: &str = "arcface_r100.onnx";

/// SCRFD detector, embedded at compile time by `build.rs`.
const SCRFD_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/scrfd_2.5g_bnkps.onnx"));
/// ArcFace embedder, embedded at compile time by `build.rs`.
const ARCFACE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/arcface_r100.onnx"));

pub struct ModelManager {
    models_dir: PathBuf,
}

impl ModelManager {
    pub fn new(folddb_home: &Path) -> Self {
        Self {
            models_dir: folddb_home.join(MODELS_DIR),
        }
    }

    /// Path to the SCRFD detector ONNX file. Materializes the embedded
    /// bytes to `<folddb_home>/models/scrfd_2.5g_bnkps.onnx` on first
    /// call and returns that path on subsequent calls.
    pub fn scrfd_path(&self) -> Result<PathBuf, SchemaError> {
        self.ensure_extracted(SCRFD_FILENAME, SCRFD_BYTES)
    }

    /// Path to the ArcFace embedder ONNX file. Same materialization
    /// contract as [`scrfd_path`].
    pub fn arcface_path(&self) -> Result<PathBuf, SchemaError> {
        self.ensure_extracted(ARCFACE_FILENAME, ARCFACE_BYTES)
    }

    /// Ensure the given model file exists on disk, writing the embedded
    /// bytes if it's missing. Idempotent.
    fn ensure_extracted(&self, filename: &str, bytes: &[u8]) -> Result<PathBuf, SchemaError> {
        let dest = self.models_dir.join(filename);
        if dest.exists() {
            return Ok(dest);
        }

        std::fs::create_dir_all(&self.models_dir).map_err(|e| {
            SchemaError::InvalidData(format!(
                "Failed to create models directory {:?}: {e}",
                self.models_dir
            ))
        })?;

        // Write atomically so a concurrent reader can't observe a
        // half-written ONNX file. Rust's `std::fs::write` is not
        // atomic, but a rename from a sibling temp file is on the same
        // filesystem.
        let tmp = self.models_dir.join(format!(".{filename}.tmp"));
        std::fs::write(&tmp, bytes)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to write {tmp:?}: {e}")))?;
        std::fs::rename(&tmp, &dest).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to rename {tmp:?} -> {dest:?}: {e}"))
        })?;
        log::info!(
            "Extracted bundled face model {} ({} bytes)",
            dest.display(),
            bytes.len()
        );
        Ok(dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_bytes_are_nonempty_and_match_expected_sizes() {
        // buffalo_sc v0.7: det_500m.onnx ≈ 2.5MB, w600k_mbf.onnx ≈ 13MB.
        // Exact sizes can shift if InsightFace re-publishes the release,
        // so we only sanity-check orders of magnitude.
        assert!(
            SCRFD_BYTES.len() > 1_000_000,
            "SCRFD bytes suspiciously small: {}",
            SCRFD_BYTES.len()
        );
        assert!(
            ARCFACE_BYTES.len() > 5_000_000,
            "ArcFace bytes suspiciously small: {}",
            ARCFACE_BYTES.len()
        );
    }

    #[test]
    fn ensure_extracted_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ModelManager::new(tmp.path());
        let p1 = mgr.scrfd_path().unwrap();
        let p2 = mgr.scrfd_path().unwrap();
        assert_eq!(p1, p2);
        assert!(p1.exists());
        assert_eq!(
            std::fs::metadata(&p1).unwrap().len() as usize,
            SCRFD_BYTES.len()
        );
    }

    #[test]
    fn both_model_files_extracted_to_expected_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ModelManager::new(tmp.path());
        let scrfd = mgr.scrfd_path().unwrap();
        let arcface = mgr.arcface_path().unwrap();
        assert!(scrfd.ends_with("models/scrfd_2.5g_bnkps.onnx"));
        assert!(arcface.ends_with("models/arcface_r100.onnx"));
        assert!(scrfd.exists());
        assert!(arcface.exists());
    }
}
