//! Shared smart-folder scan and ingestion logic.
//!
//! These functions are framework-agnostic and used by both
//! HTTP handlers (`routes.rs`) and the CLI (`folddb`).

use crate::ingestion::error::IngestionError;
use crate::ingestion::IngestionResult;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

// Re-export from sibling modules so external callers can still use
// `smart_folder::estimate_file_cost`, `smart_folder::read_file_as_json`, etc.
pub use super::cost_estimation::estimate_file_cost;
pub use super::file_conversion::{csv_to_json, read_file_as_json, read_file_with_hash, twitter_js_to_json};

use super::cost_estimation::file_size_bytes;

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

/// Result of scanning a directory tree with context for LLM classification.
pub struct DirectoryScanResult {
    /// Flat list of relative file paths for processing
    pub file_paths: Vec<String>,
    /// Indented tree display for LLM context
    pub tree_display: String,
    /// Whether the scan was truncated due to reaching max_files
    pub truncated: bool,
}

// ---- Directory scanning ----

/// Recursively scan a directory tree up to max_depth, returning both
/// a flat file list and an indented tree string for LLM context.
pub fn scan_directory_tree_with_context(
    root: &Path,
    max_depth: usize,
    max_files: usize,
) -> IngestionResult<DirectoryScanResult> {
    let mut files = Vec::new();
    scan_directory_recursive(root, root, 0, max_depth, max_files, &mut files)?;
    let truncated = files.len() >= max_files;
    let tree_display = build_directory_tree_string(&files);
    Ok(DirectoryScanResult {
        file_paths: files,
        tree_display,
        truncated,
    })
}

/// Recursively scan a directory tree up to max_depth (flat list only).
pub fn scan_directory_tree(
    root: &Path,
    max_depth: usize,
    max_files: usize,
) -> IngestionResult<Vec<String>> {
    let mut files = Vec::new();
    scan_directory_recursive(root, root, 0, max_depth, max_files, &mut files)?;
    Ok(files)
}

fn scan_directory_recursive(
    root: &Path,
    current: &Path,
    depth: usize,
    max_depth: usize,
    max_files: usize,
    files: &mut Vec<String>,
) -> IngestionResult<()> {
    if depth > max_depth || files.len() >= max_files {
        return Ok(());
    }

    // Skip non-root directories that are git repos (code repositories)
    if current != root && current.join(".git").exists() {
        return Ok(());
    }

    let entries = std::fs::read_dir(current).map_err(|e| {
        IngestionError::InvalidInput(format!(
            "Failed to read directory {}: {}",
            current.display(),
            e
        ))
    })?;

    for entry in entries.flatten() {
        if files.len() >= max_files {
            break;
        }

        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip hidden files and common skip patterns
        if file_name.starts_with('.') {
            continue;
        }

        // Skip common non-data directories
        let skip_dirs = [
            "node_modules",
            "__pycache__",
            ".git",
            ".svn",
            "target",
            "build",
            "dist",
            ".cache",
            "venv",
            ".venv",
            ".idea",
            ".vscode",
            "Pods",
            ".gradle",
            "vendor",
            "cmake-build-debug",
            "cmake-build-release",
            ".terraform",
            ".next",
            ".nuxt",
            "__MACOSX",
            ".tox",
            ".eggs",
            ".mypy_cache",
            ".pytest_cache",
            ".cargo",
            "bower_components",
            ".bundle",
            "DerivedData",
            "_build",
            "deps",
            "artifacts",
            "cache",
        ];
        if path.is_dir() && skip_dirs.contains(&file_name) {
            continue;
        }

        if path.is_dir() {
            scan_directory_recursive(root, &path, depth + 1, max_depth, max_files, files)?;
        } else if path.is_file() {
            // Get relative path from root
            if let Ok(relative) = path.strip_prefix(root) {
                files.push(relative.to_string_lossy().to_string());
            }
        }
    }

    Ok(())
}

