# DTS-1-5 Implement validation for declarative transform structures

[Back to task list](./tasks.md)

## Description

Implement validation logic to ensure declarative transforms have required fields and valid configurations. This validation will prevent invalid transform definitions from being processed and provide clear error messages to users.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 12:00:00 | Status Change | N/A | Proposed | Task file created | User |

## Requirements

1. **Required Field Validation**: Ensure all required fields are present in declarative transforms
2. **Schema Type Validation**: Validate schema types and their requirements
3. **Key Configuration Validation**: Ensure HashRange schemas have proper key configuration
4. **Field Definition Validation**: Validate field definitions have required information
5. **Error Messages**: Provide clear, actionable error messages for validation failures

## Implementation Plan

### Step 1: Implement Basic Validation Traits
- Create validation traits for declarative transform structures
- Implement validation methods for each struct type
- Return validation results with error details

### Step 2: Implement DeclarativeSchemaDefinition Validation
- Validate required fields (name, schema_type, fields)
- Validate schema type is one of supported values
- Validate key configuration when required
- Validate field definitions are not empty

### Step 3: Implement KeyConfig Validation
- Validate hash_field and range_field are not empty
- Validate field expressions are valid syntax
- Ensure both fields are present for HashRange schemas

### Step 4: Implement FieldDefinition Validation
- Validate atom_uuid expressions when present
- Validate field_type values when specified
- Ensure at least one field property is defined

### Step 5: Implement Error Reporting
- Create structured error types for validation failures
- Provide context about which field failed validation
- Include suggestions for fixing validation errors

## Verification

1. **Validation Logic**: All validation rules are properly implemented
2. **Error Messages**: Clear, actionable error messages are provided
3. **Edge Cases**: Invalid configurations are properly caught and reported
4. **Performance**: Validation runs efficiently without performance issues
5. **Integration**: Validation integrates properly with existing error handling

## Files Modified

- `src/schema/types/json_schema.rs` - Add validation methods to structs
- `src/schema/types/validation.rs` - Create validation traits and error types
- `tests/unit/schema/validation_tests.rs` - Add validation tests
- `tests/integration/validation_integration_tests.rs` - Add integration tests

## Test Plan

### Objective
Verify that validation logic properly catches invalid declarative transform configurations and provides clear error messages to help users fix issues.

### Test Scope
- Validation logic for all declarative transform structs
- Error message clarity and usefulness
- Edge case handling for invalid configurations
- Performance of validation operations

### Environment & Setup
- Standard Rust test environment
- Test data fixtures with various validation scenarios
- Error message validation utilities

### Mocking Strategy
- No external dependencies to mock
- Use static test data for consistent validation testing
- Create test fixtures for various invalid configurations

### Key Test Scenarios
1. **Required Field Validation**: Test missing required fields are caught
2. **Schema Type Validation**: Test invalid schema types are rejected
3. **Key Configuration Validation**: Test HashRange schemas without keys are rejected
4. **Field Definition Validation**: Test invalid field definitions are caught
5. **Error Message Quality**: Test error messages are clear and actionable
6. **Performance**: Test validation runs efficiently for large schemas
7. **Integration**: Test validation integrates with existing error handling

### Success Criteria
- All validation tests pass
- Invalid configurations are properly caught and reported
- Error messages are clear and actionable
- Validation performance is acceptable
- No false positives or false negatives
- Integration with existing error handling works correctly
