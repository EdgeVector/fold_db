//! Shared smart-folder scan and ingestion logic.
//!
//! These functions are framework-agnostic and used by both
//! HTTP handlers (`routes.rs`) and the CLI (`folddb`).

use crate::ingestion::IngestionResult;
use crate::log_feature;
use crate::logging::features::LogFeature;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// Re-export from sibling modules so external callers can still use
// `smart_folder::read_file_as_json`, etc.
pub use super::file_conversion::{csv_to_json, read_file_as_json, read_file_with_hash, twitter_js_to_json};
pub use super::smart_folder_scanner::*;
pub use super::smart_folder_classifier::*;

// ---- Cost estimation ----

/// Estimate the ingestion cost for a single file based on its size and type.
///
/// The model accounts for multiple AI calls per file (classification, conversion,
/// schema recommendation, child schema resolution) plus a base schema-service call.
pub fn estimate_file_cost(path: &Path, root: &Path) -> f64 {
    let full_path = root.join(path);
    let file_size = std::fs::metadata(&full_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Base cost for schema recommendation call
    let base_cost = 0.003;

    let content_cost = match ext.as_str() {
        // PDF: text extraction + conversion
        "pdf" => {
            let text_cost = text_cost_by_size(file_size);
            0.04 + text_cost
        }
        // Images: vision model call
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "heif" | "bmp" | "tiff" => 0.02,
        // Text-like files: cost scales with size
        _ => text_cost_by_size(file_size),
    };

    base_cost + content_cost
}

/// Helper: estimate the AI cost for text content based on byte size.
fn text_cost_by_size(size: u64) -> f64 {
    if size < 10_000 {
        0.005
    } else if size < 100_000 {
        0.015
    } else {
        0.028
    }
}

/// Get the file size for a path relative to root, returning 0 on error.
pub(crate) fn file_size_bytes(path: &Path, root: &Path) -> u64 {
    let full_path = root.join(path);
    std::fs::metadata(&full_path)
        .map(|m| m.len())
        .unwrap_or(0)
}

// ---- Data types ----

/// A file recommendation from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecommendation {
    /// File path relative to the scanned folder
    pub path: String,
    /// Whether the file should be ingested
    pub should_ingest: bool,
    /// Category: "personal_data", "media", "config", "website_scaffolding", "work", "unknown"
    pub category: String,
    /// Brief reason for the recommendation
    pub reason: String,
    /// Size of the file in bytes (populated during scan)
    #[serde(default)]
    pub file_size_bytes: u64,
    /// Estimated ingestion cost in USD
    #[serde(default)]
    pub estimated_cost: f64,
    /// Whether this file has already been ingested (dedup check)
    #[serde(default)]
    pub already_ingested: bool,
}

/// Summary of smart folder scan — category name → count.
/// Serializes as a flat JSON object like `{"personal_data": 5, "media": 3}`.
pub type SmartFolderSummary = HashMap<String, usize>;

/// Response from smart folder scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFolderScanResponse {
    pub success: bool,
    /// Total files scanned
    pub total_files: usize,
    /// Files recommended for ingestion
    pub recommended_files: Vec<FileRecommendation>,
    /// Files recommended to skip
    pub skipped_files: Vec<FileRecommendation>,
    /// Summary statistics
    pub summary: SmartFolderSummary,
    /// Total estimated cost for all recommended files
    #[serde(default)]
    pub total_estimated_cost: f64,
    /// Whether the scan was truncated due to reaching max_files
    #[serde(default)]
    pub scan_truncated: bool,
    /// The max_depth value used for this scan
    #[serde(default)]
    pub max_depth_used: usize,
    /// The max_files value used for this scan
    #[serde(default)]
    pub max_files_used: usize,
}

// (Scanning functions extracted to smart_folder_scanner.rs)
// (Classification functions extracted to smart_folder_classifier.rs)

// ---- Scan orchestration ----

/// Optional progress reporter for scan operations.
/// Accepts `(percentage, message)` updates.
pub type ScanProgressFn = Box<dyn Fn(u8, String) + Send + Sync>;

