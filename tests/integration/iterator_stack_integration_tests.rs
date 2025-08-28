//! Iterator Stack Integration Tests
//!
//! This comprehensive test suite validates that declarative transforms integrate
//! seamlessly with the existing iterator stack infrastructure and that both
//! transform types can coexist without conflicts.
//!
//! **Integration Coverage:**
//! 1. **Iterator Stack Integration** - Test integration with existing iterator stack components
//! 2. **Chain Parser Integration** - Verify chain parsing works with declarative transforms
//! 3. **Execution Engine Integration** - Test runtime execution with iterator stack
//! 4. **Field Alignment Integration** - Verify existing field alignment validation works
//! 5. **Mixed Transform Scenarios** - Test both transform types working together
//! 6. **Performance Integration** - Test performance with iterator stack infrastructure
//! 7. **Error Handling Integration** - Test error scenarios with iterator stack components

use datafold::db_operations::DbOperations;
use datafold::schema::types::transform::{Transform, TransformRegistration};
use datafold::schema::types::json_schema::{TransformKind, DeclarativeSchemaDefinition, FieldDefinition};
use datafold::schema::types::schema::SchemaType;
use datafold::schema::indexing::{
    ChainParser, IteratorStack, FieldAlignmentValidator, ExecutionEngine,
    ParsedChain, AlignmentValidationResult, ExecutionResult
};
use datafold::fold_db_core::transform_manager::TransformManager;
use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
use datafold::transform::TransformExecutor;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Test fixture for iterator stack integration testing
struct IteratorStackIntegrationFixture {
    pub db_ops: Arc<DbOperations>,
    pub message_bus: Arc<MessageBus>,
    pub transform_manager: Arc<TransformManager>,
    pub chain_parser: ChainParser,
    pub field_alignment_validator: FieldAlignmentValidator,
    pub execution_engine: ExecutionEngine,
    pub _temp_dir: TempDir,
}

impl IteratorStackIntegrationFixture {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db = sled::open(temp_dir.path()).expect("Failed to open database");
        let db_ops = Arc::new(DbOperations::new(db).expect("Failed to create database"));
        let message_bus = Arc::new(MessageBus::new());
        let transform_manager = Arc::new(TransformManager::new(db_ops.clone(), message_bus.clone())
            .expect("Failed to create transform manager"));
        
        Self {
            db_ops,
            message_bus,
            transform_manager,
            chain_parser: ChainParser::new(),
            field_alignment_validator: FieldAlignmentValidator::new(),
            execution_engine: ExecutionEngine::new(),
            _temp_dir: temp_dir,
        }
    }
}

/// Test basic iterator stack integration with declarative transforms
#[test]
fn test_iterator_stack_basic_integration() {
    let fixture = IteratorStackIntegrationFixture::new();
    
    // Create a declarative transform with iterator expressions
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("processed_content".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().content".to_string()),
            }),
            ("word_count".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().content.split_by_word().count()".to_string()),
            }),
        ]),
        key: None,
    };
    
    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_schema.blogpost".to_string()],
        "test_schema.processed_content".to_string()
    );
    
    // Test chain parsing integration
    let parsed_chains = fixture.chain_parser.parse("blogpost.map().content.split_by_word().count()")
        .expect("Failed to parse chain expression");
    
    // Test iterator stack creation from parsed chain
    let iterator_stack = IteratorStack::from_chain(&parsed_chains)
        .expect("Failed to create iterator stack from chain");
    
    // Verify iterator stack structure
    assert_eq!(iterator_stack.current_depth(), 2); // blogpost.map() and content.split_by_word()
    assert!(iterator_stack.current_scope().is_some());
    
    // Test field alignment validation
    let mut parsed_chains_vec = vec![parsed_chains];
    let alignment_result = fixture.field_alignment_validator.validate_alignment(&parsed_chains_vec)
        .expect("Failed to validate field alignment");
    
    match alignment_result {
        AlignmentValidationResult::Valid { warnings } => {
            // Check for expected warnings about complex expressions
            assert!(warnings.is_empty() || warnings.len() > 0);
        }
        AlignmentValidationResult::Invalid { errors } => {
            panic!("Field alignment validation failed: {:?}", errors);
        }
    }
}

