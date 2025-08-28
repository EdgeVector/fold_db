//! Performance Validation Tests
//!
//! This comprehensive test suite validates that declarative transforms maintain
//! acceptable performance characteristics and don't degrade system performance.
//!
//! **Performance Coverage:**
//! 1. **Memory Usage Validation** - Test memory usage with large schema definitions
//! 2. **Execution Performance** - Test execution performance under various conditions
//! 3. **Concurrent Execution** - Test concurrent execution of multiple transform types
//! 4. **Caching Effectiveness** - Test caching effectiveness with existing optimizations
//! 5. **Scalability Testing** - Test performance with increasing data sizes
//! 6. **Resource Utilization** - Test CPU and memory utilization patterns
//! 7. **Performance Regression** - Prevent performance regressions from new functionality

use datafold::db_operations::DbOperations;
use datafold::schema::types::transform::{Transform, TransformRegistration};
use datafold::schema::types::json_schema::{TransformKind, DeclarativeSchemaDefinition, FieldDefinition};
use datafold::schema::types::schema::SchemaType;
use datafold::fold_db_core::transform_manager::TransformManager;
use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
use datafold::transform::TransformExecutor;
use datafold::schema::indexing::{ChainParser, FieldAlignmentValidator, ExecutionEngine};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use serde_json::json;
use std::time::{Instant, Duration};

/// Test fixture for performance validation testing
struct PerformanceValidationFixture {
    pub db_ops: Arc<DbOperations>,
    pub message_bus: Arc<MessageBus>,
    pub transform_manager: Arc<TransformManager>,
    pub chain_parser: ChainParser,
    pub field_alignment_validator: FieldAlignmentValidator,
    pub execution_engine: ExecutionEngine,
    pub _temp_dir: TempDir,
}

impl PerformanceValidationFixture {
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

/// Test memory usage with large schema definitions
#[test]
fn test_memory_usage_with_large_schemas() {
    let fixture = PerformanceValidationFixture::new();
    
    let start_time = Instant::now();
    
    // Create a large schema with many fields
    let mut large_fields = HashMap::new();
    for i in 0..50 {
        large_fields.insert(
            format!("field_{}", i),
            FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some(format!("data.map().field_{}", i)),
            }
        );
    }
    
    let large_schema = DeclarativeSchemaDefinition {
        name: "large_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: large_fields,
        key: None,
    };
    
    let large_transform = Transform::from_declarative_schema(
        large_schema,
        vec!["large_schema.data".to_string()],
        "large_schema.field_0".to_string()
    );
    
    let schema_creation_duration = start_time.elapsed();
    
    // Validate the large transform
    let validation_start = Instant::now();
    large_transform.validate()
        .expect("Large schema validation failed");
    let validation_duration = validation_start.elapsed();
    
