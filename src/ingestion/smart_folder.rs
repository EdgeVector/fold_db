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

// ---- LLM analysis ----

/// Create the LLM prompt for file analysis
pub fn create_smart_folder_prompt(file_tree: &[String]) -> String {
    let files_list = file_tree.join("\n");

    format!(
        r#"Analyze this directory listing and categorize each file for personal data ingestion.

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
- Work/corporate documents (if identifiable)

INGEST CRITERIA (should_ingest = true):
- Personal documents (letters, notes, journals)
- Photos and videos (user-created, not UI assets)
- Messages and chat logs
- Financial records (statements, budgets)
- Health data
- Creative work (writing, art, music)
- Data exports from services (Twitter, Facebook, etc.)

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
                || ext == "wav";

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
        });
    }

    // Create the LLM prompt with the file tree
    let prompt = create_smart_folder_prompt(&file_tree);

    // Create service from env if not provided
    let owned_service;
    let svc = match service {
        Some(s) => s,
        None => {
            owned_service = crate::ingestion::ingestion_service::IngestionService::from_env()?;
            &owned_service
        }
    };

    // Call the LLM
    let recommendations = match call_llm_for_file_analysis(&prompt, svc).await {
        Ok(llm_response) => match parse_llm_file_recommendations(&llm_response, &file_tree) {
            Ok(recs) => recs,
            Err(e) => {
                log::warn!("Failed to parse LLM response, using heuristics: {}", e);
                apply_heuristic_filtering(&file_tree)
            }
        },
        Err(e) => {
            log::warn!("LLM call failed, using heuristics: {}", e);
            apply_heuristic_filtering(&file_tree)
        }
    };

    // Split into recommended and skipped, build summary
    let mut recommended_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut summary = SmartFolderSummary {
        personal_data_count: 0,
        media_count: 0,
        config_count: 0,
        website_scaffolding_count: 0,
        work_count: 0,
        unknown_count: 0,
    };

    for rec in recommendations {
        match rec.category.as_str() {
            "personal_data" => summary.personal_data_count += 1,
            "media" => summary.media_count += 1,
            "config" => summary.config_count += 1,
            "website_scaffolding" => summary.website_scaffolding_count += 1,
            "work" => summary.work_count += 1,
            _ => summary.unknown_count += 1,
        }

        if rec.should_ingest {
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
}
