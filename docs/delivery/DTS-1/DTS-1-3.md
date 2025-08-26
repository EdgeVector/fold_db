# DTS-1-3 Update JsonTransform to support both transform types

[Back to task list](./tasks.md)

## Description

Update the existing `JsonTransform` struct to support both procedural and declarative transform types using the new `TransformKind` enum. This change must maintain backward compatibility with existing procedural transforms while adding support for declarative transforms.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 12:00:00 | Status Change | N/A | Proposed | Task file created | User |

## Requirements

1. **Backward Compatibility**: Existing procedural transforms must continue to work unchanged
2. **TransformKind Integration**: Use the new `TransformKind` enum to support both transform types
3. **Serialization**: Maintain proper JSON serialization for both transform types
4. **Deserialization**: Support deserializing both procedural and declarative transforms
5. **Field Mapping**: Preserve existing `inputs` and `output` field functionality

## Implementation Plan

### Step 1: Update JsonTransform Struct
- Replace the `logic: String` field with `#[serde(flatten)] pub kind: TransformKind`
- Keep `inputs` and `output` fields unchanged
- Add proper serde attributes for backward compatibility

### Step 2: Implement Backward Compatibility
- Use serde's `#[serde(flatten)]` to maintain existing JSON format for procedural transforms
- Ensure existing procedural transforms deserialize correctly
- Add migration logic if needed for existing transform definitions

### Step 3: Update Serialization Logic
- Verify procedural transforms serialize to the same format as before
- Ensure declarative transforms serialize with proper "kind" tag
- Test both serialization paths

### Step 4: Update Deserialization Logic
- Support deserializing existing procedural transform format
- Support deserializing new declarative transform format
- Provide clear error messages for invalid formats

## Verification

1. **Backward Compatibility**: Existing procedural transforms work unchanged
2. **New Functionality**: Declarative transforms can be defined and serialized
3. **Serialization**: Both transform types serialize to proper JSON format
4. **Deserialization**: Both transform types can be deserialized from JSON
5. **Error Handling**: Clear error messages for invalid transform definitions

## Files Modified

- `src/schema/types/json_schema.rs` - Update JsonTransform struct
- `tests/unit/schema/json_transform_tests.rs` - Update tests for new functionality
- `tests/integration/backward_compatibility_tests.rs` - Add backward compatibility tests

## Test Plan

### Objective
Verify that JsonTransform properly supports both procedural and declarative transform types while maintaining backward compatibility with existing procedural transforms.

### Test Scope
- JsonTransform struct updates and new TransformKind integration
- Backward compatibility for existing procedural transforms
- New declarative transform support
- Serialization/deserialization for both transform types

### Environment & Setup
- Standard Rust test environment
- Serde test utilities
- Existing transform test data

### Mocking Strategy
- No external dependencies to mock
- Use existing procedural transform examples for backward compatibility tests
- Use new declarative transform examples for new functionality tests

### Key Test Scenarios
1. **Backward Compatibility**: Verify existing procedural transforms work unchanged
2. **Procedural Transform Serialization**: Verify procedural transforms serialize to expected format
3. **Declarative Transform Serialization**: Verify declarative transforms serialize with proper "kind" tag
4. **Mixed Transform Support**: Verify both transform types can coexist in the same system
5. **Error Handling**: Verify clear error messages for invalid transform definitions
6. **JSON Round-trip**: Verify both transform types can be serialized and deserialized correctly

### Success Criteria
- All existing tests continue to pass
- New tests for declarative transforms pass
- Backward compatibility tests pass
- Both transform types serialize/deserialize correctly
- No compilation errors or warnings
- Clear error messages for invalid configurations