/// Perform a smart folder scan: directory walk, LLM classification, recommendations.
///
/// This is the core logic shared between the HTTP handler and the CLI.
/// If `service` is `None`, an `IngestionService` is created from the environment.
pub async fn perform_smart_folder_scan(
    folder_path: &Path,
    max_depth: usize,
    max_files: usize,
    service: Option<&crate::ingestion::ingestion_service::IngestionService>,
    node: Option<&crate::fold_node::FoldNode>,
) -> IngestionResult<SmartFolderScanResponse> {
    perform_smart_folder_scan_with_progress(folder_path, max_depth, max_files, service, node, None)
        .await
}

pub async fn perform_smart_folder_scan_with_progress(
    folder_path: &Path,
    max_depth: usize,
    max_files: usize,
    service: Option<&crate::ingestion::ingestion_service::IngestionService>,
    node: Option<&crate::fold_node::FoldNode>,
    on_progress: Option<&ScanProgressFn>,
) -> IngestionResult<SmartFolderScanResponse> {
    let report = |pct: u8, msg: String| {
        if let Some(f) = &on_progress {
            f(pct, msg);
        }
    };

    report(5, "Listing files...".into());
    let scan = scan_directory_tree_with_context(folder_path, max_depth, max_files)?;

    if scan.file_paths.is_empty() {
        report(100, "No files found.".into());
        return Ok(SmartFolderScanResponse {
            success: true,
            total_files: 0,
            recommended_files: vec![],
            skipped_files: vec![],
            summary: HashMap::new(),
            total_estimated_cost: 0.0,
            scan_truncated: scan.truncated,
            max_depth_used: max_depth,
            max_files_used: max_files,
        });
    }

    report(15, format!("Found {} candidate files (binary/media already excluded).", scan.file_paths.len()));

    // Binary and media files are filtered out during directory collection so they
    // do not consume the max_files budget.  All paths that reach here are
    // ingestible candidates; send them all to the LLM for classification.
    let binary_skipped: Vec<FileRecommendation> = Vec::new();
    let mut llm_candidates: Vec<String> = scan.file_paths.clone();

    log_feature!(
        LogFeature::Ingestion,
        info,
        "File classification: {} candidates for dedup check",
        llm_candidates.len(),
    );

    // --- Dedup check: remove already-ingested files before AI classification ---
    let pub_key = node.map(|n| n.get_node_public_key().to_string());
    let mut already_ingested_recs: Vec<FileRecommendation> = Vec::new();

    if let (Some(ref pk), Some(n)) = (&pub_key, node) {
        report(20, format!(
            "Skipped {} binary files. Checking {} files for previously ingested (concurrent)...",
            binary_skipped.len(),
            llm_candidates.len(),
        ));

        // Check dedup concurrently — up to 16 at a time (mixed CPU hash + async DB lookup)
        let dedup_results: Vec<(String, bool, u64)> = stream::iter(llm_candidates)
            .map(|path| async {
                let full_path = folder_path.join(&path);
                if let Ok(hash) = compute_file_hash(&full_path) {
                    if n.is_file_ingested(pk, &hash).await.is_some() {
                        let size = file_size_bytes(Path::new(&path), folder_path);
                        return (path, true, size);
                    }
                }
                (path, false, 0)
            })
            .buffer_unordered(16)
            .collect()
            .await;

        let mut remaining = Vec::new();
        for (path, ingested, size) in dedup_results {
            if ingested {
                already_ingested_recs.push(FileRecommendation {
                    path,
                    should_ingest: false,
                    category: "already_ingested".to_string(),
                    reason: "Already ingested".to_string(),
                    file_size_bytes: size,
                    estimated_cost: 0.0,
                    already_ingested: true,
                });
            } else {
                remaining.push(path);
            }
        }
        llm_candidates = remaining;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Dedup check: {} already ingested, {} remaining for LLM",
            already_ingested_recs.len(),
            llm_candidates.len(),
        );
    }

    report(25, format!(
        "Classifying {} files with AI ({} already ingested, {} binary-skipped)...",
        llm_candidates.len(),
        already_ingested_recs.len(),
        binary_skipped.len(),
    ));

    // Send remaining non-binary, non-ingested files to LLM in batches (with tree context)
    let llm_recs = if llm_candidates.is_empty() {
        Vec::new()
    } else {
        // Create service from env if not provided
        let owned_service;
        let svc = match service {
            Some(s) => s,
            None => {
                owned_service = crate::ingestion::ingestion_service::IngestionService::from_env()?;
                &owned_service
            }
        };

        let batch_size = 100;
        let chunks: Vec<Vec<String>> = llm_candidates.chunks(batch_size).map(|c| c.to_vec()).collect();
        let total_batches = chunks.len();

        if total_batches > 1 {
            report(25, format!(
                "Classifying files with AI ({} batches, up to 4 concurrent)...",
                total_batches,
            ));
        }

        // Run LLM classification batches concurrently — up to 4 at a time (API rate limits)
        let tree_display = &scan.tree_display;
        let batch_results: Vec<Vec<FileRecommendation>> = stream::iter(chunks.into_iter().enumerate())
            .map(|(i, chunk_vec)| async move {
                let prompt = create_smart_folder_prompt(tree_display, &chunk_vec);
                match call_llm_for_file_analysis(&prompt, svc).await {
                    Ok(llm_response) => {
                        parse_llm_file_recommendations(&llm_response, &chunk_vec)
                            .unwrap_or_else(|e| {
                                log::warn!(
                                    "Failed to parse LLM response for batch {}: {}",
                                    i, e
                                );
                                apply_heuristic_filtering(&chunk_vec)
                            })
                    }
                    Err(e) => {
                        log::warn!("LLM call failed for batch {}: {}", i, e);
                        apply_heuristic_filtering(&chunk_vec)
                    }
                }
            })
            .buffer_unordered(4)
            .collect()
            .await;

        batch_results.into_iter().flatten().collect()
    };

    report(80, "Computing costs and finalizing...".into());

    // Merge binary-skipped + LLM recommendations (already-ingested handled separately)
    let recommendations: Vec<FileRecommendation> =
        binary_skipped.into_iter().chain(llm_recs).collect();

    // Split into recommended and skipped, build summary, compute costs
    let mut recommended_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut total_estimated_cost = 0.0;
    let mut summary: SmartFolderSummary = HashMap::new();

    // Add already-ingested files to skipped list and summary
    if !already_ingested_recs.is_empty() {
        *summary.entry("already_ingested".to_string()).or_insert(0) += already_ingested_recs.len();
        skipped_files.extend(already_ingested_recs);
    }

    let rec_count = recommendations.len();
    for (idx, mut rec) in recommendations.into_iter().enumerate() {
        // Report incremental progress every 5 files (80% → 95%)
        if rec_count > 0 && idx % 5 == 0 {
            let pct = (80 + idx * 15 / rec_count).min(95) as u8;
            report(pct, format!("Computing costs ({}/{})...", idx, rec_count));
        }

        // Populate file size and cost estimate (local providers are free)
        let rel_path = Path::new(&rec.path);
        rec.file_size_bytes = file_size_bytes(rel_path, folder_path);
        let is_local = service.is_some_and(|s| s.is_local_provider());
        rec.estimated_cost = if is_local {
            0.0
        } else {
            estimate_file_cost(rel_path, folder_path)
        };

        if rec.should_ingest {
            *summary.entry(rec.category.clone()).or_insert(0) += 1;
            total_estimated_cost += rec.estimated_cost;
            recommended_files.push(rec);
        } else {
            *summary.entry(rec.category.clone()).or_insert(0) += 1;
            skipped_files.push(rec);
        }
    }

    // Don't report 100% here — the caller sets JobStatus::Completed after we return.
    // Reporting 100% via the fire-and-forget spawned callback races with the
    // caller's completion save and can overwrite Completed back to Running.
    report(99, format!(
        "Finalizing... {} to ingest, {} skipped.",
        recommended_files.len(),
        skipped_files.len(),
    ));

    Ok(SmartFolderScanResponse {
        success: true,
        total_files: scan.file_paths.len(),
        recommended_files,
        skipped_files,
        summary,
        total_estimated_cost,
        scan_truncated: scan.truncated,
        max_depth_used: max_depth,
        max_files_used: max_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_twitter_js_to_json_valid() {
        let input = r#"window.YTD.tweet.part0 = [{"id":"123","text":"hello"}]"#;
        let result = twitter_js_to_json(input).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed[0]["id"], "123");
    }

    #[test]
    fn test_twitter_js_to_json_no_equals() {
        let input = r#"{"id":"123"}"#;
        let result = twitter_js_to_json(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_twitter_js_to_json_invalid_json() {
        let input = "window.YTD.tweet.part0 = not valid json";
        let result = twitter_js_to_json(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_csv_to_json_basic() {
        let csv_content = "name,age,active\nAlice,30,true\nBob,25,false";
        let result = csv_to_json(csv_content).unwrap();
        let parsed: Vec<Value> = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["name"], "Alice");
        assert_eq!(parsed[0]["age"], 30.0);
        assert_eq!(parsed[0]["active"], true);
        assert_eq!(parsed[1]["name"], "Bob");
        assert_eq!(parsed[1]["age"], 25.0);
        assert_eq!(parsed[1]["active"], false);
    }

    #[test]
    fn test_csv_to_json_empty() {
        let csv_content = "name,age";
        let result = csv_to_json(csv_content).unwrap();
        let parsed: Vec<Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 0);
    }

    #[test]
    fn test_extract_json_direct_array() {
        let input = r#"[{"path":"a.json","should_ingest":true,"category":"personal_data","reason":"test"}]"#;
        let result = crate::ingestion::ai_helpers::extract_json_from_response(input).unwrap();
        assert!(result.starts_with('['));
    }

    #[test]
    fn test_extract_json_from_markdown() {
        let input = "Here is the result:\n```json\n[{\"path\":\"a.json\"}]\n```\nDone.";
        let result = crate::ingestion::ai_helpers::extract_json_from_response(input).unwrap();
        assert!(result.starts_with('['));
    }

    #[test]
    fn test_heuristic_filtering_personal_doc() {
        let files = vec!["reports/q1.pdf".to_string()];
        let recs = apply_heuristic_filtering(&files);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].should_ingest);
        assert_eq!(recs[0].category, "personal_data");
    }

    #[test]
    fn test_heuristic_filtering_data_export() {
        let files = vec!["data/export.json".to_string()];
        let recs = apply_heuristic_filtering(&files);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].should_ingest);
        assert_eq!(recs[0].category, "personal_data");
    }

    #[test]
    fn test_heuristic_filtering_media_without_context() {
        // Without LLM, media files default to should_ingest=false (conservative)
        let files = vec!["photos/vacation.jpg".to_string()];
        let recs = apply_heuristic_filtering(&files);
        assert_eq!(recs.len(), 1);
        assert!(!recs[0].should_ingest);
        assert_eq!(recs[0].category, "media");
    }

    #[test]
    fn test_heuristic_filtering_media_in_export() {
        // Media in data export paths should be ingested
        let files = vec!["export/photos/vacation.jpg".to_string()];
        let recs = apply_heuristic_filtering(&files);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].should_ingest);
        assert_eq!(recs[0].category, "media");
    }

    #[test]
    fn test_heuristic_filtering_unknown_file() {
        let files = vec!["random/stuff.xyz".to_string()];
        let recs = apply_heuristic_filtering(&files);
        assert_eq!(recs.len(), 1);
        assert!(!recs[0].should_ingest);
    }

    #[test]
    fn test_read_file_as_json_unsupported() {
        let result = read_file_as_json(Path::new("/tmp/test.xyz"));
        assert!(result.is_err());
    }

    // ---- is_never_personal_data tests ----

    #[test]
    fn test_binary_files_are_never_personal() {
        assert!(is_never_personal_data("program.exe"));
        assert!(is_never_personal_data("lib/native.so"));
        assert!(is_never_personal_data("module.dll"));
        assert!(is_never_personal_data("code.class"));
        assert!(is_never_personal_data("script.pyc"));
        assert!(is_never_personal_data("app.wasm"));
    }

    #[test]
    fn test_fonts_are_never_personal() {
        assert!(is_never_personal_data("font.woff"));
        assert!(is_never_personal_data("font.woff2"));
        assert!(is_never_personal_data("font.ttf"));
        assert!(is_never_personal_data("font.otf"));
        assert!(is_never_personal_data("font.eot"));
    }

    #[test]
    fn test_lock_and_map_are_never_personal() {
        assert!(is_never_personal_data("package-lock.lock"));
        assert!(is_never_personal_data("bundle.map"));
    }

    #[test]
    fn test_media_files_are_never_personal() {
        // Images, video, and audio cannot be ingested as structured data and
        // are filtered during directory collection so they don't exhaust max_files.
        assert!(is_never_personal_data("photo.jpg"));
        assert!(is_never_personal_data("photo.jpeg"));
        assert!(is_never_personal_data("image.png"));
        assert!(is_never_personal_data("image.gif"));
        assert!(is_never_personal_data("image.webp"));
        assert!(is_never_personal_data("icon.svg"));
        assert!(is_never_personal_data("song.mp3"));
        assert!(is_never_personal_data("audio.wav"));
        assert!(is_never_personal_data("video.mp4"));
        assert!(is_never_personal_data("clip.mov"));
    }

    #[test]
    fn test_ingestible_files_go_to_llm() {
        // These should NOT be auto-skipped — they go to the LLM for classification
        // because read_file_with_hash can handle them.
        assert!(!is_never_personal_data("data.json"));
        assert!(!is_never_personal_data("notes.txt"));
        assert!(!is_never_personal_data("readme.md"));
        assert!(!is_never_personal_data("script.js"));
        assert!(!is_never_personal_data("records.csv"));
    }

    #[test]
    fn test_non_ingestible_non_media_go_to_llm() {
        // `is_never_personal_data` is a **scanner-phase** gate: it only skips
        // truly binary/media formats (images, video, compiled objects, etc.)
        // so they don't consume the `max_files` budget.
        //
        // These file types are intentionally NOT pre-filtered here because:
        //   - Some (.css, .html, .py, .yaml) are plain text that
        //     `read_file_with_hash` can read, and may contain personal data.
        //   - Others (.pdf, .zip, .xlsx) can't be ingested yet, but the
        //     LLM classifier — not the scanner — decides whether to recommend
        //     them.  If the LLM recommends them, ingestion will fail
        //     gracefully at the file-read stage.
        assert!(!is_never_personal_data("document.pdf"));
        assert!(!is_never_personal_data("style.css"));
        assert!(!is_never_personal_data("page.html"));
        assert!(!is_never_personal_data("code.py"));
        assert!(!is_never_personal_data("archive.zip"));
        assert!(!is_never_personal_data("spreadsheet.xlsx"));
        assert!(!is_never_personal_data("config.yaml"));
    }

    #[test]
    fn test_no_extension_goes_to_llm() {
        assert!(!is_never_personal_data("README"));
        assert!(!is_never_personal_data("Makefile"));
    }

    // ---- build_directory_tree_string tests ----

    #[test]
    fn test_tree_string_flat_files() {
        let paths = vec!["a.txt".to_string(), "b.pdf".to_string()];
        let tree = build_directory_tree_string(&paths);
        assert!(tree.contains("a.txt"));
        assert!(tree.contains("b.pdf"));
    }

    #[test]
    fn test_tree_string_nested_dirs() {
        let paths = vec![
            "Photos/vacation/IMG_001.jpg".to_string(),
            "Photos/vacation/IMG_002.jpg".to_string(),
            "Bank of America/statement.pdf".to_string(),
        ];
        let tree = build_directory_tree_string(&paths);
        assert!(tree.contains("Photos/"));
        assert!(tree.contains("vacation/"));
        assert!(tree.contains("IMG_001.jpg"));
        assert!(tree.contains("Bank of America/"));
        assert!(tree.contains("statement.pdf"));
    }

    #[test]
    fn test_tree_string_empty() {
        let paths: Vec<String> = vec![];
        let tree = build_directory_tree_string(&paths);
        assert!(tree.is_empty());
    }

    // ---- scan_directory_tree_with_context tests ----

    #[test]
    fn test_scan_with_context_returns_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs/notes.txt"), "hello").unwrap();
        // photo.jpg is filtered during collection — it must NOT consume max_files budget
        std::fs::write(root.join("photo.jpg"), "fake jpg").unwrap();

        let result = scan_directory_tree_with_context(root, 10, 50000).unwrap();

        // Only the ingestible file appears in file_paths; the jpg is silently excluded
        assert_eq!(result.file_paths.len(), 1);
        assert!(result.file_paths.contains(&"docs/notes.txt".to_string()));
        assert!(!result.truncated);
        assert!(!result.tree_display.is_empty());
        assert!(result.tree_display.contains("docs/"));
    }

    #[test]
    fn test_scan_with_context_truncation() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        for i in 0..5 {
            std::fs::write(root.join(format!("file_{}.txt", i)), "data").unwrap();
        }

        let result = scan_directory_tree_with_context(root, 10, 3).unwrap();

        assert_eq!(result.file_paths.len(), 3);
        assert!(result.truncated);
    }

    // ---- scan_directory_recursive tests ----

    #[test]
    fn test_scan_skips_git_repo_subdirectory() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Create a subdirectory that looks like a git repo
        let repo_dir = root.join("my_project");
        std::fs::create_dir_all(repo_dir.join(".git")).unwrap();
        std::fs::write(repo_dir.join("main.rs"), "fn main() {}").unwrap();

        // Create a normal file at the root level
        std::fs::write(root.join("notes.txt"), "hello").unwrap();

        let files = scan_directory_tree(root, 10, 50000).unwrap();

        // Should find notes.txt but NOT my_project/main.rs
        assert!(files.contains(&"notes.txt".to_string()));
        assert!(!files.iter().any(|f| f.contains("main.rs")));
    }

    #[test]
    fn test_scan_does_not_skip_root_git_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // The root itself is a git repo
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("readme.md"), "# Hello").unwrap();

        let files = scan_directory_tree(root, 10, 50000).unwrap();

        // Should still find files in the root even though it has .git
        assert!(files.contains(&"readme.md".to_string()));
    }

    #[test]
    fn test_scan_skips_coding_projects() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Node.js project (package.json)
        let node_dir = root.join("my_website");
        std::fs::create_dir_all(&node_dir).unwrap();
        std::fs::write(node_dir.join("package.json"), r#"{"name":"test"}"#).unwrap();
        std::fs::write(node_dir.join("index.js"), "console.log('hi')").unwrap();

        // Rust project (Cargo.toml)
        let rust_dir = root.join("rust_cli");
        std::fs::create_dir_all(rust_dir.join("src")).unwrap();
        std::fs::write(rust_dir.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        std::fs::write(rust_dir.join("src/main.rs"), "fn main() {}").unwrap();

        // Python project (pyproject.toml)
        let py_dir = root.join("data_analysis");
        std::fs::create_dir_all(&py_dir).unwrap();
        std::fs::write(py_dir.join("pyproject.toml"), "[project]\nname = \"test\"").unwrap();
        std::fs::write(py_dir.join("analysis.py"), "import pandas").unwrap();

        // Normal personal file
        std::fs::write(root.join("notes.txt"), "my notes").unwrap();

        let files = scan_directory_tree(root, 10, 50000).unwrap();

        // Should find the personal file but none of the coding project files
        assert!(files.contains(&"notes.txt".to_string()));
        assert!(!files.iter().any(|f| f.contains("index.js")));
        assert!(!files.iter().any(|f| f.contains("main.rs")));
        assert!(!files.iter().any(|f| f.contains("analysis.py")));
        assert!(!files.iter().any(|f| f.contains("package.json")));
        assert!(!files.iter().any(|f| f.contains("Cargo.toml")));
        assert!(!files.iter().any(|f| f.contains("pyproject.toml")));
    }

    #[test]
    fn test_scan_does_not_skip_root_coding_project() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // The root itself contains a package.json
        std::fs::write(root.join("package.json"), r#"{"name":"root"}"#).unwrap();
        std::fs::write(root.join("readme.md"), "# Hello").unwrap();

        let files = scan_directory_tree(root, 10, 50000).unwrap();

        // Should still find files in the root even though it has a manifest
        assert!(files.contains(&"readme.md".to_string()));
        assert!(files.contains(&"package.json".to_string()));
    }

    #[test]
    fn test_scan_skips_expanded_skip_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Create directories from the expanded skip list
        for dir_name in &[".idea", ".vscode", "Pods", "DerivedData", "vendor", ".next"] {
            let dir = root.join(dir_name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("junk.txt"), "junk").unwrap();
        }

        // Create a normal file
        std::fs::write(root.join("personal.txt"), "my data").unwrap();

        let files = scan_directory_tree(root, 10, 50000).unwrap();

        // Should only find the normal file
        assert_eq!(files, vec!["personal.txt".to_string()]);
    }
}
