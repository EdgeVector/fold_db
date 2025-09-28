//! Tests for the iterator stack execution engine.
//!
//! These tests exercise the public `ExecutionEngine` API using realistic
//! declarative transform expressions. They verify that nested iterator
//! structures (such as the `split_by_word().map()` pipeline used by the
//! BlogPost → BlogPostWordIndex transform) emit the expected number of index
//! entries and surface performance warnings when large fan-outs occur.

#[cfg(test)]
mod execution_engine_tests {
    use super::super::{ExecutionEngine, ExecutionWarningType};
    use crate::transform::aggregation::aggregate_results_unified;
    use crate::transform::iterator_stack::chain_parser::{ChainParser, ParsedChain};
    use crate::schema::types::DeclarativeSchemaDefinition;
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
            .map(|entry| entry.value.as_str().unwrap_or_default().to_string())
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
            .map(|entry| entry.value.as_str().unwrap_or_default().to_string())
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

    #[test]
    fn blog_post_word_index_end_to_end_aggregation() {
        // Define the output transform schema (BlogPostWordIndex)
        let transform_schema_json = r#"{
            "name": "BlogPostWordIndex",
            "schema_type": {"HashRange": {"keyconfig": {"hash_field": null, "range_field": null}}},
            "key": {"hash_field": "word", "range_field": "publish_date"},
            "transform_fields": {
                "word": "BlogPost.map().content.split_by_word().map()",
                "publish_date": "BlogPost.map().publish_date",
                "content": "BlogPost.map().content",
                "author": "BlogPost.map().author",
                "title": "BlogPost.map().title",
                "tags": "BlogPost.map().tags"
            }
        }"#;

        let transform_schema: DeclarativeSchemaDefinition = serde_json::from_str(transform_schema_json)
            .expect("failed to parse BlogPostWordIndex schema JSON");

        // Build expressions from schema mapping (field -> expression)
        let field_to_hash = transform_schema.get_field_to_hash_code();
        let hash_to_code = transform_schema.hash_to_code();
        let expressions: Vec<(String, String)> = field_to_hash
            .iter()
            .map(|(field, hash)| (field.clone(), hash_to_code.get(hash).unwrap().clone()))
            .collect();

        // Parse chains and build map for execution
        let parsed = crate::transform::shared_utilities::parse_expressions_batch(&expressions)
            .expect("parse expressions should succeed");
        let chains_map: HashMap<String, ParsedChain> = parsed
            .iter()
            .map(|(field, chain)| (field.clone(), chain.clone()))
            .collect();

        // Input data (BlogPost schema content)
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

        // Execute engine
        let mut engine = ExecutionEngine::new();
        let exec = engine
            .execute_fields(chains_map, input_data.clone())
            .expect("execution should succeed");

        // Aggregate into rows [{ key, fields }]
        let all_expressions: Vec<(String, String)> = parsed
            .iter()
            .map(|(field, chain)| (field.clone(), chain.expression.clone()))
            .collect();
        let aggregated = aggregate_results_unified(
            &transform_schema,
            &parsed,
            &exec,
            &input_data,
            &all_expressions,
        )
        .expect("aggregation should succeed");

        // Validate rows format and fields
        let rows = aggregated.as_array().expect("result should be array of rows");
        assert!(!rows.is_empty());
        for row in rows {
            let obj = row.as_object().expect("row object");
            assert!(obj.contains_key("key"));
            assert!(obj.contains_key("fields"));
        }

        // Words across rows: collect every fields.word value
        let mut words: Vec<String> = rows
            .iter()
            .filter_map(|row| row.get("fields"))
            .filter_map(|f| f.get("word"))
            .flat_map(|v| match v {
                JsonValue::Array(arr) => arr.clone(),
                other => vec![other.clone()],
            })
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        words.sort();
        assert_eq!(
            words,
            vec![
                "Rust",
                "Tests",
                "concurrency",
                "empowers",
                "fearless",
                "iterator",
                "stacks",
                "validate",
            ]
        );

        // Publish dates across rows: collect fields.publish_date values
        let mut publish_dates: Vec<String> = rows
            .iter()
            .filter_map(|row| row.get("fields"))
            .filter_map(|f| f.get("publish_date"))
            .flat_map(|v| match v {
                JsonValue::Array(arr) => arr.clone(),
                other => vec![other.clone()],
            })
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        publish_dates.sort();
        publish_dates.dedup();
        assert_eq!(publish_dates, vec!["2024-12-31", "2025-01-05"]);

        // Authors across rows
        let mut authors: Vec<String> = rows
            .iter()
            .filter_map(|row| row.get("fields"))
            .filter_map(|f| f.get("author"))
            .flat_map(|v| match v {
                JsonValue::Array(arr) => arr.clone(),
                other => vec![other.clone()],
            })
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        authors.sort();
        authors.dedup();
        assert_eq!(authors, vec!["Carol", "Dylan"]);
    }

    #[test]
    fn performance_warning_boundary_thresholds() {
        // Exactly at threshold: no warning
        let chains = parse_chain_map(&[("value", "items.map().value")]);
        let items_1000: Vec<JsonValue> = (0..1_000)
            .map(|idx| json!({ "value": format!("item-{idx}") }))
            .collect();
        let input_1000 = build_input_map(vec![("items", json!(items_1000))]);

        let mut engine = ExecutionEngine::new();
        let res_1000 = engine
            .execute_fields(chains.clone(), input_1000)
            .expect("execution should succeed");
        assert_eq!(res_1000.index_entries["value"].len(), 1_000);
        assert!(res_1000
            .warnings
            .get("value")
            .map(|v| v.is_empty())
            .unwrap_or(true));

        // Threshold + 1: expect warning
        let items_1001: Vec<JsonValue> = (0..1_001)
            .map(|idx| json!({ "value": format!("item-{idx}") }))
            .collect();
        let input_1001 = build_input_map(vec![("items", json!(items_1001))]);
        let res_1001 = ExecutionEngine::new()
            .execute_fields(chains, input_1001)
            .expect("execution should succeed");
        assert_eq!(res_1001.index_entries["value"].len(), 1_001);
        assert!(res_1001
            .warnings["value"]
            .iter()
            .any(|w| w.warning_type == ExecutionWarningType::PerformanceDegradation));
    }
}
