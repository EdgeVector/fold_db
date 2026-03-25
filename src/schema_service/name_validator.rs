//! Validation helpers for schema descriptive names.
//!
//! Detects generic or structural names that don't meaningfully describe
//! the data the schema holds (e.g. "Document Collection", "Data Set").

/// Returns `true` if `name` is a generic structural name that should be
/// replaced with a more descriptive one derived from the schema itself.
pub fn is_generic_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    let generic_words = [
        "collection",
        "dataset",
        "data set",
        "record",
        "records",
        "document",
        "documents",
        "schema",
        "table",
        "list",
        "data",
        "item",
        "items",
        "object",
        "objects",
        "entity",
        "entities",
        "entry",
        "entries",
    ];
    // A name is considered generic if it consists only of generic words
    // (case-insensitive, ignoring whitespace).
    let words: Vec<&str> = lower.split_whitespace().collect();
    !words.is_empty() && words.iter().all(|w| generic_words.contains(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_names_detected() {
        assert!(is_generic_name("Document Collection"));
        assert!(is_generic_name("Data Set"));
        assert!(is_generic_name("Records"));
        assert!(is_generic_name("data"));
    }

    #[test]
    fn specific_names_not_flagged() {
        assert!(!is_generic_name("Twitter Posts"));
        assert!(!is_generic_name("Medical Records"));
        assert!(!is_generic_name("Nature Photography"));
        assert!(!is_generic_name("Employee Salaries"));
    }
}
