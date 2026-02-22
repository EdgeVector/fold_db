//! Shared smart-folder scan and ingestion logic.
//!
//! These functions are framework-agnostic and used by both
//! HTTP handlers (`routes.rs`) and the CLI (`folddb`).

use crate::ingestion::error::IngestionError;
use crate::ingestion::IngestionResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

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
}

/// Summary of smart folder scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFolderSummary {
    pub personal_data_count: usize,
    pub media_count: usize,
    pub config_count: usize,
    pub website_scaffolding_count: usize,
    pub work_count: usize,
    pub unknown_count: usize,
}

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

// ---- Directory scanning ----

/// Recursively scan a directory tree up to max_depth
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

    let entries = std::fs::read_dir(current)
        .map_err(|e| IngestionError::InvalidInput(format!("Failed to read directory {}: {}", current.display(), e)))?;

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

// ---- Three-way file classification ----

/// Result of classifying a file by its extension and path patterns.
#[derive(Debug, Clone, PartialEq)]
pub enum FileClassification {
    /// File can be skipped without consulting the LLM.
    Skip {
        category: &'static str,
        reason: &'static str,
    },
    /// File should be ingested without consulting the LLM.
    Ingest {
        category: &'static str,
        reason: &'static str,
    },
    /// Cannot determine from extension/path alone — send to LLM.
    Ambiguous,
}

/// Extensions that are always skipped.
const SKIP_EXTS: &[&str] = &[
    // Binary/compiled
    "exe", "dll", "so", "dylib", "o", "a", "lib", "bin", "class", "pyc", "pyo",
    // Archives/installers
    "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "dmg", "iso", "pkg", "deb", "rpm", "msi",
    // Fonts
    "woff", "woff2", "eot", "ttf", "otf",
    // Database/lock
    "db", "sqlite", "sqlite3", "lock",
    // Source maps
    "map",
    // Logs
    "log",
];

/// Extensions that are always ingested as personal_data.
const INGEST_PERSONAL_EXTS: &[&str] = &[
    // Documents
    "doc", "docx", "pdf", "rtf", "odt", "pages",
    // Spreadsheets
    "xlsx", "xls", "csv", "ods", "numbers",
    // Presentations
    "pptx", "ppt", "odp", "key",
    // Email/contacts
    "eml", "mbox", "vcf",
];

/// Extensions that are always ingested as media.
const INGEST_MEDIA_EXTS: &[&str] = &[
    // Photos
    "jpg", "jpeg", "png", "gif", "heic", "heif", "webp", "bmp", "tiff", "raw", "cr2", "nef", "arw",
    // Video
    "mp4", "mov", "avi", "mkv", "m4v", "wmv",
    // Audio
    "mp3", "wav", "flac", "aac", "m4a", "ogg", "wma",
];

/// Classify a file path into Skip, Ingest, or Ambiguous based on extension and path patterns.
pub fn classify_file(path: &str) -> FileClassification {
    let lower = path.to_lowercase();
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // --- Path-pattern skips (checked first) ---
    if lower.contains("node_modules/") || lower.contains("twemoji/") {
        return FileClassification::Skip {
            category: "website_scaffolding",
            reason: "Website scaffolding directory",
        };
    }

    // Filename patterns: runtime.*.js or modules.*.js
    if let Some(file_name) = Path::new(path).file_name().and_then(|n| n.to_str()) {
        let fn_lower = file_name.to_lowercase();
        if (fn_lower.starts_with("runtime.") && fn_lower.ends_with(".js"))
            || (fn_lower.starts_with("modules.") && fn_lower.ends_with(".js"))
        {
            return FileClassification::Skip {
                category: "website_scaffolding",
                reason: "Bundled JS scaffolding file",
            };
        }
    }

    // /assets/ with font/emoji extension
    if lower.contains("/assets/") {
        let font_emoji_exts = ["woff", "woff2", "eot", "ttf", "otf", "svg"];
        if font_emoji_exts.contains(&ext.as_str()) {
            return FileClassification::Skip {
                category: "website_scaffolding",
                reason: "Font/emoji asset file",
            };
        }
    }

    // --- Extension-based skip ---
    if SKIP_EXTS.contains(&ext.as_str()) {
        return FileClassification::Skip {
            category: "binary_or_system",
            reason: "Binary, archive, or system file",
        };
    }

    // --- Extension-based ingest: media (with asset guard) ---
    if INGEST_MEDIA_EXTS.contains(&ext.as_str()) {
        // Media files in /assets/ or twemoji paths are scaffolding, not personal media
        if lower.contains("/assets/") || lower.contains("twemoji") {
            return FileClassification::Skip {
                category: "website_scaffolding",
                reason: "UI asset, not personal media",
            };
        }
        return FileClassification::Ingest {
            category: "media",
            reason: "User media file",
        };
    }

    // --- Extension-based ingest: personal data ---
    if INGEST_PERSONAL_EXTS.contains(&ext.as_str()) {
        return FileClassification::Ingest {
            category: "personal_data",
            reason: "Personal document file",
        };
    }

    // --- Everything else is ambiguous ---
    FileClassification::Ambiguous
}

