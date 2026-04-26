//! Build script for `fold_db`.
//!
//! When the `face-detection` feature is on, this script fetches the
//! InsightFace `buffalo_sc` model pack at build time, extracts the two
//! ONNX files the runtime uses (SCRFD detector + ArcFace embedder),
//! and drops them in `OUT_DIR` so the face-detection module embeds
//! them via `include_bytes!`. Prior to this, the runtime code pulled
//! the same zip from GitHub on first use — which broke the E2E harness
//! (90-second budget vs download + extract + detect) and penalized
//! every fresh deploy and every ephemeral test environment.
//!
//! Files land in `OUT_DIR` so they never enter the source tree or the
//! git repo. Cargo caches `OUT_DIR` between incremental builds, so
//! repeat builds don't re-download. `cargo:rerun-if-env-changed` lets
//! callers pin a different URL (e.g. a local mirror) via
//! `FOLDDB_FACE_MODEL_PACK_URL` without editing this file.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(feature = "face-detection")]
    face_detection::fetch_and_extract_models();
}

#[cfg(feature = "face-detection")]
mod face_detection {
    use std::io::Read;
    use std::path::PathBuf;

    const DEFAULT_MODEL_PACK_URL: &str =
        "https://github.com/deepinsight/insightface/releases/download/v0.7/buffalo_sc.zip";

    // Names inside the InsightFace buffalo_sc zip.
    const SCRFD_ZIP_NAME: &str = "det_500m.onnx";
    const ARCFACE_ZIP_NAME: &str = "w600k_mbf.onnx";

    // Output file names the runtime `ModelManager` reads via `include_bytes!`.
    // Kept aligned with the filenames the pre-bundling runtime wrote to disk
    // so log lines and external docs stay accurate.
    const SCRFD_OUT_NAME: &str = "scrfd_2.5g_bnkps.onnx";
    const ARCFACE_OUT_NAME: &str = "arcface_r100.onnx";

    pub fn fetch_and_extract_models() {
        println!("cargo:rerun-if-env-changed=FOLDDB_FACE_MODEL_PACK_URL");

        let out_dir =
            PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR set by cargo in build.rs"));
        let scrfd_out = out_dir.join(SCRFD_OUT_NAME);
        let arcface_out = out_dir.join(ARCFACE_OUT_NAME);

        // Cache: if both files are already in OUT_DIR from a previous build,
        // skip the download entirely. That's what keeps incremental rebuilds
        // fast even with `face-detection` on.
        if scrfd_out.exists() && arcface_out.exists() {
            return;
        }

        let url = std::env::var("FOLDDB_FACE_MODEL_PACK_URL")
            .unwrap_or_else(|_| DEFAULT_MODEL_PACK_URL.to_string());
        println!(
            "cargo:warning=fold_db build.rs: fetching face-detection models from {url} (~15MB)"
        );

        let response = ureq::get(&url)
            .call()
            .unwrap_or_else(|e| panic!("fold_db build.rs: failed to GET {url}: {e}"));
        let mut zip_bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut zip_bytes)
            .expect("fold_db build.rs: failed to read zip body");

        let cursor = std::io::Cursor::new(zip_bytes);
        let mut archive =
            zip::ZipArchive::new(cursor).expect("fold_db build.rs: failed to parse buffalo_sc.zip");

        let mut wrote_scrfd = false;
        let mut wrote_arcface = false;
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .expect("fold_db build.rs: failed to read zip entry");
            let name = entry.name().to_string();
            let dest = if name.contains(SCRFD_ZIP_NAME) || name.ends_with(SCRFD_ZIP_NAME) {
                wrote_scrfd = true;
                &scrfd_out
            } else if name.contains(ARCFACE_ZIP_NAME) || name.ends_with(ARCFACE_ZIP_NAME) {
                wrote_arcface = true;
                &arcface_out
            } else {
                continue;
            };
            let mut contents = Vec::new();
            entry
                .read_to_end(&mut contents)
                .expect("fold_db build.rs: failed to extract entry");
            std::fs::write(dest, &contents)
                .unwrap_or_else(|e| panic!("fold_db build.rs: failed to write {dest:?}: {e}"));
        }

        assert!(
            wrote_scrfd,
            "fold_db build.rs: {SCRFD_ZIP_NAME} not found in {url}"
        );
        assert!(
            wrote_arcface,
            "fold_db build.rs: {ARCFACE_ZIP_NAME} not found in {url}"
        );
    }
}
