//! Test for complex chain expression execution
//! 
//! This test verifies that the execution engine can correctly handle complex chain expressions
//! like `blogpost.map().content.split_by_word().map()` and return actual evaluated results
//! instead of placeholder values.

use datafold::schema::types::json_schema::{DeclarativeSchemaDefinition, FieldDefinition};
use datafold::schema::types::schema::SchemaType;
use datafold::transform::Transform;
use datafold::transform::executor::TransformExecutor;
use std::collections::HashMap;

#[test]
fn test_complex_chain_expression_execution() {
    // Create schema with complex chain expression
    let mut fields = HashMap::new();
    fields.insert("word_content".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content.split_by_word().map()".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("range_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "word_data".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(datafold::schema::types::json_schema::KeyConfig {
            hash_field: "blogpost.map().content.split_by_word().map()".to_string(),
            range_field: "blogpost.map().content".to_string(),
        }),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.word_data".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "content": "Complex chain parsed content"
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            
            // HashRange results should have hash_key and range_key arrays
            assert!(obj.contains_key("hash_key"), "HashRange result should contain hash_key array");
            assert!(obj.contains_key("range_key"), "HashRange result should contain range_key array");
            
            // The word_content should be in the hash_key array
            let hash_key = obj.get("hash_key").unwrap();
            assert!(hash_key.is_array(), "hash_key should be an array");
            
            let word_array = hash_key.as_array().unwrap();
            assert_eq!(word_array.len(), 4, "Should have 4 words");
            
            // Check that we get actual words, not placeholder values
            let first_word = word_array[0].as_str().unwrap();
            assert_eq!(first_word, "Complex", "First word should be 'Complex'");
            
            let second_word = word_array[1].as_str().unwrap();
            assert_eq!(second_word, "chain", "Second word should be 'chain'");
            
            let third_word = word_array[2].as_str().unwrap();
            assert_eq!(third_word, "parsed", "Third word should be 'parsed'");
            
            let fourth_word = word_array[3].as_str().unwrap();
            assert_eq!(fourth_word, "content", "Fourth word should be 'content'");
        }
        Err(err) => {
            panic!("Complex chain expression failed: {:?}", err);
        }
    }
}

#[test]
fn test_simple_chain_expression_execution() {
    // Create schema with simple chain expression
    let mut fields = HashMap::new();
    fields.insert("content".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "content_data".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.content_data".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "content": "Simple content value"
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    assert!(result.is_ok(), "Simple chain expression should succeed");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    
    // The result should be a single string value
    let content = obj.get("content").unwrap();
    assert!(content.is_string(), "Result should be a string");
    
    let content_str = content.as_str().unwrap();
    assert_eq!(content_str, "Simple content value", "Content should match input");
}
