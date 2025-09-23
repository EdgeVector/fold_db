# Schema Integration Documentation

The Native Transform System (NTS-3) provides seamless integration with the native schema registry for type validation, field verification, and data integrity enforcement. This document covers schema integration features and usage patterns.

## Table of Contents

- [Overview](#overview)
- [Schema Registry](#schema-registry)
- [Schema Validation](#schema-validation)
- [Field Type System](#field-type-system)
- [Schema-aware Transforms](#schema-aware-transforms)
- [Error Handling](#error-handling)
- [Performance Considerations](#performance-considerations)

## Overview

Schema integration provides:
- **Type Safety**: Compile-time and runtime type validation
- **Data Integrity**: Field validation against schema definitions
- **Performance Optimization**: Type-aware execution paths
- **Error Prevention**: Early detection of type mismatches
- **Documentation**: Self-documenting transform specifications

## Schema Registry

### Basic Schema Operations

```rust
use datafold::transform::native_schema_registry::NativeSchemaRegistry;
use datafold::schema::types::errors::SchemaError;
use std::sync::Arc;

// Create schema registry with database operations
let schema_registry = Arc::new(NativeSchemaRegistry::new(
    Arc::new(MockDatabaseOperations)
));

// Load schema from JSON
let schema_json = r#"{
    "name": "user_schema",
    "schema_type": "Single",
    "fields": {
        "id": {
            "field_type": "Single",
            "field_mappers": {}
        },
        "name": {
            "field_type": "Single",
            "field_mappers": {}
        },
        "age": {
            "field_type": "Single",
            "field_mappers": {}
        }
    }
}"#;

let schema_name = schema_registry
    .load_native_schema_from_json(schema_json)
    .await?;

// Retrieve schema
let schema = schema_registry.get_schema("user_schema")?;

// List available schemas
let schemas = schema_registry.list_schemas().await?;
```

### Schema Structure

```rust
pub struct NativeSchema {
    pub name: String,
    pub fields: HashMap<String, FieldDefinition>,
    pub schema_type: SchemaType,
    pub payment_config: PaymentConfig,
}
```

### Field Definitions in Schemas

```rust
use datafold::transform::native::field_definition::FieldDefinition;
use datafold::transform::native::types::FieldType;

// Schema field definition
let field_def = FieldDefinition::new("email", FieldType::String)
    .with_required(true)
    .with_default(FieldValue::String("default@example.com".to_string()));
```

## Schema Validation

### Data Validation

The schema registry validates data against schema definitions:

```rust
use datafold::transform::native::types::FieldValue;

// Create test data
let user_data = FieldValue::Object(vec![
    ("id".to_string(), FieldValue::Integer(123)),
    ("name".to_string(), FieldValue::String("John Doe".to_string())),
    ("age".to_string(), FieldValue::Integer(30)),
    ("email".to_string(), FieldValue::String("john@example.com".to_string())),
].into_iter().collect());

// Validate against schema
let is_valid = schema_registry
    .validate_data("user_schema", &user_data)
    .await?;

println!("Data is valid: {}", is_valid);
```

### Validation Rules

1. **Field Existence**: Required fields must be present
2. **Type Matching**: Field values must match declared types
3. **Default Values**: Optional fields use defaults when missing
4. **Nested Validation**: Objects and arrays validate recursively

### Validation Error Types

```rust
pub enum SchemaValidationError {
    SchemaNotFound { schema_name: String },
    FieldNotFound { field: String },
    TypeMismatch { field: String, expected: FieldType, actual: FieldType },
    RequiredFieldMissing { field: String },
    InvalidFieldValue { field: String, reason: String },
    NestedValidationError { field: String, error: Box<SchemaValidationError> },
}
```

## Field Type System

### Supported Field Types

#### Primitive Types

```rust
// String type
FieldType::String

// Integer type
FieldType::Integer

// Number type (floating point)
FieldType::Number

// Boolean type
FieldType::Boolean

// Null type
FieldType::Null
```

#### Complex Types

```rust
// Array type with element type
FieldType::Array {
    element_type: Box::new(FieldType::String),
}

// Object type with field definitions
FieldType::Object {
    fields: HashMap::from([
        ("name".to_string(), FieldType::String),
        ("age".to_string(), FieldType::Integer),
    ]),
}
```

### Type Matching Rules

#### Primitive Type Matching

```rust
// String matching
FieldType::String.matches(&FieldValue::String("hello")) // true
FieldType::String.matches(&FieldValue::Integer(42))     // false

// Integer matching
FieldType::Integer.matches(&FieldValue::Integer(42))    // true
FieldType::Integer.matches(&FieldValue::Number(42.0))   // false (strict)

// Number matching
FieldType::Number.matches(&FieldValue::Integer(42))     // true (auto-conversion)
FieldType::Number.matches(&FieldValue::Number(42.0))    // true
FieldType::Number.matches(&FieldValue::String("42"))    // false

// Boolean matching
FieldType::Boolean.matches(&FieldValue::Boolean(true))  // true
FieldType::Boolean.matches(&FieldValue::Boolean(false)) // true
```

#### Array Type Matching

```rust
let array_type = FieldType::Array {
    element_type: Box::new(FieldType::Integer),
};

// Valid matches
array_type.matches(&FieldValue::Array(vec![
    FieldValue::Integer(1),
    FieldValue::Integer(2),
    FieldValue::Integer(3),
])) // true

// Invalid matches
array_type.matches(&FieldValue::Array(vec![
    FieldValue::String("not_int")
])) // false
```

#### Object Type Matching

```rust
let object_type = FieldType::Object {
    fields: HashMap::from([
        ("name".to_string(), FieldType::String),
        ("age".to_string(), FieldType::Integer),
    ]),
};

// Valid match
object_type.matches(&FieldValue::Object(vec![
    ("name".to_string(), FieldValue::String("John")),
    ("age".to_string(), FieldValue::Integer(30)),
].into_iter().collect())) // true

// Invalid match (missing required field)
object_type.matches(&FieldValue::Object(vec![
    ("name".to_string(), FieldValue::String("John")),
    // missing "age" field
].into_iter().collect())) // false
```

## Schema-aware Transforms

### Transform Specification with Schema

```rust
use datafold::transform::native::transform_spec::{TransformSpec, TransformType, MapTransform, FieldMapping};

// Define transform with schema integration
let spec = TransformSpec::new(
    "user_enrichment",
    vec![
        FieldDefinition::new("id", FieldType::Integer),
        FieldDefinition::new("name", FieldType::String),
        FieldDefinition::new("email", FieldType::String),
    ],
    FieldDefinition::new("enriched_user", FieldType::Object {
        fields: HashMap::from([
            ("user_id".to_string(), FieldType::Integer),
            ("display_name".to_string(), FieldType::String),
            ("is_active".to_string(), FieldType::Boolean),
        ]),
    }),
    TransformType::Map(MapTransform::new({
        let mut mappings = HashMap::new();
        mappings.insert("user_id".to_string(), FieldMapping::Direct {
            field: "id".to_string(),
        });
        mappings.insert("display_name".to_string(), FieldMapping::Function {
            name: "uppercase".to_string(),
            arguments: vec!["name".to_string()],
        });
        mappings.insert("is_active".to_string(), FieldMapping::Expression {
            expression: "email != \"\"".to_string(),
        });
        mappings
    })),
);
```

### Automatic Schema Validation

When executing transforms with schema integration:

```rust
use datafold::transform::native_executor::{NativeTransformExecutor, NativeTransformInput};

// Create executor with schema registry
let executor = NativeTransformExecutor::new(); // Includes schema registry

// Load schema first
let schema_json = /* schema definition */;
executor.schema_registry()
    .load_native_schema_from_json(schema_json)
    .await?;

// Execute transform with automatic validation
let input = NativeTransformInput {
    values: user_data,
    schema_name: Some("user_schema".to_string()),
};

let result = executor.execute_transform(&spec, input).await?;
```

### Schema Inheritance

Transforms can inherit field definitions from schemas:

```rust
// Schema defines fields
let schema_fields = vec![
    FieldDefinition::new("id", FieldType::Integer),
    FieldDefinition::new("name", FieldType::String),
    FieldDefinition::new("email", FieldType::String),
];

// Transform uses schema field definitions
let transform = TransformSpec::new(
    "user_transform",
    schema_fields, // Use schema field definitions
    FieldDefinition::new("result", FieldType::Object {
        fields: HashMap::new(),
    }),
    TransformType::Map(/* mappings */),
);
```

## Error Handling

### Schema Validation Errors

```rust
use datafold::transform::native_executor::TransformExecutionError;

match executor.execute_transform(&spec, input).await {
    Ok(result) => println!("Transform successful"),
    Err(TransformExecutionError::SchemaValidationError { schema, reason }) => {
        println!("Schema validation failed for '{}': {}", schema, reason);
    }
    Err(e) => println!("Other error: {:?}", e),
}
```

### Common Validation Scenarios

#### Missing Required Fields

```rust
// Schema requires "email" field
let invalid_data = FieldValue::Object(vec![
    ("id".to_string(), FieldValue::Integer(123)),
    ("name".to_string(), FieldValue::String("John")),
    // Missing required "email" field
].into_iter().collect());

// Validation will fail
let is_valid = schema_registry.validate_data("user_schema", &invalid_data).await?;
// Returns: false
```

#### Type Mismatches

```rust
// Schema expects integer for "age"
let invalid_data = FieldValue::Object(vec![
    ("id".to_string(), FieldValue::Integer(123)),
    ("name".to_string(), FieldValue::String("John")),
    ("age".to_string(), FieldValue::String("thirty")), // Wrong type
].into_iter().collect());

// Validation will fail
let is_valid = schema_registry.validate_data("user_schema", &invalid_data).await?;
// Returns: false
```

#### Nested Object Validation

```rust
// Schema expects nested object with specific fields
let invalid_data = FieldValue::Object(vec![
    ("user".to_string(), FieldValue::Object(vec![
        ("name".to_string(), FieldValue::String("John")),
        ("age".to_string(), FieldValue::String("thirty")), // Wrong type in nested object
    ].into_iter().collect())),
].into_iter().collect());

// Validation will fail for nested field
let is_valid = schema_registry.validate_data("user_schema", &invalid_data).await?;
// Returns: false
```

## Performance Considerations

### Schema Loading Performance

1. **Schema Caching**: Schemas are cached after loading
   ```rust
   // First load (slower - loads from database)
   let schema = schema_registry.get_schema("user_schema")?;

   // Subsequent loads (faster - from cache)
   let schema = schema_registry.get_schema("user_schema")?;
   ```

2. **Batch Validation**: Validate multiple records together
   ```rust
   // More efficient for multiple records
   let records = vec![record1, record2, record3];
   for record in records {
       schema_registry.validate_data("schema_name", &record).await?;
   }
   ```

### Transform Optimization

1. **Early Validation**: Validate input data before complex transforms
   ```rust
   // Validate first, then transform
   let input = NativeTransformInput {
       values: data,
       schema_name: Some("input_schema".to_string()),
   };

   // Only execute transform if validation passes
   if schema_registry.validate_data("input_schema", &data).await? {
       let result = executor.execute_transform(&spec, input).await?;
   }
   ```

2. **Schema-aware Field Selection**: Use only required fields
   ```rust
   // Define transform with minimal required fields
   let minimal_spec = TransformSpec::new(
       "minimal_transform",
       vec![
           FieldDefinition::new("id", FieldType::Integer),     // Only what's needed
           FieldDefinition::new("name", FieldType::String),    // Only what's needed
       ],
       FieldDefinition::new("result", FieldType::String),
       TransformType::Map(MapTransform::new({
           let mut mappings = HashMap::new();
           mappings.insert("output".to_string(), FieldMapping::Expression {
               expression: "id + \": \" + name".to_string(),
           });
           mappings
       })),
   );
   ```

### Memory Usage

1. **Field Definition Reuse**: Reuse field definitions across transforms
   ```rust
   // Reuse field definitions
   const USER_FIELDS: &[FieldDefinition] = &[
       FieldDefinition::new("id", FieldType::Integer),
       FieldDefinition::new("name", FieldType::String),
       FieldDefinition::new("email", FieldType::String),
   ];

   let transform1 = TransformSpec::new("t1", USER_FIELDS.to_vec(), /* ... */);
   let transform2 = TransformSpec::new("t2", USER_FIELDS.to_vec(), /* ... */);
   ```

2. **Schema Field Inference**: Let schemas infer field types when possible
   ```rust
   // Schema can infer types from data
   let schema = schema_registry.load_native_schema_from_json(schema_json).await?;
   // Field types are automatically inferred
   ```

## Best Practices

### Schema Design

1. **Use Appropriate Field Types**: Choose the most specific type possible
   ```rust
   // Specific types
   FieldDefinition::new("user_id", FieldType::Integer)
   FieldDefinition::new("email", FieldType::String)
   FieldDefinition::new("score", FieldType::Number)

   // Avoid generic types when possible
   // FieldDefinition::new("data", FieldType::Any)  // Less type-safe
   ```

2. **Define Clear Field Relationships**: Use object types for related fields
   ```rust
   // Clear structure
   FieldDefinition::new("user", FieldType::Object {
       fields: HashMap::from([
           ("id".to_string(), FieldType::Integer),
           ("profile".to_string(), FieldType::Object {
               fields: HashMap::from([
                   ("name".to_string(), FieldType::String),
                   ("email".to_string(), FieldType::String),
               ]),
           }),
       ]),
   })
   ```

3. **Set Appropriate Required Flags**: Mark truly required fields as required
   ```rust
   // Required fields (must be present and valid)
   FieldDefinition::new("id", FieldType::Integer)
       .with_required(true)

   // Optional fields (can be missing, will use default)
   FieldDefinition::new("nickname", FieldType::String)
       .with_required(false)
       .with_default(FieldValue::String("".to_string()))
   ```

### Transform Design

1. **Validate Early**: Use schema validation before expensive operations
   ```rust
   // Validate input data first
   if !schema_registry.validate_data("input_schema", &input_data).await? {
       return Err("Invalid input data".into());
   }

   // Only proceed if validation passes
   let result = executor.execute_transform(&spec, input).await?;
   ```

2. **Use Schema-aware Field Selection**: Only request needed fields
   ```rust
   // Only include fields that will be used
   let focused_spec = TransformSpec::new(
       "focused_transform",
       vec![
           FieldDefinition::new("id", FieldType::Integer),
           FieldDefinition::new("name", FieldType::String),
       ],
       FieldDefinition::new("result", FieldType::String),
       TransformType::Map(/* minimal mappings */),
   );
   ```

3. **Handle Validation Errors Gracefully**: Provide meaningful error messages
   ```rust
   match executor.execute_transform(&spec, input).await {
       Ok(result) => println!("Success"),
       Err(TransformExecutionError::SchemaValidationError { schema, reason }) => {
           println!("Schema '{}' validation failed: {}", schema, reason);
           // Log detailed error information
       }
       Err(e) => println!("Other error: {:?}", e),
   }
   ```

### Performance Optimization

1. **Cache Schema Operations**: Reuse loaded schemas
   ```rust
   // Cache schema lookup
   let schema = schema_registry.get_schema("user_schema")?;
   let field_def = schema.fields.get("email").unwrap();

   // Reuse in multiple transforms
   let transform1 = /* uses field_def */;
   let transform2 = /* uses field_def */;
   ```

2. **Use Batch Validation**: Validate multiple records efficiently
   ```rust
   // Process records in batches
   let batch_size = 100;
   for chunk in records.chunks(batch_size) {
       for record in chunk {
           schema_registry.validate_data("schema", record).await?;
           executor.execute_transform(&spec, record).await?;
       }
   }
   ```

3. **Monitor Validation Performance**: Track validation overhead
   ```rust
   use std::time::Instant;

   let start = Instant::now();
   let is_valid = schema_registry.validate_data("schema", &data).await?;
   let validation_time = start.elapsed();

   if validation_time.as_millis() > 10 {
       println!("Slow validation: {:?}", validation_time);
   }
   ```

This comprehensive schema integration documentation provides all the tools needed to create type-safe, validated transforms with optimal performance.