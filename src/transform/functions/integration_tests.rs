//! Integration tests for transform functions with the typed engine
//!
//! These tests verify that functions work correctly in the full transformation pipeline,
//! including chain parsing, spec generation, and engine execution.

use super::*;
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::transform::chain_parser::parser::ChainParser;
use crate::transform::iterator_stack_typed::engine::TypedEngine;
use crate::transform::iterator_stack_typed::types::{IteratorSpec, TypedInput};
use std::collections::HashMap;

// ============================================================================
// Chain Parser Integration Tests
// ============================================================================

#[test]
fn test_chain_parser_with_all_functions() {
    let parser = ChainParser::new();

    // Test all iterator functions
    let iterator_expressions = vec!["content.split_by_word()", "data.split_array()"];

    for expr in iterator_expressions {
        let result = parser.parse(expr);
        assert!(
            result.is_ok(),
            "Failed to parse iterator expression: {}",
            expr
        );

        let parsed_chain = result.unwrap();
        assert!(!parsed_chain.operations.is_empty());

        // Verify the last operation is a function
        if let Some(last_op) = parsed_chain.operations.last() {
            match last_op {
                crate::transform::chain_parser::types::ChainOperation::Function {
                    name, ..
                } => {
                    let reg = registry();
                    assert!(
                        reg.is_iterator(name),
                        "Function {} should be registered as iterator",
                        name
                    );
                }
                _ => panic!("Expected Function operation for expression: {}", expr),
            }
        }
    }

    // Test all reducer functions
    let reducer_expressions = vec![
        "content.count()",
        "content.join()",
        "content.first()",
        "content.last()",
        "content.sum()",
        "content.max()",
        "content.min()",
    ];

    for expr in reducer_expressions {
        let result = parser.parse(expr);
        assert!(
            result.is_ok(),
            "Failed to parse reducer expression: {}",
            expr
        );

        let parsed_chain = result.unwrap();
        assert!(!parsed_chain.operations.is_empty());

        // Verify the last operation is a function
        if let Some(last_op) = parsed_chain.operations.last() {
            match last_op {
                crate::transform::chain_parser::types::ChainOperation::Function {
                    name, ..
                } => {
                    let reg = registry();
                    assert!(
                        reg.is_reducer(name),
                        "Function {} should be registered as reducer",
                        name
                    );
                }
                _ => panic!("Expected Function operation for expression: {}", expr),
            }
        }
    }
}

#[test]
fn test_chain_parser_iterator_reducer_combinations() {
    let parser = ChainParser::new();

    let valid_combinations = vec![
        "content.split_by_word().count()",
        "content.split_by_word().join()",
        "content.split_by_word().first()",
        "content.split_by_word().last()",
        "data.split_array().sum()",
        "data.split_array().max()",
        "data.split_array().min()",
        "data.split_array().count()",
    ];

    for expr in valid_combinations {
        let result = parser.parse(expr);
        assert!(
            result.is_ok(),
            "Failed to parse valid combination: {}",
            expr
        );

        let parsed_chain = result.unwrap();
        assert_eq!(parsed_chain.operations.len(), 3); // FieldAccess, Iterator, Reducer

        // Verify operation sequence
        match &parsed_chain.operations[..] {
            [crate::transform::chain_parser::types::ChainOperation::FieldAccess(_), crate::transform::chain_parser::types::ChainOperation::Function {
                name: iterator_name,
                ..
            }, crate::transform::chain_parser::types::ChainOperation::Function {
                name: reducer_name,
                ..
            }] => {
                let reg = registry();
                assert!(
                    reg.is_iterator(iterator_name),
                    "First function should be iterator: {}",
                    iterator_name
                );
                assert!(
                    reg.is_reducer(reducer_name),
                    "Second function should be reducer: {}",
                    reducer_name
                );
            }
            _ => panic!("Unexpected operation sequence for: {}", expr),
        }
    }
}

#[test]
fn test_chain_parser_invalid_combinations() {
    let parser = ChainParser::new();

    let invalid_combinations = vec![
        "content.count().split_by_word()", // reducer -> iterator
        "content.count().count()",         // reducer -> reducer
    ];

    for expr in invalid_combinations {
        let result = parser.parse(expr);
        assert!(
            result.is_err(),
            "Should reject invalid combination: {}",
            expr
        );
    }
}

// ============================================================================
// Engine Integration Tests
// ============================================================================

#[test]
fn test_engine_executes_iterator_functions() {
    let engine = TypedEngine::new();

    // Create simple input like the working tests
    let mut input: TypedInput = HashMap::new();
    let mut field_map: HashMap<KeyValue, FieldValue> = HashMap::new();
    field_map.insert(
        KeyValue::new(Some("content".to_string()), None),
        FieldValue {
            value: serde_json::Value::String("hello world test".to_string()),
            atom_uuid: "atom-1".to_string(),
            source_file_name: None,
        },
    );
    input.insert("BlogPost.content".to_string(), field_map);

    // Create specs manually like the working tests
    let specs = vec![
        IteratorSpec::Schema {
            field_name: "BlogPost.content".to_string(),
        },
        IteratorSpec::IteratorFunction {
            name: "split_by_word".to_string(),
            params: Vec::new(),
            field_name: "BlogPost.content".to_string(),
        },
    ];

    let result = engine.execute_chain(&specs, &input, "BlogPostWordIndex.word");

    assert!(result.contains_key("BlogPostWordIndex.word"));
    let entries = &result["BlogPostWordIndex.word"];
    assert_eq!(entries.len(), 3); // "hello", "world", "test"

    // Verify all words are present
    let values: Vec<String> = entries
        .iter()
        .filter_map(|entry| entry.value_text.clone())
        .collect();

    assert!(values.contains(&"hello".to_string()));
    assert!(values.contains(&"world".to_string()));
    assert!(values.contains(&"test".to_string()));
}
