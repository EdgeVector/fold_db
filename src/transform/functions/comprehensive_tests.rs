//! Comprehensive tests for all transform functions
//!
//! This module contains extensive tests covering:
//! - All iterator functions (mappers)
//! - All reducer functions
//! - Function combinations and chaining
//! - Edge cases and error conditions
//! - Performance characteristics

use super::*;
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::transform::iterator_stack_typed::types::IterationItem;

/// Helper to create test iteration items
fn create_text_item(text: &str, key: &str) -> IterationItem {
    IterationItem {
        key: KeyValue::new(Some(key.to_string()), None),
        value: FieldValue {
            value: serde_json::Value::String(text.to_string()),
            atom_uuid: format!("atom-{}", key),
            source_file_name: None,
        },
        is_text_token: false,
    }
}

fn create_numeric_item(value: f64, key: &str) -> IterationItem {
    IterationItem {
        key: KeyValue::new(Some(key.to_string()), None),
        value: FieldValue {
            value: serde_json::Value::Number(serde_json::Number::from_f64(value).unwrap()),
            atom_uuid: format!("atom-{}", key),
            source_file_name: None,
        },
        is_text_token: false,
    }
}

fn create_array_item(values: Vec<serde_json::Value>, key: &str) -> IterationItem {
    IterationItem {
        key: KeyValue::new(Some(key.to_string()), None),
        value: FieldValue {
            value: serde_json::Value::Array(values),
            atom_uuid: format!("atom-{}", key),
            source_file_name: None,
        },
        is_text_token: false,
    }
}

// ============================================================================
// Iterator Function Tests
// ============================================================================

#[test]
fn test_split_by_word_basic() {
    let reg = registry();
    let func = reg
        .get_iterator("split_by_word")
        .expect("split_by_word should exist");

    let item = create_text_item("hello world test", "content");
    let result = func.execute(&item);

    match result {
        IteratorExecutionResult::TextTokens(tokens) => {
            assert_eq!(tokens, vec!["hello", "world", "test"]);
        }
        _ => panic!("Expected TextTokens result"),
    }
}

#[test]
fn test_split_by_word_empty_string() {
    let reg = registry();
    let func = reg
        .get_iterator("split_by_word")
        .expect("split_by_word should exist");

    let item = create_text_item("", "content");
    let result = func.execute(&item);

    match result {
        IteratorExecutionResult::TextTokens(tokens) => {
            assert_eq!(tokens, Vec::<String>::new());
        }
        _ => panic!("Expected TextTokens result"),
    }
}

#[test]
fn test_split_by_word_single_word() {
    let reg = registry();
    let func = reg
        .get_iterator("split_by_word")
        .expect("split_by_word should exist");

    let item = create_text_item("hello", "content");
    let result = func.execute(&item);

    match result {
        IteratorExecutionResult::TextTokens(tokens) => {
            assert_eq!(tokens, vec!["hello"]);
        }
        _ => panic!("Expected TextTokens result"),
    }
}

#[test]
fn test_split_by_word_multiple_spaces() {
    let reg = registry();
    let func = reg
        .get_iterator("split_by_word")
        .expect("split_by_word should exist");

    let item = create_text_item("hello    world    test", "content");
    let result = func.execute(&item);

    match result {
        IteratorExecutionResult::TextTokens(tokens) => {
            assert_eq!(tokens, vec!["hello", "world", "test"]);
        }
        _ => panic!("Expected TextTokens result"),
    }
}

#[test]
fn test_split_by_word_newlines_and_tabs() {
    let reg = registry();
    let func = reg
        .get_iterator("split_by_word")
        .expect("split_by_word should exist");

    let item = create_text_item("hello\nworld\ttest", "content");
    let result = func.execute(&item);

    match result {
        IteratorExecutionResult::TextTokens(tokens) => {
            assert_eq!(tokens, vec!["hello", "world", "test"]);
        }
        _ => panic!("Expected TextTokens result"),
    }
}

