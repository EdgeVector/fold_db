//! Directory scanning, tree building, extension whitelist, and file hashing
//! for the smart folder feature.

use crate::ingestion::error::IngestionError;
use crate::ingestion::IngestionResult;
use std::collections::{BTreeSet, HashSet};
use std::path::Path;

/// Result of scanning a directory tree with context for LLM classification.
pub struct DirectoryScanResult {
    /// Flat list of relative file paths for processing
    pub file_paths: Vec<String>,
    /// Indented tree display for LLM context
    pub tree_display: String,
    /// Whether the scan was truncated due to reaching max_files
    pub truncated: bool,
}

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
                let rel_str = relative.to_string_lossy().to_string();
                // Only collect files with ingestible extensions so non-ingestible
                // files don't consume the max_files budget.
                if !is_ingestible_file(&rel_str) {
                    continue;
                }
                files.push(rel_str);
            }
        }
    }

    Ok(())
}

/// Build an indented directory tree string from a list of relative file paths.
pub fn build_directory_tree_string(file_paths: &[String]) -> String {
    // Collect all directory prefixes and files in sorted order
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    let mut all_paths: BTreeSet<String> = BTreeSet::new();

    for path in file_paths {
        all_paths.insert(path.clone());
        let p = Path::new(path);
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
    let mut entries: Vec<(String, bool)> = Vec::new();
    for d in &dirs {
        entries.push((d.clone(), true));
    }
    for f in &all_paths {
        entries.push((f.clone(), false));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

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

/// Compute SHA256 hash of a file's raw bytes (for dedup checking).
pub fn compute_file_hash(file_path: &Path) -> IngestionResult<String> {
    use sha2::{Digest, Sha256};
    let raw_bytes = std::fs::read(file_path).map_err(|e| {
        IngestionError::InvalidInput(format!("Failed to read file for hashing: {}", e))
    })?;
    Ok(format!("{:x}", Sha256::digest(&raw_bytes)))
}

/// Data file extensions (JSON, CSV, plain text, markdown).
pub const DATA_EXTS: &[&str] = &["json", "csv", "txt", "md"];

/// Document file extensions (personal data — handled by LLM classifier).
pub const DOC_EXTS: &[&str] = &[
    "pdf", "doc", "docx", "rtf", "odt", "pages",
    "xls", "xlsx", "ods", "numbers",
    "pptx", "ppt", "odp", "key",
    "eml", "mbox", "vcf",
];

/// Code file extensions handled by `extract_code_metadata`.
pub const CODE_EXTS: &[&str] = &[
    "js", "jsx", "ts", "tsx", "py", "rs", "go", "java", "kt", "rb",
    "c", "cpp", "h", "hpp", "cs", "swift", "scala", "lua", "r", "pl",
    "sh", "bash", "zsh",
];

/// Config file extensions wrapped as text content.
pub const CONFIG_EXTS: &[&str] = &["yaml", "yml", "toml", "xml"];

/// Returns true if the file has an extension we can ingest (data, code, or config).
pub fn is_ingestible_file(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let e = ext.as_str();
    DATA_EXTS.contains(&e) || DOC_EXTS.contains(&e) || CODE_EXTS.contains(&e) || CONFIG_EXTS.contains(&e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::Write;

    #[test]
    fn test_compute_file_hash_known_content() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        let content = b"hello world";
        tmp.write_all(content).unwrap();

        let hash = compute_file_hash(tmp.path()).unwrap();
        let expected = format!("{:x}", Sha256::digest(content));
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_compute_file_hash_same_content_identical() {
        let content = b"identical content for hashing";

        let mut tmp1 = tempfile::NamedTempFile::new().unwrap();
        tmp1.write_all(content).unwrap();

        let mut tmp2 = tempfile::NamedTempFile::new().unwrap();
        tmp2.write_all(content).unwrap();

        let hash1 = compute_file_hash(tmp1.path()).unwrap();
        let hash2 = compute_file_hash(tmp2.path()).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_file_hash_nonexistent_file() {
        let result = compute_file_hash(Path::new("/tmp/nonexistent_hash_test_abc.txt"));
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Failed to read file for hashing"));
    }

    // ---- is_ingestible_file tests ----

    #[test]
    fn test_data_files_are_ingestible() {
        assert!(is_ingestible_file("data.json"));
        assert!(is_ingestible_file("records.csv"));
        assert!(is_ingestible_file("notes.txt"));
        assert!(is_ingestible_file("readme.md"));
    }

    #[test]
    fn test_document_files_are_ingestible() {
        assert!(is_ingestible_file("report.pdf"));
        assert!(is_ingestible_file("letter.docx"));
        assert!(is_ingestible_file("budget.xlsx"));
        assert!(is_ingestible_file("contacts.vcf"));
        assert!(is_ingestible_file("mail.eml"));
    }

    #[test]
    fn test_code_files_are_ingestible() {
        assert!(is_ingestible_file("app.js"));
        assert!(is_ingestible_file("component.tsx"));
        assert!(is_ingestible_file("main.py"));
        assert!(is_ingestible_file("lib.rs"));
        assert!(is_ingestible_file("main.go"));
        assert!(is_ingestible_file("App.java"));
        assert!(is_ingestible_file("script.sh"));
        assert!(is_ingestible_file("code.c"));
        assert!(is_ingestible_file("code.cpp"));
        assert!(is_ingestible_file("header.h"));
    }

    #[test]
    fn test_config_files_are_ingestible() {
        assert!(is_ingestible_file("config.yaml"));
        assert!(is_ingestible_file("config.yml"));
        assert!(is_ingestible_file("settings.toml"));
        assert!(is_ingestible_file("data.xml"));
    }

    #[test]
    fn test_binary_files_not_ingestible() {
        assert!(!is_ingestible_file("program.exe"));
        assert!(!is_ingestible_file("lib/native.so"));
        assert!(!is_ingestible_file("module.dll"));
        assert!(!is_ingestible_file("code.class"));
        assert!(!is_ingestible_file("script.pyc"));
        assert!(!is_ingestible_file("app.wasm"));
    }

    #[test]
    fn test_media_files_not_ingestible() {
        assert!(!is_ingestible_file("photo.jpg"));
        assert!(!is_ingestible_file("photo.jpeg"));
        assert!(!is_ingestible_file("image.png"));
        assert!(!is_ingestible_file("image.gif"));
        assert!(!is_ingestible_file("icon.svg"));
        assert!(!is_ingestible_file("video.mp4"));
        assert!(!is_ingestible_file("song.mp3"));
        assert!(!is_ingestible_file("audio.wav"));
    }

    #[test]
    fn test_font_files_not_ingestible() {
        assert!(!is_ingestible_file("font.woff"));
        assert!(!is_ingestible_file("font.woff2"));
        assert!(!is_ingestible_file("font.ttf"));
        assert!(!is_ingestible_file("font.otf"));
        assert!(!is_ingestible_file("font.eot"));
    }

    #[test]
    fn test_no_extension_not_ingestible() {
        assert!(!is_ingestible_file("README"));
        assert!(!is_ingestible_file("Makefile"));
    }
}