// ---- LLM analysis ----

/// Create the LLM prompt for file analysis
pub fn create_smart_folder_prompt(file_tree: &[String]) -> String {
    let files_list = file_tree.join("\n");

    format!(
        r#"Analyze this directory listing and categorize each file for personal data ingestion.

NOTE: These files could not be auto-classified by extension alone (obvious media, documents,
binaries, and archives have already been handled). Focus on path context and file names to decide.

DIRECTORY LISTING:
{}

For each file, determine:
1. Should it be ingested into a personal database?
2. What category does it belong to?

CATEGORIES:
- personal_data: Personal documents, notes, journals, photos, messages, financial records, health data, creative work, personal projects
- media: Images, videos, audio (user-created content, not UI assets)
- config: Application configs, settings files, dotfiles
- website_scaffolding: HTML templates, CSS, JS bundles, emoji assets, fonts, node_modules contents
- work: Work/corporate files, professional documents
- unknown: Cannot determine

SKIP CRITERIA (should_ingest = false):
- Application scaffolding (runtime.js, modules.js, twemoji/, fonts/)
- Config files (.config, .env, settings.json unless personal)
- Cache and temporary files
- Binary executables
- Downloaded installers/archives
- Organizational/corporate files (company policies, HR forms) — but personal work output like reports, presentations, or notes you authored should be ingested

INGEST CRITERIA (should_ingest = true):
- Personal documents (letters, notes, journals)
- Photos and videos (user-created, not UI assets)
- Messages and chat logs
- Financial records (statements, budgets)
- Health data
- Creative work (writing, art, music)
- Data exports from services (Twitter, Facebook, etc.)
- Personal work output (reports, presentations, research, notes you authored)

Respond with a JSON array of objects:
```json
[
  {{"path": "file/path.ext", "should_ingest": true, "category": "personal_data", "reason": "Brief reason"}},
  ...
]
```

Only return the JSON array, no other text."#,
        files_list
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

    let parsed: Vec<FileRecommendation> =
        serde_json::from_str(&json_str).map_err(|e| IngestionError::InvalidInput(format!("Failed to parse JSON: {}", e)))?;

    // Validate that paths exist in our file tree
    let file_set: HashSet<&str> = file_tree.iter().map(|s| s.as_str()).collect();

    let valid_recs: Vec<FileRecommendation> = parsed
        .into_iter()
        .filter(|rec| file_set.contains(rec.path.as_str()))
        .collect();

    Ok(valid_recs)
}

// ---- Heuristic fallback ----

/// Apply heuristic-based filtering when LLM fails
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

            // Website scaffolding patterns
            let is_scaffolding = lower.contains("node_modules")
                || lower.contains("twemoji")
                || lower.contains("/assets/")
                || lower.contains("runtime.")
                || lower.contains("modules.")
                || ext == "woff"
                || ext == "woff2"
                || ext == "eot"
                || ext == "ttf"
                || (ext == "svg" && lower.contains("emoji"));

            // Config patterns
            let is_config = lower.starts_with(".")
                || lower.contains(".config")
                || lower.contains("config/")
                || ext == "env"
                || ext == "ini"
                || ext == "yaml"
                || ext == "yml";

            // Personal data patterns
            let is_personal = ext == "json"
                || ext == "csv"
                || ext == "txt"
                || ext == "md"
                || ext == "doc"
                || ext == "docx"
                || ext == "pdf"
                || ext == "js"
                || ext == "xlsx"
                || ext == "xls"
                || ext == "pptx"
                || ext == "ppt"
                || ext == "rtf"
                || ext == "html"
                || ext == "htm"
                || ext == "eml"
                || lower.contains("data/")
                || lower.contains("export")
                || lower.contains("backup");

            // Media patterns
            let is_media = ext == "jpg"
                || ext == "jpeg"
                || ext == "png"
                || ext == "gif"
                || ext == "mp4"
                || ext == "mp3"
                || ext == "wav"
                || ext == "heic"
                || ext == "heif"
                || ext == "mov"
                || ext == "webp"
                || ext == "bmp"
                || ext == "tiff"
                || ext == "svg";

            let (should_ingest, category, reason) = if is_scaffolding {
                (
                    false,
                    "website_scaffolding",
                    "Appears to be website/app scaffolding",
                )
            } else if is_config {
                (false, "config", "Appears to be configuration file")
            } else if is_media && !lower.contains("twemoji") && !lower.contains("/assets/") {
                (true, "media", "User media file")
            } else if is_personal {
                (true, "personal_data", "Potential personal data file")
            } else {
                (false, "unknown", "Unknown file type")
            };

            FileRecommendation {
                path: path.clone(),
                should_ingest,
                category: category.to_string(),
                reason: reason.to_string(),
                file_size_bytes: 0,
                estimated_cost: 0.0,
            }
        })
        .collect()
}