#[test]
fn test_split_by_word_metadata() {
    let reg = registry();
    let func = reg
        .get_iterator("split_by_word")
        .expect("split_by_word should exist");

    let metadata = func.metadata();
    assert_eq!(metadata.name, "split_by_word");
    assert_eq!(metadata.function_type, FunctionType::Iterator);
    assert!(metadata.description.contains("Split text"));
}

#[test]
fn test_split_array_basic() {
    let reg = registry();
    let func = reg
        .get_iterator("split_array")
        .expect("split_array should exist");

    // Note: split_array is currently not fully implemented
    // This test documents current behavior
    let item = create_array_item(
        vec![
            serde_json::Value::String("item1".to_string()),
            serde_json::Value::String("item2".to_string()),
            serde_json::Value::String("item3".to_string()),
        ],
        "array",
    );

    let result = func.execute(&item);

    match result {
        IteratorExecutionResult::Items(items) => {
            // Returns split items
            assert_eq!(items.len(), 3);
            assert_eq!(
                items[0].value.value,
                serde_json::Value::String("item1".to_string())
            );
            assert_eq!(
                items[1].value.value,
                serde_json::Value::String("item2".to_string())
            );
            assert_eq!(
                items[2].value.value,
                serde_json::Value::String("item3".to_string())
            );
        }
        _ => panic!("Expected Items result"),
    }
}

#[test]
fn test_split_array_metadata() {
    let reg = registry();
    let func = reg
        .get_iterator("split_array")
        .expect("split_array should exist");

    let metadata = func.metadata();
    assert_eq!(metadata.name, "split_array");
    assert_eq!(metadata.function_type, FunctionType::Iterator);
    assert!(metadata.description.contains("Split an array"));
}

// ============================================================================
// Reducer Function Tests
// ============================================================================

