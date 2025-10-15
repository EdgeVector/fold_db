//! Integration tests for transform functions with the typed engine
//!
//! These tests verify that functions work correctly in the full transformation pipeline,
//! including chain parsing, spec generation, and engine execution.

use super::*;
use crate::transform::chain_parser::parser::ChainParser;
use crate::transform::iterator_stack_typed::adapter::map_chain_to_specs;
use crate::transform::iterator_stack_typed::engine::TypedEngine;
use crate::transform::iterator_stack_typed::types::{IteratorSpec, TypedInput};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::field::FieldValue;
use std::collections::HashMap;

/// Helper to create test input data
fn create_test_input() -> TypedInput {
    let mut input: TypedInput = HashMap::new();
    
    // Add blog post data - each KeyValue represents a unique item
    let mut blogpost_data = HashMap::new();
    blogpost_data.insert(
        KeyValue::new(Some("content".to_string()), None),
        FieldValue {
            value: serde_json::Value::String("hello world test".to_string()),
            atom_uuid: "atom-1".to_string(),
        }
    );
    input.insert("BlogPost.content".to_string(), blogpost_data);
    
    input
}

fn create_numeric_input() -> TypedInput {
    let mut input: TypedInput = HashMap::new();
    
    // Create multiple numeric items
    let mut scores_data = HashMap::new();
    scores_data.insert(
        KeyValue::new(Some("values".to_string()), Some("item1".to_string())),
        FieldValue {
            value: serde_json::Value::Number(serde_json::Number::from(10)),
            atom_uuid: "atom-1".to_string(),
        }
    );
    scores_data.insert(
        KeyValue::new(Some("values".to_string()), Some("item2".to_string())),
        FieldValue {
            value: serde_json::Value::Number(serde_json::Number::from(20)),
            atom_uuid: "atom-2".to_string(),
        }
    );
    scores_data.insert(
        KeyValue::new(Some("values".to_string()), Some("item3".to_string())),
        FieldValue {
            value: serde_json::Value::Number(serde_json::Number::from(30)),
            atom_uuid: "atom-3".to_string(),
        }
    );
    input.insert("Scores.values".to_string(), scores_data);
    
    input
}

fn create_array_input() -> TypedInput {
    let mut input: TypedInput = HashMap::new();
    
    // Create multiple string items
    let mut tags_data = HashMap::new();
    tags_data.insert(
        KeyValue::new(Some("values".to_string()), Some("tag1".to_string())),
        FieldValue {
            value: serde_json::Value::String("rust".to_string()),
            atom_uuid: "atom-1".to_string(),
        }
    );
    tags_data.insert(
        KeyValue::new(Some("values".to_string()), Some("tag2".to_string())),
        FieldValue {
            value: serde_json::Value::String("database".to_string()),
            atom_uuid: "atom-2".to_string(),
        }
    );
    tags_data.insert(
        KeyValue::new(Some("values".to_string()), Some("tag3".to_string())),
        FieldValue {
            value: serde_json::Value::String("transforms".to_string()),
            atom_uuid: "atom-3".to_string(),
        }
    );
    input.insert("Scores.values".to_string(), tags_data);
    
    input
}

// ============================================================================
// Chain Parser Integration Tests
// ============================================================================

