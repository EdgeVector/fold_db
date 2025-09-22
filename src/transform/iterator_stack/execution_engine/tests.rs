//! Tests for the execution engine

#[allow(unused_imports)]
use crate::transform::iterator_stack::chain_parser::{ChainOperation, ParsedChain};
#[allow(unused_imports)]
use crate::transform::iterator_stack::field_alignment::{
    AlignmentValidationResult, FieldAlignmentInfo,
};
#[allow(unused_imports)]
use crate::transform::iterator_stack::{ExecutionEngine, ExecutionWarningType};
#[allow(unused_imports)]
use log::debug;
#[allow(unused_imports)]
use serde_json::json;
#[allow(unused_imports)]
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_field_execution() {
        let mut engine = ExecutionEngine::new();

        // Create a simple chain
        let chain = ParsedChain {
            expression: "user.name".to_string(),
            operations: vec![
                ChainOperation::FieldAccess("user".to_string()),
                ChainOperation::FieldAccess("name".to_string()),
            ],
            depth: 0,
            branch: "main".to_string(),
            scopes: vec![],
        };

        // Create alignment result
        let mut field_alignments = HashMap::new();
        field_alignments.insert(
            "user.name".to_string(),
            FieldAlignmentInfo {
                expression: "user.name".to_string(),
                depth: 0,
                alignment: crate::transform::iterator_stack::chain_parser::FieldAlignment::OneToOne,
                branch: "main".to_string(),
                requires_reducer: false,
                suggested_reducer: None,
            },
        );

        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: 0,
            field_alignments,
            errors: vec![],
            warnings: vec![],
        };

        // Create input data
        let input_data = json!({
            "user": {
                "name": "John Doe",
                "email": "john@example.com"
            }
        });

        let result = engine
            .execute_fields(&[chain], &alignment_result, input_data)
            .unwrap();

        assert!(!result.index_entries.is_empty());
        assert_eq!(result.index_entries[0].hash_value, json!("John Doe"));
    }

    #[test]
    fn test_broadcast_execution() {
        let mut engine = ExecutionEngine::new();

        // Create a chain that will broadcast across an array
        let chain1 = ParsedChain {
            expression: "users.name".to_string(),
            operations: vec![
                ChainOperation::FieldAccess("users".to_string()),
                ChainOperation::FieldAccess("name".to_string()),
            ],
            depth: 1,
            branch: "main".to_string(),
            scopes: vec![],
        };

        let chain2 = ParsedChain {
            expression: "users.email".to_string(),
            operations: vec![
                ChainOperation::FieldAccess("users".to_string()),
                ChainOperation::FieldAccess("email".to_string()),
            ],
            depth: 1,
            branch: "main".to_string(),
            scopes: vec![],
        };

        // Create alignment result
        let mut field_alignments = HashMap::new();
        field_alignments.insert(
            "users.name".to_string(),
            FieldAlignmentInfo {
                expression: "users.name".to_string(),
                depth: 1,
                alignment:
                    crate::transform::iterator_stack::chain_parser::FieldAlignment::Broadcast,
                branch: "main".to_string(),
                requires_reducer: false,
                suggested_reducer: None,
            },
        );
        field_alignments.insert(
            "users.email".to_string(),
            FieldAlignmentInfo {
                expression: "users.email".to_string(),
                depth: 1,
                alignment:
                    crate::transform::iterator_stack::chain_parser::FieldAlignment::Broadcast,
                branch: "main".to_string(),
                requires_reducer: false,
                suggested_reducer: None,
            },
        );

        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: 1,
            field_alignments,
            errors: vec![],
            warnings: vec![],
        };

        // Create input data with array
        let input_data = json!({
            "users": [
                {
                    "name": "John Doe",
                    "email": "john@example.com"
                },
                {
                    "name": "Jane Smith",
                    "email": "jane@example.com"
                }
            ]
        });

        let result = engine
            .execute_fields(&[chain1, chain2], &alignment_result, input_data)
            .unwrap();

        debug!(
            "Broadcast test - Index entries count: {}",
            result.index_entries.len()
        );
        debug!(
            "Broadcast test - Items per depth: {:?}",
            result.statistics.items_per_depth
        );
        debug!(
            "Broadcast test - Alignment result valid: {}",
            alignment_result.valid
        );
        debug!("Broadcast test - Max depth: {}", alignment_result.max_depth);

        assert!(!result.index_entries.is_empty());
        assert_eq!(result.statistics.items_per_depth.get(&1), Some(&4)); // 2 users × 2 fields = 4 entries at depth 1
    }

    #[test]
    fn test_shared_prefix_iterator_cache_hits() {
        let mut engine = ExecutionEngine::new();
        let parser = crate::transform::iterator_stack::chain_parser::ChainParser::new();

        let chain_words = parser
            .parse("blogpost.map().content.split_by_word().map()")
            .unwrap();
        let chain_author = parser.parse("blogpost.map().author").unwrap();

        let mut field_alignments = HashMap::new();
        field_alignments.insert(
            chain_words.expression.clone(),
            FieldAlignmentInfo {
                expression: chain_words.expression.clone(),
                depth: chain_words.depth,
                alignment: crate::transform::iterator_stack::chain_parser::FieldAlignment::OneToOne,
                branch: chain_words.branch.clone(),
                requires_reducer: false,
                suggested_reducer: None,
            },
        );
        field_alignments.insert(
            chain_author.expression.clone(),
            FieldAlignmentInfo {
                expression: chain_author.expression.clone(),
                depth: chain_author.depth,
                alignment:
                    crate::transform::iterator_stack::chain_parser::FieldAlignment::Broadcast,
                branch: chain_author.branch.clone(),
                requires_reducer: false,
                suggested_reducer: None,
            },
        );

        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: chain_words.depth.max(chain_author.depth),
            field_alignments,
            errors: vec![],
            warnings: vec![],
        };

        let input_data = json!({
            "blogpost": [
                {
                    "author": "Alice",
                    "content": "Hello world from Alice"
                },
                {
                    "author": "Bob",
                    "content": "Bob writes again"
                }
            ]
        });

        let result = engine
            .execute_fields(&[chain_words, chain_author], &alignment_result, input_data)
            .unwrap();

        assert_eq!(result.statistics.cache_hits, 1);
        assert_eq!(result.statistics.cache_misses, 2);
        assert!(
            !result.index_entries.is_empty(),
            "Expected index entries from cached execution"
        );
    }

    #[test]
    fn test_word_split_with_fields_wrapper() {
        let mut engine = ExecutionEngine::new();
        let parser = crate::transform::iterator_stack::chain_parser::ChainParser::new();

        let chain = parser
            .parse("BlogPost.map().fields.content.split_by_word().map()")
            .expect("Failed to parse chain expression");

        let mut field_alignments = HashMap::new();
        field_alignments.insert(
            chain.expression.clone(),
            FieldAlignmentInfo {
                expression: chain.expression.clone(),
                depth: chain.depth,
                alignment: crate::transform::iterator_stack::chain_parser::FieldAlignment::OneToOne,
                branch: chain.branch.clone(),
                requires_reducer: false,
                suggested_reducer: None,
            },
        );

        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: chain.depth,
            field_alignments,
            errors: vec![],
            warnings: vec![],
        };

        let input_data = json!({
            "BlogPost": [
                {
                    "fields": {
                        "content": "Split these words correctly",
                        "publish_date": "2025-01-01T00:00:00Z"
                    },
                    "hash": null,
                    "range": null
                }
            ]
        });

        let result = engine
            .execute_fields(&[chain], &alignment_result, input_data)
            .expect("Execution engine should produce word entries");

        assert_eq!(result.index_entries.len(), 4, "Expected four word entries");
        let words: Vec<_> = result
            .index_entries
            .iter()
            .map(|entry| entry.hash_value.as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(words, vec!["Split", "these", "words", "correctly"]);
    }

    #[test]
    fn test_word_split_with_input_wrapper() {
        let mut engine = ExecutionEngine::new();
        let parser = crate::transform::iterator_stack::chain_parser::ChainParser::new();

        let chain = parser
            .parse("input.BlogPost.map().fields.content.split_by_word().map()")
            .expect("Failed to parse chain expression with input prefix");

        let mut field_alignments = HashMap::new();
        field_alignments.insert(
            chain.expression.clone(),
            FieldAlignmentInfo {
                expression: chain.expression.clone(),
                depth: chain.depth,
                alignment: crate::transform::iterator_stack::chain_parser::FieldAlignment::OneToOne,
                branch: chain.branch.clone(),
                requires_reducer: false,
                suggested_reducer: None,
            },
        );

        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: chain.depth,
            field_alignments,
            errors: vec![],
            warnings: vec![],
        };

        let input_data = json!({
            "input": {
                "BlogPost": [
                    {
                        "fields": {
                            "content": "Handle words from nested input",
                            "publish_date": "2025-01-01T00:00:00Z"
                        },
                        "hash": null,
                        "range": null
                    }
                ]
            }
        });

        let result = engine
            .execute_fields(&[chain], &alignment_result, input_data)
            .expect("Execution engine should produce word entries with input wrapper");

        assert_eq!(result.index_entries.len(), 5, "Expected five word entries");
        let words: Vec<_> = result
            .index_entries
            .iter()
            .map(|entry| entry.hash_value.as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(
            words,
            vec![
                "Handle".to_string(),
                "words".to_string(),
                "from".to_string(),
                "nested".to_string(),
                "input".to_string()
            ]
        );
    }

    #[test]
    fn test_execution_warnings() {
        let mut engine = ExecutionEngine::new();

        // Use the chain parser to create the chain properly
        let parser = crate::transform::iterator_stack::chain_parser::ChainParser::new();
        let chain = parser.parse("items.value").unwrap();

        // Create alignment result
        let mut field_alignments = HashMap::new();
        field_alignments.insert(
            "items.value".to_string(),
            FieldAlignmentInfo {
                expression: "items.value".to_string(),
                depth: 1,
                alignment:
                    crate::transform::iterator_stack::chain_parser::FieldAlignment::Broadcast,
                branch: "main".to_string(),
                requires_reducer: false,
                suggested_reducer: None,
            },
        );

        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: 1,
            field_alignments,
            errors: vec![],
            warnings: vec![],
        };

        // Create input data with many items
        let mut items = Vec::new();
        for i in 0..1500 {
            items.push(json!({
                "id": i,
                "value": format!("item_{}", i)
            }));
        }

        let input_data = json!({
            "items": items
        });

        let result = engine
            .execute_fields(&[chain], &alignment_result, input_data)
            .unwrap();

        debug!(
            "Warnings test - Index entries count: {}",
            result.index_entries.len()
        );
        debug!("Warnings test - Warnings count: {}", result.warnings.len());
        debug!(
            "Warnings test - Items per depth: {:?}",
            result.statistics.items_per_depth
        );

        // Should have warnings due to high entry count
        // Temporarily check entry count first
        assert!(
            result.index_entries.len() > 1000,
            "Expected more than 1000 entries, got {}",
            result.index_entries.len()
        );
        assert!(!result.warnings.is_empty());
        assert!(result
            .warnings
            .iter()
            .any(|w| matches!(w.warning_type, ExecutionWarningType::PerformanceDegradation)));
    }

    #[test]
    fn test_simple_array_iteration() {
        let mut engine = ExecutionEngine::new();

        // Use the chain parser to create the chain properly
        let parser = crate::transform::iterator_stack::chain_parser::ChainParser::new();
        let chain = parser.parse("items.value").unwrap();

        debug!("Chain scopes: {:?}", chain.scopes);
        debug!("Chain depth: {}", chain.depth);
        debug!("Chain operations: {:?}", chain.operations);

        // Create alignment result
        let mut field_alignments = HashMap::new();
        field_alignments.insert(
            "items.value".to_string(),
            FieldAlignmentInfo {
                expression: "items.value".to_string(),
                depth: 1,
                alignment:
                    crate::transform::iterator_stack::chain_parser::FieldAlignment::Broadcast,
                branch: "main".to_string(),
                requires_reducer: false,
                suggested_reducer: None,
            },
        );

        let alignment_result = AlignmentValidationResult {
            valid: true,
            max_depth: 1,
            field_alignments,
            errors: vec![],
            warnings: vec![],
        };

        // Create simple input data
        let input_data = json!({
            "items": [
                {"value": "item1"},
                {"value": "item2"},
                {"value": "item3"}
            ]
        });

        let result = engine
            .execute_fields(&[chain], &alignment_result, input_data)
            .unwrap();

        debug!(
            "Simple array test - Index entries count: {}",
            result.index_entries.len()
        );
        debug!(
            "Simple array test - Items per depth: {:?}",
            result.statistics.items_per_depth
        );

        // Should have 3 entries (one for each item)
        assert_eq!(result.index_entries.len(), 3);
    }
}
