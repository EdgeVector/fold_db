//! Keyword normalization utilities for native indexing.
//!
//! Keyword extraction is performed inline during ingestion
//! (see `ingestion_service.rs`). This module provides the shared
//! normalization logic used by both ingestion and tests.

/// Normalize a list of keywords: lowercase, deduplicate, split multi-word.
pub fn normalize_keywords(keywords: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut normalized: Vec<String> = Vec::new();
    for raw in keywords {
        let kw = raw.to_lowercase().trim().to_string();
        if kw.len() >= 2 && seen.insert(kw.clone()) {
            normalized.push(kw.clone());
            // Split multi-word keywords into parts
            if kw.contains(' ') {
                for part in kw.split_whitespace() {
                    let part = part.to_string();
                    if part.len() >= 2 && seen.insert(part.clone()) {
                        normalized.push(part);
                    }
                }
            }
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_keywords_basic() {
        let input = vec![
            "Rust".to_string(),
            "alice johnson".to_string(),
            "a".to_string(),  // too short
            "rust".to_string(),  // duplicate
            "".to_string(),  // empty
        ];
        let result = normalize_keywords(input);
        assert_eq!(result, vec!["rust", "alice johnson", "alice", "johnson"]);
    }
}