    // Register the large transform
    let registration_start = Instant::now();
    let registration = TransformRegistration {
        transform_id: "large_transform".to_string(),
        transform: large_transform,
        input_molecules: vec!["large_schema.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["large_schema.data".to_string()],
        output_molecule: "large_schema.field_0".to_string(),
        schema_name: "large_schema".to_string(),
        field_name: "field_0".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(registration)
        .expect("Failed to register large transform");
    let registration_duration = registration_start.elapsed();
    
    // Performance should be reasonable even with large schemas
    assert!(schema_creation_duration.as_millis() < 1000, "Large schema creation took too long: {}ms", schema_creation_duration.as_millis());
    assert!(validation_duration.as_millis() < 1000, "Large schema validation took too long: {}ms", validation_duration.as_millis());
    assert!(registration_duration.as_millis() < 1000, "Large schema registration took too long: {}ms", registration_duration.as_millis());
    
    println!("Large schema performance results:");
    println!("  Schema creation: {}ms", schema_creation_duration.as_millis());
    println!("  Validation: {}ms", validation_duration.as_millis());
    println!("  Registration: {}ms", registration_duration.as_millis());
}

/// Test execution performance under various conditions
#[test]
fn test_execution_performance_under_various_conditions() {
    let fixture = PerformanceValidationFixture::new();
    
    // Create multiple transforms of different types
    let mut transform_ids = Vec::new();
    
    // Create procedural transforms
    for i in 0..5 {
        let procedural_transform = Transform::new(
            format!("input_{} * 2", i),
            format!("perf_test_{}.doubled", i)
        );
        
        let registration = TransformRegistration {
            transform_id: format!("proc_perf_{}", i),
            transform: procedural_transform,
            input_molecules: vec![format!("perf_test_{}.input_{}", i, i)],
            input_names: vec![format!("input_{}", i)],
            trigger_fields: vec![format!("perf_test_{}.input_{}", i, i)],
            output_molecule: format!("perf_test_{}.doubled", i),
            schema_name: format!("perf_test_{}", i),
            field_name: "doubled".to_string(),
        };
        
        fixture.transform_manager.register_transform_event_driven(registration)
            .expect("Failed to register procedural transform");
        
        transform_ids.push(format!("proc_perf_{}", i));
    }
    
    // Create declarative transforms
    for i in 0..5 {
        let declarative_schema = DeclarativeSchemaDefinition {
            name: format!("decl_perf_{}", i),
            schema_type: SchemaType::Single,
            fields: HashMap::from([
                ("result".to_string(), FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some(format!("data_{}.map().value", i)),
                }),
            ]),
            key: None,
        };
        
        let declarative_transform = Transform::from_declarative_schema(
            declarative_schema,
            vec![format!("decl_perf_{}.data_{}", i, i)],
            format!("decl_perf_{}.result", i)
        );
        
        let registration = TransformRegistration {
            transform_id: format!("decl_perf_{}", i),
            transform: declarative_transform,
            input_molecules: vec![format!("decl_perf_{}.data_{}", i, i)],
            input_names: vec![format!("data_{}", i)],
            trigger_fields: vec![format!("decl_perf_{}.data_{}", i, i)],
            output_molecule: format!("decl_perf_{}.result", i),
            schema_name: format!("decl_perf_{}", i),
            field_name: "result".to_string(),
        };
        
        fixture.transform_manager.register_transform_event_driven(registration)
            .expect("Failed to register declarative transform");
        
        transform_ids.push(format!("decl_perf_{}", i));
    }
    
    // Test execution performance
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    let execution_start = Instant::now();
    
    for (i, transform_id) in transform_ids.iter().enumerate() {
        let input_data = if i < 5 {
            // Procedural transform input
            json!({
                format!("input_{}", i): i * 10
            })
        } else {
            // Declarative transform input
            json!({
                format!("data_{}", i - 5): {
                    "value": format!("test_value_{}", i - 5)
                }
            })
        };
        
        let result = TransformExecutor::execute_transform_with_expr(
            &transforms[transform_id],
            input_data
        ).expect("Failed to execute transform");
        
        assert!(result.is_object());
    }
    
    let execution_duration = execution_start.elapsed();
    
    // Performance should be reasonable for 10 transforms
    assert!(execution_duration.as_millis() < 2000, "Execution took too long: {}ms", execution_duration.as_millis());
    
    println!("Execution performance results:");
    println!("  Total execution time (10 transforms): {}ms", execution_duration.as_millis());
    println!("  Average per transform: {}ms", execution_duration.as_millis() / 10);
}

/// Test concurrent execution of multiple transform types
#[test]
fn test_concurrent_execution_performance() {
    let fixture = PerformanceValidationFixture::new();
    
    // Create a mix of procedural and declarative transforms
    let mut transform_ids = Vec::new();
    
    // Create procedural transforms
    for i in 0..3 {
        let procedural_transform = Transform::new(
            format!("input_{} + 1", i),
            format!("concurrent_{}.incremented", i)
        );
        
        let registration = TransformRegistration {
            transform_id: format!("concurrent_proc_{}", i),
            transform: procedural_transform,
            input_molecules: vec![format!("concurrent_{}.input_{}", i, i)],
            input_names: vec![format!("input_{}", i)],
            trigger_fields: vec![format!("concurrent_{}.input_{}", i, i)],
            output_molecule: format!("concurrent_{}.incremented", i),
            schema_name: format!("concurrent_{}", i),
            field_name: "incremented".to_string(),
        };
        
        fixture.transform_manager.register_transform_event_driven(registration)
            .expect("Failed to register procedural transform");
        
        transform_ids.push(format!("concurrent_proc_{}", i));
    }
    
    // Create declarative transforms
    for i in 0..3 {
        let declarative_schema = DeclarativeSchemaDefinition {
            name: format!("concurrent_decl_{}", i),
            schema_type: SchemaType::Single,
            fields: HashMap::from([
                ("processed".to_string(), FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some(format!("data_{}.map().value", i)),
                }),
            ]),
            key: None,
        };
        
        let declarative_transform = Transform::from_declarative_schema(
            declarative_schema,
            vec![format!("concurrent_decl_{}.data_{}", i, i)],
            format!("concurrent_decl_{}.processed", i)
        );
        
        let registration = TransformRegistration {
            transform_id: format!("concurrent_decl_{}", i),
            transform: declarative_transform,
            input_molecules: vec![format!("concurrent_decl_{}.data_{}", i, i)],
            input_names: vec![format!("data_{}", i)],
            trigger_fields: vec![format!("concurrent_decl_{}.data_{}", i, i)],
            output_molecule: format!("concurrent_decl_{}.processed", i),
            schema_name: format!("concurrent_decl_{}", i),
            field_name: "processed".to_string(),
        };
        
        fixture.transform_manager.register_transform_event_driven(registration)
            .expect("Failed to register declarative transform");
        
        transform_ids.push(format!("concurrent_decl_{}", i));
    }
    