// ---- File conversion ----

/// Convert CSV content to JSON array
pub fn csv_to_json(csv_content: &str) -> IngestionResult<String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_content.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| IngestionError::InvalidInput(format!("Failed to read CSV headers: {}", e)))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut records: Vec<Value> = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| IngestionError::InvalidInput(format!("Failed to read CSV record: {}", e)))?;
        let mut obj = serde_json::Map::new();

        for (i, field) in record.iter().enumerate() {
            if let Some(header) = headers.get(i) {
                let value = if let Ok(n) = field.parse::<f64>() {
                    Value::Number(
                        serde_json::Number::from_f64(n)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    )
                } else if field == "true" {
                    Value::Bool(true)
                } else if field == "false" {
                    Value::Bool(false)
                } else {
                    Value::String(field.to_string())
                };
                obj.insert(header.clone(), value);
            }
        }

        records.push(Value::Object(obj));
    }

    serde_json::to_string(&records).map_err(|e| IngestionError::InvalidInput(format!("Failed to serialize JSON: {}", e)))
}

/// Convert a Twitter data export `.js` file to JSON.
///
/// Twitter data exports use files like `window.YTD.tweet.part0 = [...]`.
/// This strips the variable assignment prefix and returns the pure JSON.
pub fn twitter_js_to_json(content: &str) -> IngestionResult<String> {
    if let Some(eq_pos) = content.find('=') {
        let json_part = content[eq_pos + 1..].trim();
        // Validate it parses as JSON
        serde_json::from_str::<Value>(json_part)
            .map_err(|e| IngestionError::InvalidInput(format!("Invalid JSON in .js file: {}", e)))?;
        Ok(json_part.to_string())
    } else {
        Err(IngestionError::InvalidInput("Not a Twitter data export .js file (no '=' found)".to_string()))
    }
}

// ---- Unified file reader ----

/// Read a file and convert it to a JSON Value regardless of format.
///
/// Supported extensions: `.json`, `.js` (Twitter export), `.csv`, `.txt`, `.md`
pub fn read_file_as_json(file_path: &Path) -> IngestionResult<Value> {
    let content =
        std::fs::read_to_string(file_path).map_err(|e| IngestionError::InvalidInput(format!("Failed to read file: {}", e)))?;

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let json_string = match ext.as_str() {
        "json" => content,
        "js" => twitter_js_to_json(&content)?,
        "csv" => csv_to_json(&content)?,
        "txt" | "md" => {
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            serde_json::to_string(&serde_json::json!({
                "content": content,
                "source_file": file_name,
                "file_type": ext
            }))
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to wrap text content: {}", e)))?
        }
        _ => return Err(IngestionError::InvalidInput(format!("Unsupported file type: {}", ext))),
    };

    serde_json::from_str(&json_string).map_err(|e| IngestionError::InvalidInput(format!("Failed to parse JSON: {}", e)))
}

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
fn file_size_bytes(path: &Path, root: &Path) -> u64 {
    let full_path = root.join(path);
    std::fs::metadata(&full_path)
        .map(|m| m.len())
        .unwrap_or(0)
}

// ---- Scan orchestration ----

