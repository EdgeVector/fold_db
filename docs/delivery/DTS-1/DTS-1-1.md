# DTS-1-1 Implement TransformKind enum with Procedural and Declarative variants

[Back to task list](./tasks.md)

## Description

Implement the `TransformKind` enum that will support both procedural and declarative transform types in the DataFold system. This enum will be the foundation for extending the existing transform system to handle declarative schema definitions.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 12:00:00 | Status Change | N/A | Proposed | Task file created | User |
| 2025-01-27 12:30:00 | Status Change | Proposed | In Progress | Started implementation | AI_Agent |
| 2025-01-27 13:30:00 | Status Change | In Progress | Done | TransformKind enum and tests implemented | AI_Agent |

## Requirements

1. **TransformKind Enum**: Create an enum with two variants:
   - `Procedural { logic: String }` - for existing DSL-based transforms
   - `Declarative { schema: DeclarativeSchemaDefinition }` - for new declarative transforms

2. **Serialization Support**: Implement proper serde serialization/deserialization with tag-based serialization

3. **Backward Compatibility**: Ensure existing procedural transforms continue to work unchanged

4. **Type Safety**: Use proper Rust types and derive necessary traits

## Implementation Plan

### Step 1: Create TransformKind Enum
- Add the enum to `src/schema/types/json_schema.rs`
- Use `#[serde(tag = "kind", rename_all = "snake_case")]` for proper JSON serialization
- Include both variants with appropriate field types

### Step 2: Add Required Imports
- Import necessary serde traits
- Import any required types for DeclarativeSchemaDefinition (placeholder for now)

### Step 3: Implement Serialization
- Ensure proper JSON output format for both variants
- Test serialization with both procedural and declarative examples

### Step 4: Add Documentation
- Add comprehensive doc comments explaining each variant
- Include usage examples in comments

## Verification

1. **Compilation**: Code compiles without errors
2. **Serialization**: Both variants serialize to proper JSON format
3. **Deserialization**: Both variants can be deserialized from JSON
4. **Type Safety**: Compile-time type checking works correctly
5. **Documentation**: Clear documentation explains usage

## Files Modified

- `src/schema/types/json_schema.rs` - Add TransformKind enum
- `tests/unit/schema/transform_kind_tests.rs` - Add unit tests for the enum

## Test Plan

### Objective
Verify that the TransformKind enum properly supports both procedural and declarative transform types with correct serialization/deserialization.

### Test Scope
- TransformKind enum definition and variants
- Serde serialization/deserialization
- JSON format validation

### Environment & Setup
- Standard Rust test environment
- Serde test utilities

### Mocking Strategy
- No external dependencies to mock
- Use simple string values for testing

### Key Test Scenarios
1. **Procedural Variant Serialization**: Verify procedural variant serializes to correct JSON format
2. **Declarative Variant Serialization**: Verify declarative variant serializes to correct JSON format  
3. **Deserialization**: Verify both variants can be deserialized from JSON
4. **Tag-based Serialization**: Verify the "kind" tag is properly included in JSON output
5. **Variant Discrimination**: Verify serde can correctly discriminate between variants

### Success Criteria
- All tests pass
- Both variants serialize/deserialize correctly
- JSON output matches expected format with "kind" tag
- No compilation errors or warnings
