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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_llm_file_recommendations tests ----

    #[test]
    fn test_parse_llm_valid_json_with_matching_paths() {
        let response = r#"```json
[
  {"path": "docs/notes.txt", "should_ingest": true, "category": "personal_data", "reason": "Personal notes"},
  {"path": "photos/pic.jpg", "should_ingest": true, "category": "media", "reason": "Photo"}
]
```"#;
        let file_tree = vec![
            "docs/notes.txt".to_string(),
            "photos/pic.jpg".to_string(),
        ];
        let result = parse_llm_file_recommendations(response, &file_tree).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, "docs/notes.txt");
        assert_eq!(result[1].path, "photos/pic.jpg");
    }

    #[test]
    fn test_parse_llm_hallucinated_paths_filtered() {
        let response = r#"[
  {"path": "docs/notes.txt", "should_ingest": true, "category": "personal_data", "reason": "ok"},
  {"path": "fake/hallucinated.txt", "should_ingest": true, "category": "unknown", "reason": "nope"}
]"#;
        let file_tree = vec!["docs/notes.txt".to_string()];
        let result = parse_llm_file_recommendations(response, &file_tree).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "docs/notes.txt");
    }

    #[test]
    fn test_parse_llm_empty_response_returns_error() {
        let file_tree = vec!["a.txt".to_string()];
        let result = parse_llm_file_recommendations("", &file_tree);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_llm_mixed_valid_invalid_paths() {
        let response = r#"[
  {"path": "a.txt", "should_ingest": true, "category": "personal_data", "reason": "ok"},
  {"path": "b.txt", "should_ingest": false, "category": "unknown", "reason": "nope"},
  {"path": "c.txt", "should_ingest": true, "category": "work", "reason": "work file"}
]"#;
        let file_tree = vec!["a.txt".to_string(), "c.txt".to_string()];
        let result = parse_llm_file_recommendations(response, &file_tree).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, "a.txt");
        assert_eq!(result[1].path, "c.txt");
    }

    #[test]
    fn test_parse_llm_empty_file_tree_returns_empty() {
        let response = r#"[
  {"path": "a.txt", "should_ingest": true, "category": "personal_data", "reason": "ok"}
]"#;
        let file_tree: Vec<String> = vec![];
        let result = parse_llm_file_recommendations(response, &file_tree).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_llm_malformed_json_returns_error() {
        let response = r#"This is not JSON at all, just some text."#;
        let file_tree = vec!["a.txt".to_string()];
        let result = parse_llm_file_recommendations(response, &file_tree);
        assert!(result.is_err());
    }

    // ---- create_smart_folder_prompt tests ----

    #[test]
    fn test_prompt_contains_tree_and_file_paths() {
        let tree = "docs/\n  notes.txt\n  report.pdf";
        let files = vec![
            "docs/notes.txt".to_string(),
            "docs/report.pdf".to_string(),
        ];
        let prompt = create_smart_folder_prompt(tree, &files);
        assert!(prompt.contains(tree));
        assert!(prompt.contains("docs/notes.txt"));
        assert!(prompt.contains("docs/report.pdf"));
    }

    #[test]
    fn test_prompt_contains_categories_and_instructions() {
        let prompt = create_smart_folder_prompt("tree", &["f.txt".to_string()]);
        assert!(prompt.contains("personal_data"));
        assert!(prompt.contains("media"));
        assert!(prompt.contains("website_scaffolding"));
        assert!(prompt.contains("should_ingest"));
        assert!(prompt.contains("JSON array"));
    }

    // ---- apply_heuristic_filtering tests ----

    #[test]
    fn test_heuristic_mixed_file_types() {
        let files = vec![
            "report.pdf".to_string(),
            "photo.jpg".to_string(),
            "script.py".to_string(),
            "data.csv".to_string(),
            "export/backup.json".to_string(),
        ];
        let recs = apply_heuristic_filtering(&files);
        assert_eq!(recs.len(), 5);

        // PDF → personal_data, should_ingest
        assert!(recs[0].should_ingest);
        assert_eq!(recs[0].category, "personal_data");

        // JPG without export context → media, should_ingest = false
        assert!(!recs[1].should_ingest);
        assert_eq!(recs[1].category, "media");

        // .py → unknown, should_ingest = false
        assert!(!recs[2].should_ingest);

        // CSV → personal_data, should_ingest
        assert!(recs[3].should_ingest);
        assert_eq!(recs[3].category, "personal_data");

        // backup path → personal_data, should_ingest
        assert!(recs[4].should_ingest);
    }

    #[test]
    fn test_heuristic_case_insensitive_extensions() {
        let files = vec![
            "REPORT.PDF".to_string(),
            "Data.Csv".to_string(),
            "photo.JPG".to_string(),
        ];
        let recs = apply_heuristic_filtering(&files);

        // .PDF → personal_data (extension lowercased internally)
        assert!(recs[0].should_ingest);
        assert_eq!(recs[0].category, "personal_data");

        // .Csv → personal_data
        assert!(recs[1].should_ingest);
        assert_eq!(recs[1].category, "personal_data");

        // .JPG without export → media, not ingested
        assert!(!recs[2].should_ingest);
        assert_eq!(recs[2].category, "media");
    }
}
