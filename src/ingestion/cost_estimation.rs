//! File ingestion cost estimation utilities.

use std::path::Path;

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
