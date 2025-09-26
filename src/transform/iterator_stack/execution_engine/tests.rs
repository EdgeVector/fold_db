//! Tests for the iterator stack execution engine.
//!
//! These tests exercise the public `ExecutionEngine` API using realistic
//! declarative transform expressions. They verify that nested iterator
//! structures (such as the `split_by_word().map()` pipeline used by the
//! BlogPost → BlogPostWordIndex transform) emit the expected number of index
//! entries and surface performance warnings when large fan-outs occur.

#[cfg(test)]
mod tests {
    use super::super::{ExecutionEngine, ExecutionWarningType};
    use crate::transform::iterator_stack::chain_parser::{ChainParser, ParsedChain};
    use serde_json::{json, Value as JsonValue};
    use std::collections::HashMap;

    /// Helper to parse a collection of (field, expression) pairs into the map
    /// expected by `ExecutionEngine::execute_fields`.
    fn parse_chain_map(pairs: &[(&str, &str)]) -> HashMap<String, ParsedChain> {
        let parser = ChainParser::new();
        pairs
            .iter()
            .map(|(field, expression)| {
                let parsed = parser
                    .parse(expression)
                    .unwrap_or_else(|err| panic!("Failed to parse '{}': {}", expression, err));
                ((*field).to_string(), parsed)
            })
            .collect()
    }

    /// Helper to convert simple (&str, JsonValue) pairs into the input map
    /// consumed by the execution engine.
    fn build_input_map(pairs: Vec<(&str, JsonValue)>) -> HashMap<String, JsonValue> {
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect()
    }

    #[test]
    fn blog_post_word_index_produces_entries_for_each_word() {
        let chains = parse_chain_map(&[
            ("word", "BlogPost.map().content.split_by_word().map()"),
            ("publish_date", "BlogPost.map().publish_date"),
        ]);

        let input_data = build_input_map(vec![(
            "BlogPost",
            json!([
                {
                    "title": "First",
                    "content": "Rust empowers fearless concurrency",
                    "author": "Carol",
                    "publish_date": "2024-12-31",
                    "tags": ["rust", "systems"],
                },
                {
                    "title": "Second",
                    "content": "Tests validate iterator stacks",
                    "author": "Dylan",
                    "publish_date": "2025-01-05",
                    "tags": ["testing", "rust"],
                }
            ]),
        )]);

        let mut engine = ExecutionEngine::new();
        let result = engine
            .execute_fields(chains, input_data)
            .expect("execution should succeed");

        let word_entries = result
            .index_entries
            .get("word")
            .expect("word field entries should be present");
        let produced_words: Vec<String> = word_entries
            .iter()
            .map(|entry| entry.hash_value.as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(
            produced_words,
            vec![
                "Rust",
                "empowers",
                "fearless",
                "concurrency",
                "Tests",
                "validate",
                "iterator",
                "stacks"
            ],
            "all words should be emitted across nested iterators"
        );

        let publish_date_entries = result
            .index_entries
            .get("publish_date")
            .expect("publish_date entries should be present");
        let publish_dates: Vec<String> = publish_date_entries
            .iter()
            .map(|entry| entry.hash_value.as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(publish_dates, vec!["2024-12-31", "2025-01-05"]);

        assert!(
            result
                .warnings
                .get("word")
                .map(Vec::is_empty)
                .unwrap_or(true),
            "word execution should not emit warnings"
        );
    }

    #[test]
    fn performance_warning_triggered_for_large_fanout() {
        let chains = parse_chain_map(&[("value", "items.map().value")]);

        let large_input: Vec<JsonValue> = (0..1_200)
            .map(|idx| json!({ "value": format!("item-{idx}") }))
            .collect();
        let input_data = build_input_map(vec![("items", json!(large_input))]);

        let mut engine = ExecutionEngine::new();
        let result = engine
            .execute_fields(chains, input_data)
            .expect("execution should succeed for large fan-out");

        let entries = result
            .index_entries
            .get("value")
            .expect("value entries should be present");
        assert_eq!(entries.len(), 1_200);

        let warnings = result.warnings.get("value").expect("warnings entry");
        assert!(warnings
            .iter()
            .any(|warning| warning.warning_type == ExecutionWarningType::PerformanceDegradation));
    }
}
