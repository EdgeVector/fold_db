# [DTS-1-7C2] Field Alignment Validation Integration

[Back to task list](./tasks.md)

## Description

Integrate with the existing `FieldAlignmentValidator` to validate field alignment for declarative transform expressions. This task focuses on ensuring that declarative transform field expressions are compatible with the existing field alignment rules.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 18:00:00 | Created | N/A | Proposed | Task file created | AI Agent |
| 2025-01-27 22:30:00 | Status Update | Proposed | InProgress | Started FieldAlignmentValidator integration | AI Agent |
| 2025-01-27 23:15:00 | Status Update | InProgress | Done | Field alignment validation integration completed and tested | AI Agent |

## Requirements

1. **Field Alignment Validation**: Use existing `FieldAlignmentValidator` for declarative transforms
2. **Validation Rules**: Apply existing field alignment rules (1:1, broadcast, reduced)
3. **Error Handling**: Handle validation failures with clear error messages
4. **Integration**: Integrate validation into declarative transform processing
5. **No Execution**: Defer actual execution to later tasks

## Dependencies

- **DTS-1-7C1**: Basic Chain Parser Integration (must be completed first)
- **DTS-1-7A**: Basic Transform Type Routing (must be completed first)
- **DTS-1-7B**: Simple Declarative Transform Execution (must be completed first)
- **DTS-1-6**: Schema Interpreter (for parsing declarative transforms)
- **DTS-1-1**: TransformKind enum (for transform type detection)
- **DTS-1-2**: DeclarativeSchemaDefinition (for schema structure)

## Implementation Plan

### Step 1: Import Field Alignment Validator
- **Import existing `FieldAlignmentValidator`** from `src/schema/indexing/field_alignment.rs`
- **Create basic instance** using `FieldAlignmentValidator::new()`
- **Add to transform executor** for declarative transform validation

### Step 2: Basic Field Alignment Validation
- **Validate single field expressions** using existing validation logic
- **Apply existing alignment rules** (1:1, broadcast, reduced) to declarative expressions
- **Check depth compatibility** using existing depth validation
- **Validate branch compatibility** using existing branch logic

### Step 3: Validation Error Handling
- **Map validation errors** to appropriate error types
- **Provide clear error messages** for alignment failures
- **Handle common validation issues** (depth mismatches, incompatible operations)
- **Ensure validation failures don't crash** the transform system

### Step 4: Integration with Chain Parser Results
- **Use parsed chains** from DTS-1-7C1 for validation
- **Validate field alignment** before attempting execution
- **Store validation results** for later use by execution components
- **Basic logging** of validation results and errors

## Verification

1. **Field Alignment Validation**: Declarative expressions pass field alignment validation
2. **Validation Rules**: Existing field alignment rules are properly applied
3. **Error Handling**: Validation errors are handled gracefully with clear messages
4. **Integration**: Field alignment validator integrates with transform executor
5. **No Execution**: No actual execution occurs (validation only)
6. **Chain Integration**: Works with parsed chains from DTS-1-7C1

## Files Modified

- `src/transform/executor.rs` - Added FieldAlignmentValidator integration with comprehensive validation logic
- `tests/unit/transform/field_alignment_validation_tests.rs` - Added comprehensive field alignment validation tests  
- `tests/unit/transform/mod.rs` - Added new test module inclusion
- `tests/unit/transform/chain_parser_integration_tests.rs` - Updated tests to handle new validation behavior

## Test Plan

### Objective
Verify that field alignment validation works correctly for declarative transforms using existing validation infrastructure.

### Test Scope
- Field alignment validation using existing FieldAlignmentValidator
- Validation error handling and error messages
- Integration with chain parser results
- No execution testing (validation only)

### Environment & Setup
- Standard Rust test environment
- Existing FieldAlignmentValidator component
- Existing ChainParser component (from DTS-1-7C1)
- Existing transform system components
- Completed DTS-1-7A, DTS-1-7B, and DTS-1-7C1

### Mocking Strategy
- Mock external dependencies as needed
- Use existing FieldAlignmentValidator component for testing
- Use existing ChainParser component for testing
- Use existing transform system components for testing
- Create test fixtures for validation scenarios

### Key Test Scenarios
1. **Valid Field Alignment**: Test that valid declarative expressions pass validation
2. **Invalid Field Alignment**: Test error handling for alignment failures
3. **Depth Mismatches**: Test validation of depth compatibility
4. **Branch Compatibility**: Test validation of branch compatibility
5. **Integration**: Test integration with chain parser results
6. **Error Messages**: Test that validation error messages are clear and helpful

### Success Criteria
- All field alignment validation tests pass
- Declarative expressions are properly validated using existing rules
- Validation errors are handled gracefully with clear messages
- Integration with chain parser results works correctly
- No execution occurs (validation only)
- No regression in existing functionality
