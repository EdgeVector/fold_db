# SSF-1-3 Implement custom deserialization for mixed format support

[Back to task list](./tasks.md)

## Description

Implement custom deserialization for `DeclarativeSchemaDefinition` to support both string expressions and `FieldDefinition` objects in the same schema. This enables mixed format schemas where some fields can use simplified string expressions while others use verbose `FieldDefinition` objects.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 20:30:00 | Created | N/A | Proposed | Task file created | User |
| 2025-01-27 20:45:00 | Status Update | Proposed | InProgress | Started implementation | User |
| 2025-01-27 21:15:00 | Status Update | InProgress | Review | Implementation completed, ready for review | User |
| 2025-01-27 21:30:00 | Status Update | Review | Done | Task approved and completed successfully | User |

## Requirements

### Functional Requirements
1. **Mixed Format Support**: Allow both string expressions and `FieldDefinition` objects in the same schema
2. **String Expression Conversion**: Convert string expressions to `FieldDefinition` with `atom_uuid` field
3. **Backward Compatibility**: All existing schemas continue to work without modification
4. **Error Handling**: Clear error messages for invalid string expressions

### Technical Requirements
1. **Custom Deserialization**: Implement `serde::Deserialize` for `DeclarativeSchemaDefinition`
2. **Field Type Inference**: Infer field type from string expression context
3. **Validation**: Validate string expressions are valid iterator stack expressions
4. **Performance**: Deserialization should be efficient and not significantly slower than standard

## Implementation Plan

### Phase 1: Custom Deserialization Structure
1. Create custom deserializer for `DeclarativeSchemaDefinition`
2. Handle both string and object field values
3. Convert strings to `FieldDefinition` with appropriate defaults

### Phase 2: String Expression Processing
1. Parse string expressions to extract field information
2. Set `atom_uuid` field based on expression
3. Apply default values from `JsonSchemaField`

### Phase 3: Validation and Error Handling
1. Validate string expressions are valid iterator stack syntax
2. Provide clear error messages for invalid formats
3. Handle edge cases gracefully

### Phase 4: Testing
1. Test mixed format schemas
2. Test backward compatibility
3. Test error handling
4. Performance testing

## Test Plan

### Unit Tests
1. **Mixed Format Parsing**: Test schemas with both string and object fields
2. **String Expression Conversion**: Verify strings are converted to proper `FieldDefinition`
3. **Backward Compatibility**: Ensure existing schemas still work
4. **Error Handling**: Test invalid string expressions produce clear errors

### Integration Tests
1. **End-to-End Mixed Format**: Test complete workflow with mixed format schemas
2. **Performance Comparison**: Compare deserialization performance with standard approach

### Success Criteria
- Mixed format schemas parse correctly
- String expressions convert to proper `FieldDefinition` objects
- All existing tests continue to pass
- Error messages are clear and helpful
- Performance impact is minimal (<10% slower)

## Files Modified

- `src/schema/types/json_schema.rs` - Add custom deserialization implementation
- `tests/unit/schema/mixed_format_tests.rs` - New test file for mixed format support
- `tests/integration/mixed_format_integration_tests.rs` - Integration tests

## Verification

1. Run `cargo test` to ensure all tests pass
2. Run `cargo test mixed_format` to verify new tests
3. Run existing schema tests to ensure backward compatibility
4. Performance benchmark comparison