/// Build an indented directory tree string from a list of relative file paths.
///
/// Example output:
/// ```text
/// Photos/
///   vacation_2024/
///     IMG_001.jpg
/// Bank of America/
///   Statements/
///     statement.pdf
///     ajax-loader.gif
/// ```
pub fn build_directory_tree_string(file_paths: &[String]) -> String {
    use std::collections::BTreeSet;

    // Collect all directory prefixes and files in sorted order
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    let mut all_paths: BTreeSet<String> = BTreeSet::new();

    for path in file_paths {
        all_paths.insert(path.clone());
        let p = Path::new(path);
        // Collect all ancestor directories
        let mut ancestor = p.parent();
        while let Some(dir) = ancestor {
            let dir_str = dir.to_string_lossy().to_string();
            if dir_str.is_empty() {
                break;
            }
            dirs.insert(dir_str);
            ancestor = dir.parent();
        }
    }

    let mut lines = Vec::new();

    // Merge dirs and files into a sorted list of entries
    // We want to show directories with trailing "/" followed by their contents
    let mut entries: Vec<(String, bool)> = Vec::new(); // (path, is_dir)
    for d in &dirs {
        entries.push((d.clone(), true));
    }
    for f in &all_paths {
        entries.push((f.clone(), false));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // Track which directories we've already printed
    let mut printed_dirs: HashSet<String> = HashSet::new();

    for (path, is_dir) in &entries {
        let depth = path.matches('/').count();
        let indent = "  ".repeat(depth);
        if *is_dir {
            if !printed_dirs.contains(path) {
                let name = Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path);
                lines.push(format!("{}{}/", indent, name));
                printed_dirs.insert(path.clone());
            }
        } else {
            let name = Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            lines.push(format!("{}{}", indent, name));
        }
    }

    lines.join("\n")
}

// ---- File hashing ----

/// Compute SHA256 hash of a file's raw bytes (for dedup checking).
pub fn compute_file_hash(file_path: &Path) -> IngestionResult<String> {
    use sha2::{Digest, Sha256};
    let raw_bytes = std::fs::read(file_path).map_err(|e| {
        IngestionError::InvalidInput(format!("Failed to read file for hashing: {}", e))
    })?;
    Ok(format!("{:x}", Sha256::digest(&raw_bytes)))
}

// ---- Binary file detection ----

/// Extensions for truly binary files that can never contain personal data.
/// Everything NOT in this list goes to the LLM for classification.
const BINARY_SKIP_EXTS: &[&str] = &[
    // Compiled binaries
    "exe", "dll", "so", "dylib", "o", "a", "lib", "class", "pyc", "pyo", "beam", "wasm",
    // Fonts
    "woff", "woff2", "eot", "ttf", "otf",
    // Source maps / lock files
    "map", "lock",
];

/// Returns true if the file is a truly binary format that can never contain personal data.
/// These files are auto-skipped without consulting the LLM.
pub fn is_never_personal_data(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    BINARY_SKIP_EXTS.contains(&ext.as_str())
}

// ---- LLM analysis ----

