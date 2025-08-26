# [DTS-1-8] Validation and Error Handling Using Existing Infrastructure

[Back to task list](./tasks.md)

## Description

Implement comprehensive validation for declarative transforms using the existing iterator stack infrastructure. This task focuses on leveraging existing field alignment validation, error types, and validation logic to ensure declarative transforms meet all requirements and integrate seamlessly with the existing system.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 16:30:00 | Created | N/A | Proposed | Task file created | AI Agent |

## Requirements

1. **Field Alignment Validation**: Use existing field alignment validation from `src/schema/indexing/field_alignment.rs`
2. **Error Type Integration**: Leverage existing error types from `src/schema/indexing/errors.rs`
3. **Validation Logic**: Apply existing validation logic for iterator depth and branch compatibility
4. **Schema Validation**: Ensure declarative transforms have required fields and valid configurations
5. **Integration Testing**: Verify validation works correctly with existing infrastructure

## Implementation Plan

### Step 1: Integrate with Existing Field Alignment Validation
- **Use existing `FieldAlignmentValidator`** from `src/schema/indexing/field_alignment.rs`
- **Leverage existing validation logic** for iterator depth and branch compatibility
- **Apply existing field alignment rules** (1:1, broadcast, reduced) to declarative transforms
- **Ensure consistent behavior** with existing procedural transform validation

### Step 2: Implement Declarative Schema Validation
- **Validate schema name** and basic structure requirements
- **Validate schema type** ("Single" or "HashRange") and key configuration
- **Validate field definitions** and required atom UUID mappings
- **Ensure required fields** are present for each schema type

### Step 3: Integrate with Existing Error Types
- **Use existing `IteratorStackError`** types from `src/schema/indexing/errors.rs`
- **Map validation errors** to appropriate existing error types
- **Maintain consistent error messages** with existing system
- **Provide clear feedback** for validation failures

### Step 4: Implement Iterator Expression Validation
- **Parse declarative expressions** into existing chain format for validation
- **Use existing chain parser** to validate expression syntax
- **Apply existing depth validation** for iterator expressions
- **Validate branch compatibility** using existing logic

### Step 5: Create Validation Functions
- **Implement `validate_declarative_schema`** function using existing infrastructure
- **Create validation helpers** for specific validation requirements
- **Integrate validation** into schema loading and transform creation
- **Add comprehensive error reporting** for validation failures

## Verification

1. **Field Alignment**: Existing field alignment validation works for declarative transforms
2. **Error Handling**: Proper error types and messages are used consistently
3. **Validation Logic**: All validation requirements are properly enforced
4. **Integration**: Validation integrates seamlessly with existing infrastructure
5. **Performance**: Validation runs efficiently without performance degradation
6. **User Experience**: Clear error messages guide users to fix validation issues

## Files Modified

- `src/schema/schema_interpretation.rs` - Add validation during schema interpretation
- `src/schema/types/transform.rs` - Add validation for declarative transform creation
- `src/schema/indexing/field_alignment.rs` - Integration with existing validation logic
- `src/schema/indexing/errors.rs` - Use existing error types for validation
- `tests/unit/schema/validation_tests.rs` - Add validation tests
- `tests/integration/validation_integration_tests.rs` - Add integration tests

## Test Plan

### Objective
Verify that declarative transform validation works correctly using existing iterator stack infrastructure and provides clear feedback for validation failures.

### Test Scope
- Field alignment validation using existing infrastructure
- Error type integration and consistency
- Schema validation for declarative transforms
- Iterator expression validation
- Integration with existing validation components

### Environment & Setup
- Standard Rust test environment
- Existing iterator stack infrastructure
- Existing validation components
- Test data fixtures for various validation scenarios

### Mocking Strategy
- Mock external dependencies as needed
- Use existing validation components for integration testing
- Create test fixtures for various validation scenarios
- Test both valid and invalid declarative transform configurations

### Key Test Scenarios
1. **Valid Declarative Transforms**: Test that valid transforms pass all validation
2. **Invalid Schema Types**: Test validation failures for invalid schema types
3. **Missing Required Fields**: Test validation for missing hash/range fields in HashRange schemas
4. **Invalid Iterator Expressions**: Test validation for malformed iterator expressions
5. **Field Alignment Issues**: Test validation for incompatible field depths and branches
6. **Error Type Consistency**: Test that validation uses existing error types correctly
7. **Performance Testing**: Test validation performance with large schema definitions

### Success Criteria
- All validation tests pass
- Field alignment validation works correctly for declarative transforms
- Error types are consistent with existing system
- Validation provides clear, actionable error messages
- Performance is acceptable for large schema definitions
- Integration with existing infrastructure is seamless
- Validation catches all invalid configurations