#[test]
fn test_count_reducer_basic() {
    let reg = registry();
    let reducer = reg
        .get_reducer("count")
        .expect("count reducer should exist");

    let items = vec![
        create_text_item("one", "item1"),
        create_text_item("two", "item2"),
        create_text_item("three", "item3"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, "3");
}

#[test]
fn test_count_reducer_empty() {
    let reg = registry();
    let reducer = reg
        .get_reducer("count")
        .expect("count reducer should exist");

    let items = vec![];

    let result = reducer.execute(&items);
    assert_eq!(result, "0");
}

#[test]
fn test_count_reducer_single() {
    let reg = registry();
    let reducer = reg
        .get_reducer("count")
        .expect("count reducer should exist");

    let items = vec![create_text_item("single", "item1")];

    let result = reducer.execute(&items);
    assert_eq!(result, "1");
}

#[test]
fn test_sum_reducer_basic() {
    let reg = registry();
    let reducer = reg.get_reducer("sum").expect("sum reducer should exist");

    let items = vec![
        create_numeric_item(10.0, "item1"),
        create_numeric_item(20.0, "item2"),
        create_numeric_item(30.0, "item3"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, "60");
}

#[test]
fn test_sum_reducer_mixed_types() {
    let reg = registry();
    let reducer = reg.get_reducer("sum").expect("sum reducer should exist");

    let items = vec![
        create_numeric_item(10.0, "item1"),
        create_text_item("not_a_number", "item2"), // Should be ignored
        create_numeric_item(30.0, "item3"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, "40"); // Only numeric values summed
}

#[test]
fn test_sum_reducer_empty() {
    let reg = registry();
    let reducer = reg.get_reducer("sum").expect("sum reducer should exist");

    let items = vec![];

    let result = reducer.execute(&items);
    assert_eq!(result, "0");
}

#[test]
fn test_sum_reducer_no_numeric() {
    let reg = registry();
    let reducer = reg.get_reducer("sum").expect("sum reducer should exist");

    let items = vec![
        create_text_item("not_a_number", "item1"),
        create_text_item("also_not_a_number", "item2"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, "0"); // No numeric values to sum
}

#[test]
fn test_join_reducer_basic() {
    let reg = registry();
    let reducer = reg.get_reducer("join").expect("join reducer should exist");

    let items = vec![
        create_text_item("hello", "item1"),
        create_text_item("world", "item2"),
        create_text_item("test", "item3"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, "hello, world, test");
}

#[test]
fn test_join_reducer_empty() {
    let reg = registry();
    let reducer = reg.get_reducer("join").expect("join reducer should exist");

    let items = vec![];

    let result = reducer.execute(&items);
    assert_eq!(result, "");
}

#[test]
fn test_join_reducer_single() {
    let reg = registry();
    let reducer = reg.get_reducer("join").expect("join reducer should exist");

    let items = vec![create_text_item("single", "item1")];

    let result = reducer.execute(&items);
    assert_eq!(result, "single");
}

#[test]
fn test_first_reducer_basic() {
    let reg = registry();
    let reducer = reg
        .get_reducer("first")
        .expect("first reducer should exist");

    let items = vec![
        create_text_item("first", "item1"),
        create_text_item("second", "item2"),
        create_text_item("third", "item3"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, "first");
}

#[test]
fn test_first_reducer_empty() {
    let reg = registry();
    let reducer = reg
        .get_reducer("first")
        .expect("first reducer should exist");

    let items = vec![];

    let result = reducer.execute(&items);
    assert_eq!(result, "");
}

#[test]
fn test_last_reducer_basic() {
    let reg = registry();
    let reducer = reg.get_reducer("last").expect("last reducer should exist");

    let items = vec![
        create_text_item("first", "item1"),
        create_text_item("second", "item2"),
        create_text_item("third", "item3"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, "third");
}

#[test]
fn test_last_reducer_empty() {
    let reg = registry();
    let reducer = reg.get_reducer("last").expect("last reducer should exist");

    let items = vec![];

    let result = reducer.execute(&items);
    assert_eq!(result, "");
}

#[test]
fn test_max_reducer_basic() {
    let reg = registry();
    let reducer = reg.get_reducer("max").expect("max reducer should exist");

    let items = vec![
        create_numeric_item(10.0, "item1"),
        create_numeric_item(25.0, "item2"),
        create_numeric_item(15.0, "item3"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, "25");
}

#[test]
fn test_max_reducer_mixed_types() {
    let reg = registry();
    let reducer = reg.get_reducer("max").expect("max reducer should exist");

    let items = vec![
        create_numeric_item(10.0, "item1"),
        create_text_item("not_a_number", "item2"), // Should be ignored
        create_numeric_item(25.0, "item3"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, "25"); // Only numeric values considered
}

#[test]
fn test_max_reducer_empty() {
    let reg = registry();
    let reducer = reg.get_reducer("max").expect("max reducer should exist");

    let items = vec![];

    let result = reducer.execute(&items);
    assert_eq!(result, "");
}

#[test]
fn test_max_reducer_no_numeric() {
    let reg = registry();
    let reducer = reg.get_reducer("max").expect("max reducer should exist");

    let items = vec![
        create_text_item("not_a_number", "item1"),
        create_text_item("also_not_a_number", "item2"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, ""); // No numeric values
}

#[test]
fn test_min_reducer_basic() {
    let reg = registry();
    let reducer = reg.get_reducer("min").expect("min reducer should exist");

    let items = vec![
        create_numeric_item(25.0, "item1"),
        create_numeric_item(10.0, "item2"),
        create_numeric_item(15.0, "item3"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, "10");
}

#[test]
fn test_min_reducer_mixed_types() {
    let reg = registry();
    let reducer = reg.get_reducer("min").expect("min reducer should exist");

    let items = vec![
        create_numeric_item(25.0, "item1"),
        create_text_item("not_a_number", "item2"), // Should be ignored
        create_numeric_item(10.0, "item3"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, "10"); // Only numeric values considered
}

#[test]
fn test_min_reducer_empty() {
    let reg = registry();
    let reducer = reg.get_reducer("min").expect("min reducer should exist");

    let items = vec![];

    let result = reducer.execute(&items);
    assert_eq!(result, "");
}

#[test]
fn test_min_reducer_no_numeric() {
    let reg = registry();
    let reducer = reg.get_reducer("min").expect("min reducer should exist");

    let items = vec![
        create_text_item("not_a_number", "item1"),
        create_text_item("also_not_a_number", "item2"),
    ];

    let result = reducer.execute(&items);
    assert_eq!(result, ""); // No numeric values
}

// ============================================================================
// Edge Cases and Error Conditions
// ============================================================================

#[test]
fn test_functions_with_null_values() {
    let reg = registry();

    // Test with null JSON values
    let null_item = IterationItem {
        key: KeyValue::new(Some("null".to_string()), None),
        value: FieldValue {
            value: serde_json::Value::Null,
            atom_uuid: "atom-null".to_string(),
            source_file_name: None,
        },
        is_text_token: false,
    };

    // split_by_word should handle null gracefully
    let iterator = reg
        .get_iterator("split_by_word")
        .expect("split_by_word should exist");
    let result = iterator.execute(&null_item);
    match result {
        IteratorExecutionResult::TextTokens(tokens) => {
            assert_eq!(tokens, Vec::<String>::new()); // Empty result for null
        }
        _ => panic!("Expected TextTokens result"),
    }

    // count should handle null items
    let reducer = reg
        .get_reducer("count")
        .expect("count reducer should exist");
    let items = vec![null_item.clone()];
    let result = reducer.execute(&items);
    assert_eq!(result, "1"); // Counts the null item
}

#[test]
fn test_functions_with_complex_json() {
    let reg = registry();

    // Test with complex JSON objects
    let mut obj = serde_json::Map::new();
    obj.insert(
        "value".to_string(),
        serde_json::Value::String("test".to_string()),
    );

    let complex_item = IterationItem {
        key: KeyValue::new(Some("complex".to_string()), None),
        value: FieldValue {
            value: serde_json::Value::Object(obj),
            atom_uuid: "atom-complex".to_string(),
            source_file_name: None,
        },
        is_text_token: false,
    };

    // split_by_word should extract text from complex objects
    let iterator = reg
        .get_iterator("split_by_word")
        .expect("split_by_word should exist");
    let result = iterator.execute(&complex_item);
    match result {
        IteratorExecutionResult::TextTokens(tokens) => {
            assert_eq!(tokens, vec!["test"]); // Extracts "test" from object
        }
        _ => panic!("Expected TextTokens result"),
    }
}

#[test]
fn test_large_collections_performance() {
    let reg = registry();

    // Test with large collections (1000 items)
    let mut items = Vec::new();
    for i in 0..1000 {
        items.push(create_numeric_item(i as f64, &format!("item{}", i)));
    }

    // count should be fast even with large collections
    let count_reducer = reg
        .get_reducer("count")
        .expect("count reducer should exist");
    let start = std::time::Instant::now();
    let result = count_reducer.execute(&items);
    let duration = start.elapsed();

    assert_eq!(result, "1000");
    assert!(
        duration.as_millis() < 100,
        "count should be fast even with large collections"
    );

    // sum should also be reasonably fast
    let sum_reducer = reg.get_reducer("sum").expect("sum reducer should exist");
    let start = std::time::Instant::now();
    let result = sum_reducer.execute(&items);
    let duration = start.elapsed();

    assert_eq!(result, "499500"); // Sum of 0+1+2+...+999
    assert!(duration.as_millis() < 100, "sum should be reasonably fast");
}

#[test]
fn test_unicode_and_special_characters() {
    let reg = registry();
    let func = reg
        .get_iterator("split_by_word")
        .expect("split_by_word should exist");

    // Test with Unicode characters
    let unicode_item = create_text_item("hello 世界 🌍 test", "unicode");
    let result = func.execute(&unicode_item);

    match result {
        IteratorExecutionResult::TextTokens(tokens) => {
            assert_eq!(tokens, vec!["hello", "世界", "🌍", "test"]);
        }
        _ => panic!("Expected TextTokens result"),
    }

    // Test with special characters
    let special_item = create_text_item("hello@#$%^&*()world", "special");
    let result = func.execute(&special_item);

    match result {
        IteratorExecutionResult::TextTokens(tokens) => {
            assert_eq!(tokens, vec!["hello@#$%^&*()world"]); // Treated as single word
        }
        _ => panic!("Expected TextTokens result"),
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_iterator_then_reducer_chain() {
    // This test simulates the full chain execution
    let reg = registry();

    // Simulate: content.split_by_word().count()
    let content_item = create_text_item("hello world test", "content");

    // Step 1: Apply iterator
    let iterator = reg
        .get_iterator("split_by_word")
        .expect("split_by_word should exist");
    let iterator_result = iterator.execute(&content_item);

    // Convert iterator result to items (simulating engine behavior)
    let mut items = Vec::new();
    match iterator_result {
        IteratorExecutionResult::TextTokens(tokens) => {
            for (i, token) in tokens.iter().enumerate() {
                items.push(create_text_item(token, &format!("word{}", i)));
            }
        }
        IteratorExecutionResult::Items(items_vec) => {
            items.extend(items_vec);
        }
    }

    // Step 2: Apply reducer
    let reducer = reg
        .get_reducer("count")
        .expect("count reducer should exist");
    let final_result = reducer.execute(&items);

    assert_eq!(final_result, "3"); // Should count 3 words
}

#[test]
fn test_multiple_reducers_on_same_data() {
    let reg = registry();

    let items = vec![
        create_numeric_item(10.0, "item1"),
        create_numeric_item(25.0, "item2"),
        create_numeric_item(15.0, "item3"),
    ];

    // Test multiple reducers on same data
    let count_reducer = reg
        .get_reducer("count")
        .expect("count reducer should exist");
    let sum_reducer = reg.get_reducer("sum").expect("sum reducer should exist");
    let max_reducer = reg.get_reducer("max").expect("max reducer should exist");
    let min_reducer = reg.get_reducer("min").expect("min reducer should exist");

    assert_eq!(count_reducer.execute(&items), "3");
    assert_eq!(sum_reducer.execute(&items), "50");
    assert_eq!(max_reducer.execute(&items), "25");
    assert_eq!(min_reducer.execute(&items), "10");
}

#[test]
fn test_all_function_metadata() {
    let reg = registry();

    // Test all iterator functions have proper metadata
    for name in reg.iterator_names() {
        let func = reg.get_iterator(&name).expect("iterator should exist");
        let metadata = func.metadata();

        assert_eq!(metadata.name, name);
        assert_eq!(metadata.function_type, FunctionType::Iterator);
        assert!(!metadata.description.is_empty());
    }

    // Test all reducer functions have proper metadata
    for name in reg.reducer_names() {
        let func = reg.get_reducer(&name).expect("reducer should exist");
        let metadata = func.metadata();

        assert_eq!(metadata.name, name);
        assert_eq!(metadata.function_type, FunctionType::Reducer);
        assert!(!metadata.description.is_empty());
    }
}

#[test]
fn test_function_registry_completeness() {
    let reg = registry();

    // Verify all expected functions are registered
    let expected_iterators = vec!["split_by_word", "split_array"];
    let expected_reducers = vec!["count", "sum", "join", "first", "last", "max", "min"];

    for expected in expected_iterators {
        assert!(
            reg.is_iterator(expected),
            "Iterator {} should be registered",
            expected
        );
        assert_eq!(
            reg.get_function_type(expected),
            Some(FunctionType::Iterator)
        );
    }

    for expected in expected_reducers {
        assert!(
            reg.is_reducer(expected),
            "Reducer {} should be registered",
            expected
        );
        assert_eq!(reg.get_function_type(expected), Some(FunctionType::Reducer));
    }

    // Verify no unexpected functions
    assert!(!reg.is_registered("unknown_function"));
    assert!(!reg.is_registered("fake_function"));
    assert_eq!(reg.get_function_type("unknown"), None);
}