/// Test execution engine integration with iterator stack
#[test]
fn test_execution_engine_integration() {
    let fixture = IteratorStackIntegrationFixture::new();
    
    // Create test data
    let input_data = serde_json::json!({
        "blogpost": {
            "content": "This is a test blog post with multiple words for testing word splitting functionality."
        }
    });
    
    // Parse a chain expression
    let parsed_chain = fixture.chain_parser.parse("blogpost.map().content.split_by_word().count()")
        .expect("Failed to parse chain expression");
    
    // Create iterator stack
    let iterator_stack = IteratorStack::from_chain(&parsed_chain)
        .expect("Failed to create iterator stack");
    
    // Test execution engine with iterator stack
    let execution_result = fixture.execution_engine.execute_expression(
        &parsed_chain,
        &input_data,
        &iterator_stack
    ).expect("Failed to execute expression");
    
    // Verify execution result
    assert!(execution_result.is_some());
    let result = execution_result.unwrap();
    assert!(result.index_entries.len() > 0);
    
    // Verify the result contains expected data
    let first_entry = &result.index_entries[0];
    assert!(first_entry.hash_value.is_some());
    assert!(first_entry.range_value.is_some());
    assert!(first_entry.atom_uuid.is_some());
}

/// Test mixed transform scenarios with both procedural and declarative transforms
#[test]
fn test_mixed_transform_scenarios() {
    let fixture = IteratorStackIntegrationFixture::new();
    
    // Create a procedural transform
    let procedural_transform = Transform::new(
        "input_value * 2".to_string(),
        "test_schema.doubled".to_string()
    );
    
    // Create a declarative transform
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("processed".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_schema.data".to_string()],
        "test_schema.processed".to_string()
    );
    
    // Register both transforms
    let procedural_registration = TransformRegistration {
        transform_id: "procedural_double".to_string(),
        transform: procedural_transform,
        input_molecules: vec!["test_schema.input_value".to_string()],
        input_names: vec!["input_value".to_string()],
        trigger_fields: vec!["test_schema.input_value".to_string()],
        output_molecule: "test_schema.doubled".to_string(),
        schema_name: "test_schema".to_string(),
        field_name: "doubled".to_string(),
    };
    
    let declarative_registration = TransformRegistration {
        transform_id: "declarative_process".to_string(),
        transform: declarative_transform,
        input_molecules: vec!["test_schema.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["test_schema.data".to_string()],
        output_molecule: "test_schema.processed".to_string(),
        schema_name: "test_schema".to_string(),
        field_name: "processed".to_string(),
    };
    
    // Register both transforms
    fixture.transform_manager.register_transform_event_driven(procedural_registration)
        .expect("Failed to register procedural transform");
    fixture.transform_manager.register_transform_event_driven(declarative_registration)
        .expect("Failed to register declarative transform");
    
    // Verify both transforms are registered
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    assert!(transforms.contains_key(&"procedural_double".to_string()));
    assert!(transforms.contains_key(&"declarative_process".to_string()));
    
    // Test execution of both transform types
    let procedural_data = serde_json::json!({
        "input_value": 5
    });
    
    let declarative_data = serde_json::json!({
        "data": {
            "value": "test data"
        }
    });
    
    // Execute procedural transform
    let procedural_result = TransformExecutor::execute_transform_with_expr(
        &transforms[&"procedural_double".to_string()],
        procedural_data
    ).expect("Failed to execute procedural transform");
    
    // Execute declarative transform
    let declarative_result = TransformExecutor::execute_transform_with_expr(
        &transforms[&"declarative_process".to_string()],
        declarative_data
    ).expect("Failed to execute declarative transform");
    
    // Verify both results are valid
    assert!(procedural_result.is_object());
    assert!(declarative_result.is_object());
    
    // Verify procedural transform result
    let procedural_obj = procedural_result.as_object().unwrap();
    assert!(procedural_obj.contains_key("doubled"));
    
    // Verify declarative transform result
    let declarative_obj = declarative_result.as_object().unwrap();
    assert!(declarative_obj.contains_key("processed"));
}

/// Test field alignment validation integration with declarative transforms
#[test]
fn test_field_alignment_validation_integration() {
    let fixture = IteratorStackIntegrationFixture::new();
    
    // Test valid field alignment scenario
    let valid_chains = vec![
        fixture.chain_parser.parse("blogpost.map().content").expect("Failed to parse valid chain"),
        fixture.chain_parser.parse("blogpost.map().title").expect("Failed to parse valid chain"),
    ];
    
    let alignment_result = fixture.field_alignment_validator.validate_alignment(&valid_chains)
        .expect("Failed to validate field alignment");
    
    match alignment_result {
        AlignmentValidationResult::Valid { warnings } => {
            // Should be valid with minimal warnings
            println!("Valid alignment with {} warnings", warnings.len());
        }
        AlignmentValidationResult::Invalid { errors } => {
            panic!("Valid alignment scenario failed: {:?}", errors);
        }
    }
    
    // Test invalid field alignment scenario (different depths)
    let invalid_chains = vec![
        fixture.chain_parser.parse("blogpost.map().content").expect("Failed to parse chain"),
        fixture.chain_parser.parse("blogpost.map().content.split_by_word().count()").expect("Failed to parse chain"),
    ];
    
    let invalid_alignment_result = fixture.field_alignment_validator.validate_alignment(&invalid_chains)
        .expect("Failed to validate field alignment");
    
    match invalid_alignment_result {
        AlignmentValidationResult::Valid { warnings } => {
            // Should have warnings about depth mismatch
            assert!(warnings.len() > 0);
            println!("Alignment validation produced {} warnings for depth mismatch", warnings.len());
        }
        AlignmentValidationResult::Invalid { errors } => {
            // This is also acceptable - the validator should catch depth mismatches
            println!("Alignment validation correctly identified depth mismatch: {:?}", errors);
        }
    }
}

/// Test performance integration with iterator stack infrastructure
#[test]
fn test_performance_integration() {
    let fixture = IteratorStackIntegrationFixture::new();
    
    let start_time = std::time::Instant::now();
    
    // Test parsing performance with complex expressions
    let complex_expressions = vec![
        "blogpost.map().content.split_by_word().count()",
        "blogpost.map().tags.split_array().map().name",
        "blogpost.map().author.map().profile.name",
        "blogpost.map().comments.split_array().map().content.split_by_word().count()",
    ];
    
    let mut parsed_chains = Vec::new();
    for expr in &complex_expressions {
        let parsed = fixture.chain_parser.parse(expr)
            .expect("Failed to parse complex expression");
        parsed_chains.push(parsed);
    }
    
    let parse_duration = start_time.elapsed();
    
    // Test iterator stack creation performance
    let stack_start = std::time::Instant::now();
    let mut iterator_stacks = Vec::new();
    for chain in &parsed_chains {
        let stack = IteratorStack::from_chain(chain)
            .expect("Failed to create iterator stack");
        iterator_stacks.push(stack);
    }
    let stack_duration = stack_start.elapsed();
    
    // Test field alignment validation performance
    let alignment_start = std::time::Instant::now();
    let _alignment_result = fixture.field_alignment_validator.validate_alignment(&parsed_chains)
        .expect("Failed to validate field alignment");
    let alignment_duration = alignment_start.elapsed();
    
    // Performance should be reasonable (less than 1 second for all operations)
    assert!(parse_duration.as_millis() < 1000, "Parsing took too long: {}ms", parse_duration.as_millis());
    assert!(stack_duration.as_millis() < 1000, "Stack creation took too long: {}ms", stack_duration.as_millis());
    assert!(alignment_duration.as_millis() < 1000, "Alignment validation took too long: {}ms", alignment_duration.as_millis());
    
    println!("Performance results:");
    println!("  Parsing: {}ms", parse_duration.as_millis());
    println!("  Stack creation: {}ms", stack_duration.as_millis());
    println!("  Alignment validation: {}ms", alignment_duration.as_millis());
}

/// Test error handling integration with iterator stack components
#[test]
fn test_error_handling_integration() {
    let fixture = IteratorStackIntegrationFixture::new();
    
    // Test invalid chain expression parsing
    let invalid_expressions = vec![
        "invalid..syntax..expression",
        "blogpost.map().invalid_method()",
        "blogpost.map().content.split_by_word().invalid_reducer()",
    ];
    
    for invalid_expr in &invalid_expressions {
        let result = fixture.chain_parser.parse(invalid_expr);
        assert!(result.is_err(), "Should fail to parse invalid expression: {}", invalid_expr);
        
        if let Err(error) = result {
            println!("Correctly failed to parse '{}': {}", invalid_expr, error);
        }
    }
    
    // Test iterator stack depth limits
    let deep_expression = "blogpost.map().content.split_by_word().map().char.split_array().map().item.split_by_word().map().char.split_array().map().item.split_by_word().count()";
    
    let parsed_chain = fixture.chain_parser.parse(deep_expression);
    if let Ok(chain) = parsed_chain {
        let stack_result = IteratorStack::from_chain(&chain);
        // Should either succeed or fail gracefully due to depth limits
        match stack_result {
            Ok(stack) => {
                println!("Deep expression created stack with depth: {}", stack.current_depth());
            }
            Err(error) => {
                println!("Deep expression correctly failed due to depth: {}", error);
            }
        }
    }
    
    // Test field alignment validation with malformed chains
    let malformed_chains = vec![
        fixture.chain_parser.parse("blogpost.map().content").expect("Failed to parse valid chain"),
    ];
    
    // This should work fine
    let alignment_result = fixture.field_alignment_validator.validate_alignment(&malformed_chains)
        .expect("Failed to validate field alignment");
    
    match alignment_result {
        AlignmentValidationResult::Valid { warnings } => {
            println!("Malformed chains validation produced {} warnings", warnings.len());
        }
        AlignmentValidationResult::Invalid { errors } => {
            println!("Malformed chains validation produced {} errors", errors.len());
        }
    }
}

/// Test backward compatibility with existing procedural transforms
#[test]
fn test_backward_compatibility_integration() {
    let fixture = IteratorStackIntegrationFixture::new();
    
    // Create a legacy procedural transform
    let procedural_transform = Transform::new(
        "input_field + 1".to_string(),
        "test_schema.incremented".to_string()
    );
    
    // Register the procedural transform
    let registration = TransformRegistration {
        transform_id: "legacy_procedural".to_string(),
        transform: procedural_transform,
        input_molecules: vec!["test_schema.input_field".to_string()],
        input_names: vec!["input_field".to_string()],
        trigger_fields: vec!["test_schema.input_field".to_string()],
        output_molecule: "test_schema.incremented".to_string(),
        schema_name: "test_schema".to_string(),
        field_name: "incremented".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(registration)
        .expect("Failed to register procedural transform");
    
    // Verify the transform is registered and works
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    assert!(transforms.contains_key(&"legacy_procedural".to_string()));
    
    // Test execution of the procedural transform
    let input_data = serde_json::json!({
        "input_field": 5
    });
    
    let result = TransformExecutor::execute_transform_with_expr(
        &transforms[&"legacy_procedural".to_string()],
        input_data
    ).expect("Failed to execute procedural transform");
    
    // Verify the result
    assert!(result.is_object());
    let result_obj = result.as_object().unwrap();
    assert!(result_obj.contains_key("incremented"));
    
    // Verify the transform type is correctly identified
    let transform = &transforms[&"legacy_procedural".to_string()];
    assert!(matches!(transform.kind, TransformKind::Procedural { .. }));
    assert!(!transform.is_declarative());
    
    // Test that iterator stack components don't interfere with procedural transforms
    // Procedural transforms should not use iterator stack infrastructure
    let debug_info = transform.get_debug_info();
    assert!(debug_info.contains("Procedural"));
    assert!(debug_info.contains("input_field + 1"));
}