/// Create the LLM prompt for file analysis with directory tree context.
///
/// The prompt includes the full directory tree so the LLM can reason about
/// what folders represent (e.g. a .gif inside a "Bank of America" HTML save
/// is scaffolding, not personal media).
pub fn create_smart_folder_prompt(tree_display: &str, file_paths: &[String]) -> String {
    let files_list = file_paths.join("\n");

    format!(
        r#"You are classifying files in a user's personal folder for ingestion into their personal database.

DIRECTORY TREE (for context — understand what each folder represents):
{tree_display}

FILES TO CLASSIFY:
{files_list}

For each file path listed in FILES TO CLASSIFY, determine:
1. Should it be ingested into the user's personal database?
2. What category does it belong to?

IMPORTANT: Use the directory tree to understand context. For example:
- A .gif inside a "Bank of America" saved HTML page is website scaffolding, NOT personal media
- A .js file inside a Twitter data export IS personal data
- A .css or .html file inside a saved webpage folder is scaffolding
- A .pdf in a "Statements" folder IS personal financial data
- Source code files (.py, .rs, .js) in a code project folder are NOT personal data
- But a .py notebook in a "Research" folder might be personal work

CATEGORIES:
- personal_data: Personal documents, notes, journals, financial records, health data, creative work, personal projects
- media: Images, videos, audio that are user-created content (NOT UI assets or website graphics)
- config: Application configs, settings files, dotfiles
- website_scaffolding: HTML templates, CSS, JS bundles, emoji assets, fonts, saved webpage resources
- work: Work/corporate files, professional documents
- unknown: Cannot determine

SKIP CRITERIA (should_ingest = false):
- Website scaffolding (CSS, JS bundles, images that are part of saved web pages)
- Application config files
- Source code (unless it's personal creative work)
- Cache and temporary files
- Downloaded installers/archives

INGEST CRITERIA (should_ingest = true):
- Personal documents (letters, notes, journals)
- Photos and videos (user-created, not UI assets)
- Messages and chat logs
- Financial records (statements, budgets, tax documents)
- Health data
- Creative work (writing, art, music)
- Data exports from services (Twitter, Facebook, Google Takeout, etc.)
- Personal work output (reports, presentations, research notes)

When in doubt, set should_ingest to false.

Respond with a JSON array of objects:
```json
[
  {{"path": "file/path.ext", "should_ingest": true, "category": "personal_data", "reason": "Brief reason"}},
  ...
]
```

Only return the JSON array, no other text."#
    )
}

/// Call the LLM for file analysis using the provided IngestionService
pub async fn call_llm_for_file_analysis(
    prompt: &str,
    service: &crate::ingestion::ingestion_service::IngestionService,
) -> IngestionResult<String> {
    service.call_ai_raw(prompt).await
}

/// Parse LLM response into file recommendations
pub fn parse_llm_file_recommendations(
    response: &str,
    file_tree: &[String],
) -> IngestionResult<Vec<FileRecommendation>> {
    let json_str = crate::ingestion::ai_helpers::extract_json_from_response(response)?;

    let parsed: Vec<FileRecommendation> = serde_json::from_str(&json_str)
        .map_err(|e| IngestionError::InvalidInput(format!("Failed to parse JSON: {}", e)))?;

    // Validate that paths exist in our file tree
    let file_set: HashSet<&str> = file_tree.iter().map(|s| s.as_str()).collect();

    let valid_recs: Vec<FileRecommendation> = parsed
        .into_iter()
        .filter(|rec| file_set.contains(rec.path.as_str()))
        .collect();

    Ok(valid_recs)
}

// ---- Heuristic fallback ----

/// Apply conservative heuristic-based filtering when LLM fails.
/// When in doubt, marks files as should_ingest = false.
pub fn apply_heuristic_filtering(file_tree: &[String]) -> Vec<FileRecommendation> {
    file_tree
        .iter()
        .map(|path| {
            let lower = path.to_lowercase();
            let ext = Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            // Strong personal data signals (documents with well-known personal formats)
            let is_personal_doc = matches!(
                ext.as_str(),
                "doc" | "docx" | "pdf" | "rtf" | "odt" | "pages"
                    | "xlsx" | "xls" | "csv" | "ods" | "numbers"
                    | "pptx" | "ppt" | "odp" | "key"
                    | "eml" | "mbox" | "vcf"
            );

            // Strong media signals
            let is_media = matches!(
                ext.as_str(),
                "jpg" | "jpeg" | "png" | "gif" | "heic" | "heif" | "webp" | "bmp" | "tiff"
                    | "raw" | "cr2" | "nef" | "arw"
                    | "mp4" | "mov" | "avi" | "mkv" | "m4v" | "wmv"
                    | "mp3" | "wav" | "flac" | "aac" | "m4a" | "ogg" | "wma"
            );

            // Data export patterns (high confidence personal data)
            let is_data_export = lower.contains("export")
                || lower.contains("backup")
                || lower.contains("takeout");

            let (should_ingest, category, reason) = if is_personal_doc {
                (true, "personal_data", "Personal document file")
            } else if is_media && is_data_export {
                (true, "media", "Media in data export")
            } else if is_data_export {
                (true, "personal_data", "Data export file")
            } else if is_media {
                // Without LLM context, we can't tell if media is personal or scaffolding
                (false, "media", "Media file (needs review)")
            } else {
                (false, "unknown", "Could not classify without AI")
            };

            FileRecommendation {
                path: path.clone(),
                should_ingest,
                category: category.to_string(),
                reason: reason.to_string(),
                file_size_bytes: 0,
                estimated_cost: 0.0,
                already_ingested: false,
            }
        })
        .collect()
}

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

    report(15, format!("Found {} files. Filtering known extensions...", scan.file_paths.len()));

    // Split files: binary auto-skip vs everything else goes to LLM
    let mut binary_skipped: Vec<FileRecommendation> = Vec::new();
    let mut llm_candidates: Vec<String> = Vec::new();

    for path in &scan.file_paths {
        if is_never_personal_data(path) {
            binary_skipped.push(FileRecommendation {
                path: path.clone(),
                should_ingest: false,
                category: "binary_or_system".to_string(),
                reason: "Binary/system file".to_string(),
                file_size_bytes: 0,
                estimated_cost: 0.0,
                already_ingested: false,
            });
        } else {
            llm_candidates.push(path.clone());
        }
    }

    log::info!(
        "File classification: {} binary-skipped, {} candidates → LLM",
        binary_skipped.len(),
        llm_candidates.len(),
    );

    report(25, format!(
        "Skipped {} binary files. Classifying {} files with AI...",
        binary_skipped.len(),
        llm_candidates.len(),
    ));

    // Send all non-binary files to LLM in batches (with tree context)
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
        let mut all_recs = Vec::new();
        for (i, chunk_vec) in chunks.into_iter().enumerate() {
            if total_batches > 1 {
                report(
                    25 + ((i as u8) * 50 / total_batches as u8),
                    format!("Classifying files with AI (batch {}/{})", i + 1, total_batches),
                );
            }
            let prompt = create_smart_folder_prompt(&scan.tree_display, &chunk_vec);
            match call_llm_for_file_analysis(&prompt, svc).await {
                Ok(llm_response) => {
                    match parse_llm_file_recommendations(&llm_response, &chunk_vec) {
                        Ok(recs) => all_recs.extend(recs),
                        Err(e) => {
                            log::warn!(
                                "Failed to parse LLM response, using heuristics for batch: {}",
                                e
                            );
                            all_recs.extend(apply_heuristic_filtering(&chunk_vec));
                        }
                    }
                }
                Err(e) => {
                    log::warn!("LLM call failed, using heuristics for batch: {}", e);
                    all_recs.extend(apply_heuristic_filtering(&chunk_vec));
                }
            }
        }
        all_recs
    };

    report(80, "Checking dedup status...".into());

    // Merge binary-skipped + LLM recommendations
    let recommendations: Vec<FileRecommendation> =
        binary_skipped.into_iter().chain(llm_recs).collect();

    // Split into recommended and skipped, build summary, compute costs
    let mut recommended_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut total_estimated_cost = 0.0;
    let mut summary: SmartFolderSummary = HashMap::new();

    // Get pub_key from node for dedup checking
    let pub_key = node.map(|n| n.get_node_public_key().to_string());

    for mut rec in recommendations {
        // Populate file size and cost estimate
        let rel_path = Path::new(&rec.path);
        rec.file_size_bytes = file_size_bytes(rel_path, folder_path);
        rec.estimated_cost = estimate_file_cost(rel_path, folder_path);

        if rec.should_ingest {
            // Check if this file has already been ingested
            if let (Some(ref pk), Some(n)) = (&pub_key, node) {
                let full_path = folder_path.join(&rec.path);
                if let Ok(hash) = compute_file_hash(&full_path) {
                    if n.is_file_ingested(pk, &hash).await.is_some() {
                        rec.should_ingest = false;
                        rec.already_ingested = true;
                        rec.reason = "Already ingested".to_string();
                        *summary.entry("already_ingested".to_string()).or_insert(0) += 1;
                        skipped_files.push(rec);
                        continue;
                    }
                }
            }
            *summary.entry(rec.category.clone()).or_insert(0) += 1;
            total_estimated_cost += rec.estimated_cost;
            recommended_files.push(rec);
        } else {
            *summary.entry(rec.category.clone()).or_insert(0) += 1;
            skipped_files.push(rec);
        }
    }

    report(100, format!(
        "Scan complete. {} to ingest, {} skipped.",
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
    fn test_common_files_go_to_llm() {
        // These should NOT be auto-skipped — they go to the LLM
        assert!(!is_never_personal_data("photo.jpg"));
        assert!(!is_never_personal_data("document.pdf"));
        assert!(!is_never_personal_data("notes.txt"));
        assert!(!is_never_personal_data("data.json"));
        assert!(!is_never_personal_data("style.css"));
        assert!(!is_never_personal_data("page.html"));
        assert!(!is_never_personal_data("script.js"));
        assert!(!is_never_personal_data("code.py"));
        assert!(!is_never_personal_data("archive.zip"));
        assert!(!is_never_personal_data("image.gif"));
        assert!(!is_never_personal_data("song.mp3"));
        assert!(!is_never_personal_data("video.mp4"));
        assert!(!is_never_personal_data("spreadsheet.xlsx"));
        assert!(!is_never_personal_data("config.yaml"));
        assert!(!is_never_personal_data("readme.md"));
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
        std::fs::write(root.join("photo.jpg"), "fake jpg").unwrap();

        let result = scan_directory_tree_with_context(root, 10, 50000).unwrap();

        assert_eq!(result.file_paths.len(), 2);
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