#[test]
fn test_chain_parser_with_all_functions() {
    let parser = ChainParser::new();
    
    // Test all iterator functions
    let iterator_expressions = vec![
        "content.split_by_word()",
        "data.split_array()",
    ];
    
    for expr in iterator_expressions {
        let result = parser.parse(expr);
        assert!(result.is_ok(), "Failed to parse iterator expression: {}", expr);
        
        let parsed_chain = result.unwrap();
        assert!(!parsed_chain.operations.is_empty());
        
        // Verify the last operation is a function
        if let Some(last_op) = parsed_chain.operations.last() {
            match last_op {
                crate::transform::chain_parser::types::ChainOperation::Function { name, .. } => {
                    let reg = registry();
                    assert!(reg.is_iterator(name), "Function {} should be registered as iterator", name);
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
        assert!(result.is_ok(), "Failed to parse reducer expression: {}", expr);
        
        let parsed_chain = result.unwrap();
        assert!(!parsed_chain.operations.is_empty());
        
        // Verify the last operation is a function
        if let Some(last_op) = parsed_chain.operations.last() {
            match last_op {
                crate::transform::chain_parser::types::ChainOperation::Function { name, .. } => {
                    let reg = registry();
                    assert!(reg.is_reducer(name), "Function {} should be registered as reducer", name);
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
        assert!(result.is_ok(), "Failed to parse valid combination: {}", expr);
        
        let parsed_chain = result.unwrap();
        assert_eq!(parsed_chain.operations.len(), 3); // FieldAccess, Iterator, Reducer
        
        // Verify operation sequence
        match &parsed_chain.operations[..] {
            [
                crate::transform::chain_parser::types::ChainOperation::FieldAccess(_),
                crate::transform::chain_parser::types::ChainOperation::Function { name: iterator_name, .. },
                crate::transform::chain_parser::types::ChainOperation::Function { name: reducer_name, .. },
            ] => {
                let reg = registry();
                assert!(reg.is_iterator(iterator_name), "First function should be iterator: {}", iterator_name);
                assert!(reg.is_reducer(reducer_name), "Second function should be reducer: {}", reducer_name);
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
        "content.split_by_word().split_array()", // iterator -> iterator
    ];
    
    for expr in invalid_combinations {
        let result = parser.parse(expr);
        assert!(result.is_err(), "Should reject invalid combination: {}", expr);
    }
}

// ============================================================================
// Adapter Integration Tests
// ============================================================================

#[test]
#[ignore]
fn test_adapter_generates_correct_specs() {
    let parser = ChainParser::new();
    
    // Test iterator-only chain
    let iterator_chain = parser
        .parse("BlogPost.content.split_by_word()")
        .expect("Should parse");
    let iterator_specs = map_chain_to_specs(&iterator_chain);

    assert_eq!(iterator_specs.len(), 2);
    match &iterator_specs[0] {
        IteratorSpec::Schema { field_name } => {
            assert_eq!(field_name, "BlogPost.content");
        }
        _ => panic!("Expected Schema spec"),
    }
    match &iterator_specs[1] {
        IteratorSpec::IteratorFunction { name, .. } => {
            assert_eq!(name, "split_by_word");
        }
        _ => panic!("Expected IteratorFunction spec"),
    }

    // Test reducer-only chain
    let reducer_chain = parser
        .parse("BlogPost.content.count()")
        .expect("Should parse");
    let reducer_specs = map_chain_to_specs(&reducer_chain);

    assert_eq!(reducer_specs.len(), 2);
    match &reducer_specs[0] {
        IteratorSpec::Schema { field_name } => {
            assert_eq!(field_name, "BlogPost.content");
        }
        _ => panic!("Expected Schema spec"),
    }
    match &reducer_specs[1] {
        IteratorSpec::ReducerFunction { name, .. } => {
            assert_eq!(name, "count");
        }
        _ => panic!("Expected ReducerFunction spec"),
    }

    // Test iterator -> reducer chain
    let combined_chain = parser
        .parse("BlogPost.content.split_by_word().count()")
        .expect("Should parse");
    let combined_specs = map_chain_to_specs(&combined_chain);

    assert_eq!(combined_specs.len(), 3);
    match &combined_specs[0] {
        IteratorSpec::Schema { field_name } => {
            assert_eq!(field_name, "BlogPost.content");
        }
        _ => panic!("Expected Schema spec"),
    }
    match &combined_specs[1] {
        IteratorSpec::IteratorFunction { name, .. } => {
            assert_eq!(name, "split_by_word");
        }
        _ => panic!("Expected IteratorFunction as second spec"),
    }
    match &combined_specs[2] {
        IteratorSpec::ReducerFunction { name, .. } => {
            assert_eq!(name, "count");
        }
        _ => panic!("Expected ReducerFunction as third spec"),
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
        }
    );
    input.insert("BlogPost.content".to_string(), field_map);

    // Create specs manually like the working tests
    let specs = vec![
        IteratorSpec::Schema { field_name: "BlogPost.content".to_string() },
        IteratorSpec::IteratorFunction { 
            name: "split_by_word".to_string(),
            params: Vec::new(),
            field_name: "BlogPost.content".to_string() 
        }
    ];
    
    let result = engine.execute_chain(&specs, &input, "BlogPostWordIndex.word");
    
    assert!(result.contains_key("BlogPostWordIndex.word"));
    let entries = &result["BlogPostWordIndex.word"];
    assert_eq!(entries.len(), 3); // "hello", "world", "test"
    
    // Verify all words are present
    let values: Vec<String> = entries.iter()
        .filter_map(|entry| entry.value_text.clone())
        .collect();
    
    assert!(values.contains(&"hello".to_string()));
    assert!(values.contains(&"world".to_string()));
    assert!(values.contains(&"test".to_string()));
}

#[test]
#[ignore]
fn test_engine_executes_reducer_functions() {
    let parser = ChainParser::new();
    let engine = TypedEngine::new();
    
    // Test count reducer
    let count_chain = parser
        .parse("BlogPost.content.split_by_word().count()")
        .expect("Should parse");
    let count_specs = map_chain_to_specs(&count_chain);
    let input = create_test_input();

    let result = engine.execute_chain(&count_specs, &input, "BlogPostSummary.count");

    assert!(result.contains_key("BlogPostSummary.count"));
    let entries = &result["BlogPostSummary.count"];
    assert_eq!(entries.len(), 1); // Single count result
    assert_eq!(entries[0].value_text, Some("3".to_string()));
}

#[test]
#[ignore]
fn test_engine_executes_all_reducer_types() {
    let parser = ChainParser::new();
    let engine = TypedEngine::new();
    
    let input = create_array_input();
    
    // Test each reducer type
    let test_cases = vec![
        ("Scores.values.count()", "3"),
        ("Scores.values.join()", "rust, database, transforms"),
        ("Scores.values.first()", "rust"),
        ("Scores.values.last()", "transforms"),
    ];

    for (expr, expected) in test_cases {
        let chain = parser.parse(expr).unwrap_or_else(|_| panic!("Should parse: {}", expr));
        let specs = map_chain_to_specs(&chain);

        let output_key = format!("{}_result", expr.replace('.', "_").replace("()", ""));
        let result = engine.execute_chain(&specs, &input, &output_key);

        assert!(
            result.contains_key(&output_key),
            "Result should contain key '{}' for: {}",
            output_key,
            expr
        );
        let entries = &result[&output_key];
        assert_eq!(entries.len(), 1, "Should have single result for: {}", expr);
        assert_eq!(entries[0].value_text, Some(expected.to_string()), "Wrong result for: {}", expr);
    }
}

#[test]
#[ignore]
fn test_engine_executes_numeric_reducers() {
    let parser = ChainParser::new();
    let engine = TypedEngine::new();
    
    let input = create_numeric_input();
    
    // Test numeric reducers
    let test_cases = vec![
        ("Scores.values.sum()", "60"),    // 10 + 20 + 30
        ("Scores.values.max()", "30"),    // max(10, 20, 30)
        ("Scores.values.min()", "10"),    // min(10, 20, 30)
        ("Scores.values.count()", "3"),   // count
    ];

    for (expr, expected) in test_cases {
        let chain = parser.parse(expr).unwrap_or_else(|_| panic!("Should parse: {}", expr));
        let specs = map_chain_to_specs(&chain);

        let output_key = format!("{}_result", expr.replace('.', "_").replace("()", ""));
        let result = engine.execute_chain(&specs, &input, &output_key);

        assert!(
            result.contains_key(&output_key),
            "Result should contain key '{}' for: {}",
            output_key,
            expr
        );
        let entries = &result[&output_key];
        assert_eq!(entries.len(), 1, "Should have single result for: {}", expr);
        assert_eq!(entries[0].value_text, Some(expected.to_string()), "Wrong result for: {}", expr);
    }
}

#[test]
#[ignore]
fn test_engine_handles_empty_collections() {
    let parser = ChainParser::new();
    let engine = TypedEngine::new();
    
    // Create empty input
    let mut input: TypedInput = HashMap::new();
    let empty_data = HashMap::new(); // Empty HashMap
    input.insert("Empty.empty".to_string(), empty_data);

    // Test reducers on empty collections
    let test_cases = vec![
        ("Empty.empty.count()", "0"),
        ("Empty.empty.join()", ""),
        ("Empty.empty.sum()", "0"),
    ];

    for (expr, expected) in test_cases {
        let chain = parser.parse(expr).unwrap_or_else(|_| panic!("Should parse: {}", expr));
        let specs = map_chain_to_specs(&chain);

        let output_key = format!("{}_result", expr.replace('.', "_").replace("()", ""));
        let result = engine.execute_chain(&specs, &input, &output_key);

        assert!(
            result.contains_key(&output_key),
            "Result should contain key '{}' for: {}",
            output_key,
            expr
        );
        let entries = &result[&output_key];
        assert_eq!(entries.len(), 1, "Should have single result for: {}", expr);
        assert_eq!(entries[0].value_text, Some(expected.to_string()), "Wrong result for: {}", expr);
    }
}

#[test]
#[ignore]
fn test_engine_complex_chains() {
    let parser = ChainParser::new();
    let engine = TypedEngine::new();
    
    let input = create_test_input();
    
    // Test complex chain: split words, then join them back
    let chain = parser
        .parse("BlogPost.content.split_by_word().join()")
        .expect("Should parse");
    let specs = map_chain_to_specs(&chain);

    let result = engine.execute_chain(&specs, &input, "BlogPostSummary.join");

    assert!(result.contains_key("BlogPostSummary.join"));
    let entries = &result["BlogPostSummary.join"];
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].value_text, Some("hello, world, test".to_string()));
}

#[test]
#[ignore]
fn test_engine_preserves_atom_traceability() {
    let parser = ChainParser::new();
    let engine = TypedEngine::new();
    
    let input = create_test_input();
    
    // Test that reducer preserves atom_uuid from source
    let chain = parser
        .parse("BlogPost.content.split_by_word().count()")
        .expect("Should parse");
    let specs = map_chain_to_specs(&chain);

    let result = engine.execute_chain(&specs, &input, "BlogPostSummary.count");

    assert!(result.contains_key("BlogPostSummary.count"));
    let entries = &result["BlogPostSummary.count"];
    assert_eq!(entries.len(), 1);
    
    // Should preserve atom_uuid from the first item (HashMap iteration order dependent)
    // Just verify it's one of the valid atom_uuids
    let valid_uuids = ["atom-1", "atom-2", "atom-3"];
    assert!(valid_uuids.contains(&entries[0].atom_uuid.as_str()));
}

#[test]
#[ignore]
fn test_engine_mixed_data_types() {
    let parser = ChainParser::new();
    let engine = TypedEngine::new();
    
    // Create input with mixed numeric and text data
    let mut input: TypedInput = HashMap::new();
    let mut mixed_data = HashMap::new();
    mixed_data.insert(
        KeyValue::new(Some("values".to_string()), Some("item1".to_string())),
        FieldValue {
            value: serde_json::Value::Number(serde_json::Number::from(10)),
            atom_uuid: "atom-1".to_string(),
        }
    );
    mixed_data.insert(
        KeyValue::new(Some("values".to_string()), Some("item2".to_string())),
        FieldValue {
            value: serde_json::Value::String("not_a_number".to_string()),
            atom_uuid: "atom-2".to_string(),
        }
    );
    mixed_data.insert(
        KeyValue::new(Some("values".to_string()), Some("item3".to_string())),
        FieldValue {
            value: serde_json::Value::Number(serde_json::Number::from(20)),
            atom_uuid: "atom-3".to_string(),
        }
    );
    input.insert("Mixed.values".to_string(), mixed_data);
    
    // Test numeric reducers with mixed data
    let test_cases = vec![
        ("Mixed.values.sum()", "30"),    // Only numeric values: 10 + 20
        ("Mixed.values.max()", "20"),    // Max of numeric values
        ("Mixed.values.min()", "10"),    // Min of numeric values
        ("Mixed.values.count()", "3"),   // Count all items
        ("Mixed.values.join()", "10, not_a_number, 20"), // Join all as strings
    ];

    for (expr, expected) in test_cases {
        let chain = parser.parse(expr).unwrap_or_else(|_| panic!("Should parse: {}", expr));
        let specs = map_chain_to_specs(&chain);

        let output_key = format!("{}_result", expr.replace('.', "_").replace("()", ""));
        let result = engine.execute_chain(&specs, &input, &output_key);

        assert!(
            result.contains_key(&output_key),
            "Result should contain key '{}' for: {}",
            output_key,
            expr
        );
        let entries = &result[&output_key];
        assert_eq!(entries.len(), 1, "Should have single result for: {}", expr);
        assert_eq!(entries[0].value_text, Some(expected.to_string()), "Wrong result for: {}", expr);
    }
}