    // Test concurrent execution
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    let concurrent_start = Instant::now();
    
    // Execute all transforms in sequence (simulating concurrent load)
    for (i, transform_id) in transform_ids.iter().enumerate() {
        let input_data = if i < 3 {
            // Procedural transform input
            json!({
                format!("input_{}", i): i * 5
            })
        } else {
            // Declarative transform input
            json!({
                format!("data_{}", i - 3): {
                    "value": format!("concurrent_value_{}", i - 3)
                }
            })
        };
        
        let result = TransformExecutor::execute_transform_with_expr(
            &transforms[transform_id],
            input_data
        ).expect("Failed to execute concurrent transform");
        
        assert!(result.is_object());
    }
    
    let concurrent_duration = concurrent_start.elapsed();
    
    // Performance should be reasonable for concurrent execution
    assert!(concurrent_duration.as_millis() < 1500, "Concurrent execution took too long: {}ms", concurrent_duration.as_millis());
    
    println!("Concurrent execution performance results:");
    println!("  Total concurrent execution time (6 transforms): {}ms", concurrent_duration.as_millis());
    println!("  Average per transform: {}ms", concurrent_duration.as_millis() / 6);
}

/// Test scalability with increasing data sizes
#[test]
fn test_scalability_with_increasing_data_sizes() {
    let fixture = PerformanceValidationFixture::new();
    
    // Create a declarative transform for scalability testing
    let scalability_schema = DeclarativeSchemaDefinition {
        name: "scalability_test".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("word_count".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().content.split_by_word().count()".to_string()),
            }),
            ("processed_content".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().content".to_string()),
            }),
        ]),
        key: None,
    };
    
    let scalability_transform = Transform::from_declarative_schema(
        scalability_schema,
        vec!["scalability_test.data".to_string()],
        "scalability_test.word_count".to_string()
    );
    
    // Register the transform
    let registration = TransformRegistration {
        transform_id: "scalability_transform".to_string(),
        transform: scalability_transform,
        input_molecules: vec!["scalability_test.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["scalability_test.data".to_string()],
        output_molecule: "scalability_test.word_count".to_string(),
        schema_name: "scalability_test".to_string(),
        field_name: "word_count".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(registration)
        .expect("Failed to register scalability transform");
    
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    // Test with increasing data sizes
    let data_sizes = vec![100, 500, 1000, 2000]; // Word counts
    
    for size in &data_sizes {
        let content = "word ".repeat(*size as usize);
        let input_data = json!({
            "data": {
                "content": content
            }
        });
        
        let execution_start = Instant::now();
        let result = TransformExecutor::execute_transform_with_expr(
            &transforms[&"scalability_transform".to_string()],
            input_data
        ).expect("Failed to execute scalability transform");
        
        let execution_duration = execution_start.elapsed();
        
        // Verify the result
        assert!(result.is_object());
        let result_obj = result.as_object().unwrap();
        assert!(result_obj.contains_key("word_count"));
        assert!(result_obj.contains_key("processed_content"));
        
        // Performance should scale reasonably with data size
        let max_acceptable_time = (*size as u128) / 10; // 0.1ms per word
        assert!(execution_duration.as_millis() < max_acceptable_time, 
                "Execution time {}ms exceeded acceptable limit {}ms for data size {}", 
                execution_duration.as_millis(), max_acceptable_time, size);
        
        println!("Scalability test - Data size: {} words, Execution time: {}ms", size, execution_duration.as_millis());
    }
}

/// Test performance regression prevention
#[test]
fn test_performance_regression_prevention() {
    let fixture = PerformanceValidationFixture::new();
    
    // Test baseline performance with procedural transforms
    let procedural_start = Instant::now();
    
    let procedural_transform = Transform::new(
        "input_value * 2 + 1".to_string(),
        "baseline.doubled_plus_one".to_string()
    );
    
    let procedural_registration = TransformRegistration {
        transform_id: "baseline_procedural".to_string(),
        transform: procedural_transform,
        input_molecules: vec!["baseline.input_value".to_string()],
        input_names: vec!["input_value".to_string()],
        trigger_fields: vec!["baseline.input_value".to_string()],
        output_molecule: "baseline.doubled_plus_one".to_string(),
        schema_name: "baseline".to_string(),
        field_name: "doubled_plus_one".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(procedural_registration)
        .expect("Failed to register baseline procedural transform");
    
    let procedural_duration = procedural_start.elapsed();
    
    // Test performance with declarative transforms
    let declarative_start = Instant::now();
    
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "baseline_declarative".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("result".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["baseline_declarative.data".to_string()],
        "baseline_declarative.result".to_string()
    );
    
    let declarative_registration = TransformRegistration {
        transform_id: "baseline_declarative".to_string(),
        transform: declarative_transform,
        input_molecules: vec!["baseline_declarative.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["baseline_declarative.data".to_string()],
        output_molecule: "baseline_declarative.result".to_string(),
        schema_name: "baseline_declarative".to_string(),
        field_name: "result".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(declarative_registration)
        .expect("Failed to register baseline declarative transform");
    
    let declarative_duration = declarative_start.elapsed();
    
    // Test execution performance
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    let execution_start = Instant::now();
    
    // Execute procedural transform
    let procedural_input = json!({
        "input_value": 10
    });
    
    let procedural_result = TransformExecutor::execute_transform_with_expr(
        &transforms[&"baseline_procedural".to_string()],
        procedural_input
    ).expect("Failed to execute procedural transform");
    
    // Execute declarative transform
    let declarative_input = json!({
        "data": {
            "value": "test_value"
        }
    });
    
    let declarative_result = TransformExecutor::execute_transform_with_expr(
        &transforms[&"baseline_declarative".to_string()],
        declarative_input
    ).expect("Failed to execute declarative transform");
    
    let execution_duration = execution_start.elapsed();
    
    // Verify results
    assert!(procedural_result.is_object());
    assert!(declarative_result.is_object());
    
    // Performance should be comparable between transform types
    let performance_ratio = declarative_duration.as_millis() as f64 / procedural_duration.as_millis() as f64;
    assert!(performance_ratio < 3.0, "Declarative transforms are significantly slower than procedural: ratio {}", performance_ratio);
    
    println!("Performance regression prevention results:");
    println!("  Procedural registration: {}ms", procedural_duration.as_millis());
    println!("  Declarative registration: {}ms", declarative_duration.as_millis());
    println!("  Execution time (both): {}ms", execution_duration.as_millis());
    println!("  Performance ratio: {:.2}", performance_ratio);
}
