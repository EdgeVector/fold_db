//! Directory scanning, tree building, binary detection, and file hashing
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
            // Skip directories that are coding projects (contain manifest files)
            if is_coding_project(&path) {
                continue;
            }
            scan_directory_recursive(root, &path, depth + 1, max_depth, max_files, files)?;
        } else if path.is_file() {
            // Get relative path from root
            if let Ok(relative) = path.strip_prefix(root) {
                let rel_str = relative.to_string_lossy().to_string();
                // Skip binary/media files so they don't consume the max_files budget
                // and prevent data files from being discovered.
                if is_never_personal_data(&rel_str) {
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

/// Files whose presence marks a directory as a coding project.
const PROJECT_MANIFEST_FILES: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "go.mod",
    "pyproject.toml",
    "setup.py",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "Gemfile",
    "composer.json",
    "CMakeLists.txt",
];

/// Returns true if the directory contains a project manifest file.
fn is_coding_project(dir: &Path) -> bool {
    PROJECT_MANIFEST_FILES.iter().any(|f| dir.join(f).exists())
}

/// Extensions for files that can never be ingested as structured data.
///
/// These are filtered out **during directory collection** so they do not consume
/// the `max_files` budget.  This prevents large media/font directories (e.g. a
/// Twitter export's `assets/images/`) from exhausting the quota before the
/// scanner reaches the actual data files (e.g. `data/tweets.js`).
const BINARY_SKIP_EXTS: &[&str] = &[
    // Compiled binaries
    "exe", "dll", "so", "dylib", "o", "a", "lib", "class", "pyc", "pyo", "beam", "wasm",
    // Fonts
    "woff", "woff2", "eot", "ttf", "otf",
    // Source maps / lock files
    "map", "lock",
    // Images — not handled by read_file_with_hash
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "ico", "tiff", "tif", "avif", "heic", "heif",
    "svg",
    // Video
    "mp4", "mov", "avi", "mkv", "webm", "m4v", "flv", "wmv", "3gp",
    // Audio
    "mp3", "wav", "ogg", "aac", "m4a", "flac", "opus", "wma",
];

/// Returns true if the file is a truly binary format that can never contain personal data.
pub fn is_never_personal_data(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    BINARY_SKIP_EXTS.contains(&ext.as_str())
}