/// Perform a smart folder scan: directory walk → LLM classification → recommendations.
///
/// This is the core logic shared between the HTTP handler and the CLI.
/// If `service` is `None`, an `IngestionService` is created from the environment.
pub async fn perform_smart_folder_scan(
    folder_path: &Path,
    max_depth: usize,
    max_files: usize,
    service: Option<&crate::ingestion::ingestion_service::IngestionService>,
) -> IngestionResult<SmartFolderScanResponse> {
    let file_tree = scan_directory_tree(folder_path, max_depth, max_files)?;

    let scan_truncated = file_tree.len() >= max_files;

    if file_tree.is_empty() {
        return Ok(SmartFolderScanResponse {
            success: true,
            total_files: 0,
            recommended_files: vec![],
            skipped_files: vec![],
            summary: SmartFolderSummary {
                personal_data_count: 0,
                media_count: 0,
                config_count: 0,
                website_scaffolding_count: 0,
                work_count: 0,
                unknown_count: 0,
            },
            total_estimated_cost: 0.0,
            scan_truncated,
            max_depth_used: max_depth,
            max_files_used: max_files,
        });
    }

    // Three-way classify each file: Skip, Ingest, or Ambiguous
    let mut hardcoded_recs: Vec<FileRecommendation> = Vec::new();
    let mut ambiguous_paths: Vec<String> = Vec::new();

    for path in &file_tree {
        match classify_file(path) {
            FileClassification::Skip { category, reason } => {
                hardcoded_recs.push(FileRecommendation {
                    path: path.clone(),
                    should_ingest: false,
                    category: category.to_string(),
                    reason: reason.to_string(),
                    file_size_bytes: 0,
                    estimated_cost: 0.0,
                });
            }
            FileClassification::Ingest { category, reason } => {
                hardcoded_recs.push(FileRecommendation {
                    path: path.clone(),
                    should_ingest: true,
                    category: category.to_string(),
                    reason: reason.to_string(),
                    file_size_bytes: 0,
                    estimated_cost: 0.0,
                });
            }
            FileClassification::Ambiguous => {
                ambiguous_paths.push(path.clone());
            }
        }
    }

    log::info!(
        "File classification: {} hardcoded ({} ingest, {} skip), {} ambiguous → LLM",
        hardcoded_recs.len(),
        hardcoded_recs.iter().filter(|r| r.should_ingest).count(),
        hardcoded_recs.iter().filter(|r| !r.should_ingest).count(),
        ambiguous_paths.len(),
    );

    // Send only ambiguous files to the LLM (if any)
    let ambiguous_recs = if ambiguous_paths.is_empty() {
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

        let batch_size = 500;
        let mut all_recs = Vec::new();
        for chunk in ambiguous_paths.chunks(batch_size) {
            let chunk_vec: Vec<String> = chunk.to_vec();
            let prompt = create_smart_folder_prompt(&chunk_vec);
            match call_llm_for_file_analysis(&prompt, svc).await {
                Ok(llm_response) => match parse_llm_file_recommendations(&llm_response, &chunk_vec) {
                    Ok(recs) => all_recs.extend(recs),
                    Err(e) => {
                        log::warn!("Failed to parse LLM response, using heuristics for batch: {}", e);
                        all_recs.extend(apply_heuristic_filtering(&chunk_vec));
                    }
                },
                Err(e) => {
                    log::warn!("LLM call failed, using heuristics for batch: {}", e);
                    all_recs.extend(apply_heuristic_filtering(&chunk_vec));
                }
            }
        }
        all_recs
    };

    // Merge hardcoded + ambiguous recommendations
    let recommendations: Vec<FileRecommendation> = hardcoded_recs
        .into_iter()
        .chain(ambiguous_recs)
        .collect();

    // Split into recommended and skipped, build summary, compute costs
    let mut recommended_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut total_estimated_cost = 0.0;
    let mut summary = SmartFolderSummary {
        personal_data_count: 0,
        media_count: 0,
        config_count: 0,
        website_scaffolding_count: 0,
        work_count: 0,
        unknown_count: 0,
    };

    for mut rec in recommendations {
        // Populate file size and cost estimate
        let rel_path = Path::new(&rec.path);
        rec.file_size_bytes = file_size_bytes(rel_path, folder_path);
        rec.estimated_cost = estimate_file_cost(rel_path, folder_path);

        match rec.category.as_str() {
            "personal_data" => summary.personal_data_count += 1,
            "media" => summary.media_count += 1,
            "config" => summary.config_count += 1,
            "website_scaffolding" => summary.website_scaffolding_count += 1,
            "work" => summary.work_count += 1,
            _ => summary.unknown_count += 1,
        }

        if rec.should_ingest {
            total_estimated_cost += rec.estimated_cost;
            recommended_files.push(rec);
        } else {
            skipped_files.push(rec);
        }
    }

    Ok(SmartFolderScanResponse {
        success: true,
        total_files: file_tree.len(),
        recommended_files,
        skipped_files,
        summary,
        total_estimated_cost,
        scan_truncated,
        max_depth_used: max_depth,
        max_files_used: max_files,
    })
}


