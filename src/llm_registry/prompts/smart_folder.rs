//! Prompt templates for smart folder scanning and file classification.
//!
//! Used by the ingestion pipeline to classify files for database ingestion.

/// Build the LLM prompt for classifying files in a user's folder.
///
/// Takes a directory tree display and list of file paths to classify.
pub fn build_smart_folder_prompt(tree_display: &str, file_paths: &[String]) -> String {
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

{CATEGORIES}

{SKIP_CRITERIA}

{INGEST_CRITERIA}

When in doubt, set should_ingest to false.

Respond with a JSON array of objects:
```json
[
  {{"path": "file/path.ext", "should_ingest": true, "category": "personal_data", "reason": "Brief reason"}},
  ...
]
```

Only return the JSON array, no other text."#,
        CATEGORIES = FILE_CATEGORIES,
        SKIP_CRITERIA = SKIP_CRITERIA,
        INGEST_CRITERIA = INGEST_CRITERIA,
    )
}

/// Build an LLM prompt to classify image directories as personal or asset.
pub fn build_image_directory_prompt(dir_lines: &[String]) -> String {
    format!(
        r#"You are classifying IMAGE DIRECTORIES to determine if they contain personal images or non-personal asset images.

IMAGE DIRECTORIES (with file counts and sample filenames):
{}

For each directory, classify it as either:
- "personal" — user photos, screenshots, personal artwork, scanned documents, camera images
- "asset" — UI assets, emoji/icon collections, website graphics, app resources, stock images, thumbnails

GUIDELINES:
- Directories named like "tweets_media", "profile_media", "photos", "camera", "screenshots" → personal
- Directories named like "twemoji", "emoji", "icons", "assets/images", "thumbnails", "sprites" → asset
- Directories with few large files (photos) → likely personal
- Directories with many small files (icons, emoji) → likely asset
- When in doubt, classify as "personal" (better to include than exclude)

Respond with a JSON object mapping each directory path to "personal" or "asset":
```json
{{
  "directory/path": "personal",
  "assets/images/twemoji": "asset"
}}
```

Only return the JSON object, no other text."#,
        dir_lines.join("\n")
    )
}

/// Build a prompt for adjusting scan results based on a user instruction.
pub fn build_adjust_prompt(
    instruction: &str,
    rec_lines: &[String],
    skip_lines: &[String],
) -> String {
    format!(
        r#"You are adjusting file ingestion recommendations based on the user's instruction.

USER INSTRUCTION: "{instruction}"

CURRENT FILES TO INGEST:
[
{rec_list}
]

CURRENT SKIPPED FILES:
[
{skip_list}
]

Apply the user's instruction to reclassify files. For example:
- "include all work files" → move work-category files from skipped to should_ingest=true
- "skip all images" → move image files from recommended to should_ingest=false
- "include everything" → set all files to should_ingest=true

{CATEGORIES}

Respond with a JSON array of ALL files (both recommended and skipped) with updated classifications:
```json
[
  {{"path": "file/path.ext", "should_ingest": true, "category": "personal_data", "reason": "Brief reason"}},
  ...
]
```

Only return the JSON array, no other text."#,
        rec_list = rec_lines.join(",\n"),
        skip_list = skip_lines.join(",\n"),
        CATEGORIES = FILE_CATEGORIES,
    )
}

/// File category definitions shared across smart folder prompts.
pub const FILE_CATEGORIES: &str = r#"CATEGORIES:
- personal_data: Personal documents, notes, journals, financial records, health data, creative work, personal projects
- media: Images, videos, audio that are user-created content (NOT UI assets or website graphics)
- config: Application configs, settings files, dotfiles
- website_scaffolding: HTML templates, CSS, JS bundles, emoji assets, fonts, saved webpage resources
- work: Work/corporate files, professional documents
- unknown: Cannot determine"#;

/// Criteria for skipping files (should_ingest = false).
const SKIP_CRITERIA: &str = r#"SKIP CRITERIA (should_ingest = false):
- Website scaffolding (CSS, JS bundles, images that are part of saved web pages)
- Application config files
- Source code (unless it's personal creative work)
- Cache and temporary files
- Downloaded installers/archives"#;

/// Criteria for ingesting files (should_ingest = true).
const INGEST_CRITERIA: &str = r#"INGEST CRITERIA (should_ingest = true):
- Personal documents (letters, notes, journals)
- Photos and videos (user-created, not UI assets)
- Messages and chat logs
- Financial records (statements, budgets, tax documents)
- Health data
- Creative work (writing, art, music)
- Data exports from services (Twitter, Facebook, Google Takeout, etc.)
- Personal work output (reports, presentations, research notes)"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_folder_prompt_contains_tree_and_files() {
        let prompt = build_smart_folder_prompt("docs/\n  notes.txt", &["docs/notes.txt".to_string()]);
        assert!(prompt.contains("docs/\n  notes.txt"));
        assert!(prompt.contains("docs/notes.txt"));
        assert!(prompt.contains("personal_data"));
        assert!(prompt.contains("should_ingest"));
    }

    #[test]
    fn image_directory_prompt_contains_dirs() {
        let lines = vec!["photos/vacation: 5 files [img1.jpg, img2.jpg]".to_string()];
        let prompt = build_image_directory_prompt(&lines);
        assert!(prompt.contains("photos/vacation"));
        assert!(prompt.contains("personal"));
        assert!(prompt.contains("asset"));
    }

    #[test]
    fn adjust_prompt_contains_instruction() {
        let prompt = build_adjust_prompt(
            "include all work files",
            &[r#"{"path": "a.txt", "should_ingest": true}"#.to_string()],
            &[r#"{"path": "b.txt", "should_ingest": false}"#.to_string()],
        );
        assert!(prompt.contains("include all work files"));
        assert!(prompt.contains("a.txt"));
        assert!(prompt.contains("b.txt"));
    }
}
