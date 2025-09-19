# Universal Key Configuration Testing Guide

This guide provides comprehensive information about testing universal key configuration functionality in FoldDB.

## Overview

Universal key configuration allows schemas to define their key fields in a unified way across all schema types (Single, Range, HashRange). This eliminates the need for hardcoded field names and provides consistent key management.

## Schema Types and Universal Key Configuration

### Single Schema
```rust
Schema {
    name: "TestSingle".to_string(),
    schema_type: SchemaType::Single,
    key: Some(KeyConfig {
        hash_field: "".to_string(),      // Single schemas don't use hash fields
        range_field: "".to_string(),     // Single schemas don't use range fields
    }),
    fields: HashMap::new(),
    hash: Some("test_hash".to_string()),
    payment_config: SchemaPaymentConfig::default(),
}
```

### Range Schema
```rust
Schema {
    name: "TestRange".to_string(),
    schema_type: SchemaType::Range { 
        range_key: "timestamp".to_string() 
    },
    key: Some(KeyConfig {
        hash_field: "".to_string(),           // Range schemas don't use hash fields
        range_field: "timestamp".to_string(), // Range field name
    }),
    fields: HashMap::new(),
    hash: Some("test_hash".to_string()),
    payment_config: SchemaPaymentConfig::default(),
}
```

### HashRange Schema
```rust
Schema {
    name: "TestHashRange".to_string(),
    schema_type: SchemaType::HashRange,
    key: Some(KeyConfig {
        hash_field: "user_id".to_string(),     // Hash field name
        range_field: "timestamp".to_string(),  // Range field name
    }),
    fields: HashMap::new(),
    hash: Some("test_hash".to_string()),
    payment_config: SchemaPaymentConfig::default(),
}
```

## Test Patterns

### 1. Schema Creation with Test Fixtures

Use test fixtures to create schemas with universal key configuration:

```rust
struct UniversalKeyTestFixture {
    mutation_service: MutationService,
}

impl UniversalKeyTestFixture {
    fn create_hashrange_schema_with_universal_key(
        &self, 
        name: &str, 
        hash_field: &str, 
        range_field: &str
    ) -> Schema {
        // Implementation details...
    }
}
```

### 2. Field Processing Tests

Test that field processing utilities work with universal key extraction:

```rust
#[test]
fn test_universal_key_field_processing() {
    let fixture = UniversalKeyTestFixture::new();
    let schema = fixture.create_hashrange_schema_with_universal_key(
        "TestSchema", "user_id", "timestamp"
    );
    
    // Test field name extraction
    let (hash_field, range_field) = fixture.mutation_service
        .get_hashrange_key_field_names(&schema)?;
    assert_eq!(hash_field, "user_id");
    assert_eq!(range_field, "timestamp");
}
```

### 3. Error Handling Tests

Test error scenarios for invalid universal key configurations:

```rust
#[test]
fn test_universal_key_error_handling() {
    let fixture = UniversalKeyTestFixture::new();
    
    // Test HashRange schema without key configuration
    let schema_no_key = Schema {
        name: "TestNoKey".to_string(),
        schema_type: SchemaType::HashRange,
        key: None,  // Missing key configuration
        fields: HashMap::new(),
        hash: Some("test_hash".to_string()),
        payment_config: SchemaPaymentConfig::default(),
    };
    
    let result = fixture.mutation_service.get_hashrange_key_field_names(&schema_no_key);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("requires key configuration"));
}
```

### 4. Validation Tests

Test that schemas validate correctly with universal key configuration:

```rust
#[test]
fn test_universal_key_validation_rules() {
    let fixture = UniversalKeyTestFixture::new();
    
    // Test Single schema with universal key (should work)
    let single_schema = fixture.create_single_schema_with_universal_key("TestSingle");
    assert_eq!(single_schema.schema_type, SchemaType::Single);
    assert!(single_schema.key.is_some());
    
    // Test Range schema with universal key (should work)
    let range_schema = fixture.create_range_schema_with_universal_key("TestRange", "timestamp");
    assert!(matches!(range_schema.schema_type, SchemaType::Range { .. }));
    assert!(range_schema.key.is_some());
    
    // Test HashRange schema with universal key (should work)
    let hashrange_schema = fixture.create_hashrange_schema_with_universal_key(
        "TestHashRange", "user_id", "timestamp"
    );
    assert_eq!(hashrange_schema.schema_type, SchemaType::HashRange);
    assert!(hashrange_schema.key.is_some());
}
```

## Best Practices

### 1. Use Test Fixtures
Always use test fixtures to create schemas with universal key configuration. This ensures consistency and makes tests easier to maintain.

### 2. Test All Schema Types
When testing universal key functionality, test all three schema types (Single, Range, HashRange) to ensure comprehensive coverage.

### 3. Test Error Scenarios
Include tests for error scenarios such as:
- Missing key configuration
- Empty key fields
- Invalid key field names

### 4. Validate Field Extraction
Test that field name extraction works correctly with universal key configuration:
- HashRange schemas should extract both hash and range field names
- Range schemas should extract range field names
- Single schemas should handle empty key fields gracefully

### 5. Test Backward Compatibility
While focusing on universal key configuration, ensure that existing functionality continues to work.

## Common Test Patterns

### Schema Validation Pattern
```rust
#[test]
fn test_schema_with_universal_key() {
    let fixture = UniversalKeyTestFixture::new();
    let schema = fixture.create_schema_with_universal_key("TestSchema");
    
    // Validate schema structure
    assert_eq!(schema.name, "TestSchema");
    assert!(schema.key.is_some());
    
    let key_config = schema.key.unwrap();
    // Validate key configuration based on schema type
}
```

### Field Processing Pattern
```rust
#[test]
fn test_field_processing_with_universal_key() {
    let fixture = UniversalKeyTestFixture::new();
    let schema = fixture.create_schema_with_universal_key("TestSchema");
    
    // Test field processing utilities
    let field_names = fixture.mutation_service.get_field_names(&schema)?;
    // Validate field names
}
```

### Error Handling Pattern
```rust
#[test]
fn test_error_handling_with_universal_key() {
    let fixture = UniversalKeyTestFixture::new();
    
    // Create invalid schema
    let invalid_schema = create_invalid_schema();
    
    // Test error handling
    let result = fixture.mutation_service.process_schema(&invalid_schema);
    assert!(result.is_err());
    // Validate error message
}
```

## Migration from Legacy Patterns

When migrating from legacy `range_key` patterns to universal key configuration:

1. **Update Schema Creation**: Replace hardcoded field names with universal key configuration
2. **Update Field Processing**: Use universal key extraction methods instead of hardcoded field names
3. **Update Tests**: Replace legacy test patterns with universal key test patterns
4. **Remove Legacy Code**: Remove backward compatibility code once migration is complete

## Troubleshooting

### Common Issues

1. **Missing Key Configuration**: Ensure all schemas have proper key configuration
2. **Empty Key Fields**: Validate that key fields are not empty for schemas that require them
3. **Field Name Mismatches**: Ensure field names in key configuration match actual field names

### Debug Tips

1. Use `println!` statements to debug field name extraction
2. Check schema structure with `dbg!` macro
3. Validate key configuration before processing

## Conclusion

Universal key configuration provides a consistent and flexible way to manage schema keys across all schema types. By following these testing patterns and best practices, you can ensure that your universal key functionality works correctly and maintains backward compatibility.
