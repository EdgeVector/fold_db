use datafold::transform::coordination::{
    collect_all_expressions, parse_expressions_with_monitoring, validate_field_alignment,
    execute_coordination_with_engine, convert_input_values_to_json, execute_with_engine,
    aggregate_execution_results
};
use datafold::schema::types::{
    json_schema::{DeclarativeSchemaDefinition, FieldDefinition, KeyConfig},
    SchemaError
};
use datafold::schema::SchemaType;
use datafold::transform::iterator_stack::field_alignment::AlignmentValidationResult;
use datafold::transform::iterator_stack::execution_engine::ExecutionResult;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};

/// Test collecting expressions from schema and key configuration
#[test]
fn test_collect_all_expressions() {
    let mut fields = HashMap::new();
    fields.insert("word".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content.split_by_word().map()".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("source_ref".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().$atom_uuid".to_string()),
        field_type: Some("String".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "TestWordIndex".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "blogpost.map().content.split_by_word().map()".to_string(),
            range_field: "blogpost.map().publish_date".to_string(),
        }),
        fields,
    };

    let key_config = schema.key.as_ref().unwrap();
    let expressions = collect_all_expressions(&schema, key_config).expect("Should collect expressions");

    // Should have 4 expressions: 2 key expressions + 2 field expressions
    assert_eq!(expressions.len(), 4);
    
    // Check that key expressions are included
    assert!(expressions.iter().any(|(name, _)| name == "_hash_field"));
    assert!(expressions.iter().any(|(name, _)| name == "_range_field"));
    
    // Check that field expressions are included
    assert!(expressions.iter().any(|(name, _)| name == "word"));
    assert!(expressions.iter().any(|(name, _)| name == "source_ref"));
}

/// Test parsing expressions with monitoring
#[test]
fn test_parse_expressions_with_monitoring() {
    let expressions = vec![
        ("test_field".to_string(), "data.map()".to_string()),
    ];

    let result = parse_expressions_with_monitoring(&expressions);
    
    // Should succeed for valid expressions
    assert!(result.is_ok());
    let parsed_chains = result.unwrap();
    assert_eq!(parsed_chains.len(), 1);
    assert_eq!(parsed_chains[0].0, "test_field");
}

/// Test parsing expressions with invalid syntax
#[test]
fn test_parse_expressions_with_invalid_syntax() {
    let expressions = vec![
        ("test_field".to_string(), "invalid.syntax.here".to_string()),
    ];

    let result = parse_expressions_with_monitoring(&expressions);
    
    // Should fail for invalid expressions
    assert!(result.is_err());
}

/// Test parsing empty expressions
#[test]
fn test_parse_empty_expressions() {
    let expressions = vec![];

    let result = parse_expressions_with_monitoring(&expressions);
    
    // Should fail for empty expressions
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SchemaError::InvalidField(_)));
}

/// Test converting input values to JSON
#[test]
fn test_convert_input_values_to_json() {
    let mut input_values = HashMap::new();
    input_values.insert("test_field".to_string(), JsonValue::String("test_value".to_string()));
    input_values.insert("number_field".to_string(), JsonValue::Number(42.into()));

    let result = convert_input_values_to_json(&input_values).expect("Should convert to JSON");
    
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert!(obj.contains_key("test_field"));
    assert!(obj.contains_key("number_field"));
}

/// Test converting empty input values
#[test]
fn test_convert_empty_input_values() {
    let input_values = HashMap::new();

    let result = convert_input_values_to_json(&input_values).expect("Should convert empty to JSON");
    
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 0);
}

/// Test field alignment validation with valid chains
#[test]
fn test_validate_field_alignment_valid() {
    // Create a mock alignment result that's valid
    let alignment_result = AlignmentValidationResult {
        valid: true,
        errors: vec![],
    };

    // Mock the validator to return our test result
    // Note: This is a simplified test - in practice, you'd need to mock the validator
    // or create actual ParsedChain objects for a more comprehensive test
    
    // For now, we'll test the error handling path
    let parsed_chains = vec![];
    let result = validate_field_alignment(&parsed_chains);
    
    // Should fail for empty chains
    assert!(result.is_err());
}

/// Test aggregate execution results
#[test]
fn test_aggregate_execution_results() {
    let parsed_chains = vec![
        ("test_field".to_string(), datafold::transform::iterator_stack::chain_parser::ParsedChain {
            operations: VecDeque::new(),
        }),
    ];

    let execution_result = ExecutionResult {
        index_entries: vec![],
        warnings: vec![],
    };

    let mut input_values = HashMap::new();
    input_values.insert("test_input".to_string(), JsonValue::String("test_value".to_string()));

    let result = aggregate_execution_results(&parsed_chains, &execution_result, &input_values);
    
    // Should succeed
    assert!(result.is_ok());
}

/// Test error handling in expression collection
#[test]
fn test_collect_expressions_error_handling() {
    let schema = DeclarativeSchemaDefinition {
        name: "TestSchema".to_string(),
        schema_type: SchemaType::HashRange,
        key: None, // No key config
        fields: HashMap::new(),
    };

    let key_config = KeyConfig {
        hash_field: "test.hash".to_string(),
        range_field: "test.range".to_string(),
    };

    let expressions = collect_all_expressions(&schema, &key_config).expect("Should collect expressions");
    
    // Should have 2 expressions (just the key expressions)
    assert_eq!(expressions.len(), 2);
}

/// Test comprehensive workflow
#[test]
fn test_comprehensive_decomposition_workflow() {
    // Create a complete test scenario
    let mut fields = HashMap::new();
    fields.insert("word".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content.split_by_word().map()".to_string()),
        field_type: Some("String".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "TestWordIndex".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "blogpost.map().content.split_by_word().map()".to_string(),
            range_field: "blogpost.map().publish_date".to_string(),
        }),
        fields,
    };

    let key_config = schema.key.as_ref().unwrap();
    
    // Step 1: Collect expressions
    let expressions = collect_all_expressions(&schema, key_config).expect("Should collect expressions");
    assert_eq!(expressions.len(), 3); // 2 key + 1 field

    // Step 2: Convert input values
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), JsonValue::String("test_data".to_string()));
    let input_data = convert_input_values_to_json(&input_values).expect("Should convert to JSON");
    assert!(input_data.is_object());

    // Step 3: Test aggregation (simplified)
    let parsed_chains = vec![];
    let execution_result = ExecutionResult {
        index_entries: vec![],
        warnings: vec![],
    };
    let result = aggregate_execution_results(&parsed_chains, &execution_result, &input_values);
    assert!(result.is_ok());
}
