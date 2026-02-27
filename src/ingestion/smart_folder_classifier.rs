//! LLM-based file classification and heuristic fallback
//! for the smart folder feature.

use crate::ingestion::error::IngestionError;
use crate::ingestion::IngestionResult;
use std::collections::HashSet;
use std::path::Path;

use super::smart_folder::FileRecommendation;

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