#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_heuristic_filtering_json_file() {
        let files = vec!["data/export.json".to_string()];
        let recs = apply_heuristic_filtering(&files);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].should_ingest);
        assert_eq!(recs[0].category, "personal_data");
    }

    #[test]
    fn test_heuristic_filtering_scaffolding() {
        let files = vec!["assets/twemoji/1f600.svg".to_string()];
        let recs = apply_heuristic_filtering(&files);
        assert_eq!(recs.len(), 1);
        assert!(!recs[0].should_ingest);
        assert_eq!(recs[0].category, "website_scaffolding");
    }

    #[test]
    fn test_heuristic_filtering_js_file() {
        let files = vec!["data/tweet.js".to_string()];
        let recs = apply_heuristic_filtering(&files);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].should_ingest);
        assert_eq!(recs[0].category, "personal_data");
    }

    #[test]
    fn test_read_file_as_json_unsupported() {
        let result = read_file_as_json(Path::new("/tmp/test.xyz"));
        assert!(result.is_err());
    }

    // ---- classify_file tests ----

    #[test]
    fn test_classify_binary_skip() {
        assert_eq!(
            classify_file("program.exe"),
            FileClassification::Skip {
                category: "binary_or_system",
                reason: "Binary, archive, or system file",
            }
        );
        assert_eq!(
            classify_file("lib/native.so"),
            FileClassification::Skip {
                category: "binary_or_system",
                reason: "Binary, archive, or system file",
            }
        );
    }

    #[test]
    fn test_classify_archive_skip() {
        assert_eq!(
            classify_file("backup.zip"),
            FileClassification::Skip {
                category: "binary_or_system",
                reason: "Binary, archive, or system file",
            }
        );
    }

    #[test]
    fn test_classify_photo_ingest() {
        assert_eq!(
            classify_file("photos/vacation.jpg"),
            FileClassification::Ingest {
                category: "media",
                reason: "User media file",
            }
        );
    }

    #[test]
    fn test_classify_photo_in_assets_skip() {
        assert_eq!(
            classify_file("static/assets/icon.png"),
            FileClassification::Skip {
                category: "website_scaffolding",
                reason: "UI asset, not personal media",
            }
        );
    }

    #[test]
    fn test_classify_document_ingest() {
        assert_eq!(
            classify_file("reports/q1.pdf"),
            FileClassification::Ingest {
                category: "personal_data",
                reason: "Personal document file",
            }
        );
        assert_eq!(
            classify_file("notes.docx"),
            FileClassification::Ingest {
                category: "personal_data",
                reason: "Personal document file",
            }
        );
    }

    #[test]
    fn test_classify_json_ambiguous() {
        assert_eq!(classify_file("data/config.json"), FileClassification::Ambiguous);
    }

    #[test]
    fn test_classify_unknown_ext_ambiguous() {
        assert_eq!(classify_file("something.xyz"), FileClassification::Ambiguous);
    }

    #[test]
    fn test_classify_node_modules_skip() {
        assert_eq!(
            classify_file("node_modules/react/index.js"),
            FileClassification::Skip {
                category: "website_scaffolding",
                reason: "Website scaffolding directory",
            }
        );
    }

    #[test]
    fn test_classify_runtime_js_skip() {
        assert_eq!(
            classify_file("dist/runtime.abc123.js"),
            FileClassification::Skip {
                category: "website_scaffolding",
                reason: "Bundled JS scaffolding file",
            }
        );
    }

    #[test]
    fn test_classify_twemoji_media_skip() {
        assert_eq!(
            classify_file("twemoji/1f600.png"),
            FileClassification::Skip {
                category: "website_scaffolding",
                reason: "Website scaffolding directory",
            }
        );
    }

    #[test]
    fn test_classify_font_in_assets_skip() {
        assert_eq!(
            classify_file("static/assets/font.woff2"),
            FileClassification::Skip {
                category: "website_scaffolding",
                reason: "Font/emoji asset file",
            }
        );
    }

    #[test]
    fn test_classify_csv_ingest() {
        assert_eq!(
            classify_file("data/contacts.csv"),
            FileClassification::Ingest {
                category: "personal_data",
                reason: "Personal document file",
            }
        );
    }

    #[test]
    fn test_classify_txt_ambiguous() {
        assert_eq!(classify_file("readme.txt"), FileClassification::Ambiguous);
    }
}
