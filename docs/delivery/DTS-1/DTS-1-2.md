# DTS-1-2 Implement DeclarativeSchemaDefinition and supporting structs

[Back to task list](./tasks.md)

## Description

Implement the core data structures for declarative transforms including `DeclarativeSchemaDefinition`, `KeyConfig`, and `FieldDefinition`. These structs will define the structure and configuration for declarative transforms that automatically generate and maintain data structures.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 12:00:00 | Status Change | N/A | Proposed | Task file created | User |

## Requirements

1. **DeclarativeSchemaDefinition**: Main struct containing schema metadata and field definitions
2. **KeyConfig**: Configuration for HashRange schemas with hash and range field expressions
3. **FieldDefinition**: Individual field definitions with atom UUID mappings and type information
4. **Serialization Support**: Proper serde serialization/deserialization for all structs
5. **Validation**: Basic validation to ensure required fields are present
6. **SchemaType Extension**: Extend existing SchemaType enum to include HashRange variant

## Implementation Plan

### Step 1: Extend SchemaType Enum
- Add `HashRange` variant to existing `SchemaType` enum in `src/schema/types/schema.rs`
- Ensure backward compatibility with existing Single and Range variants
- Add proper documentation for the new variant

### Step 2: Implement KeyConfig Struct
- Create struct with `hash_field` and `range_field` String fields
- Add proper serde attributes for JSON serialization
- Include comprehensive documentation

### Step 3: Implement FieldDefinition Struct
- Create struct with optional `atom_uuid` and `field_type` fields
- Use `Option<String>` for optional fields
- Add proper serde attributes

### Step 4: Implement DeclarativeSchemaDefinition Struct
- Create main struct with required fields: `name`, `schema_type`, `key`, `fields`
- Use `Option<KeyConfig>` for the key field (required for HashRange schemas)
- Use `HashMap<String, FieldDefinition>` for fields
- Add proper serde attributes

### Step 5: Add Validation Logic
- Implement basic validation methods
- Ensure HashRange schemas have key configuration
- Validate field definitions have required information

## Verification

1. **Compilation**: All structs compile without errors
2. **Serialization**: All structs serialize to proper JSON format
3. **Deserialization**: All structs can be deserialized from JSON
4. **Validation**: Basic validation logic works correctly
5. **Documentation**: Clear documentation explains usage and requirements

## Files Modified

- `src/schema/types/schema.rs` - Extend SchemaType enum with HashRange variant
- `src/schema/types/json_schema.rs` - Add new structs
- `tests/unit/schema/declarative_schema_tests.rs` - Add unit tests for new structs

## Test Plan

### Objective
Verify that the DeclarativeSchemaDefinition and supporting structs properly define declarative transform schemas with correct serialization/deserialization and validation.

### Test Scope
- SchemaType enum extension with HashRange variant
- KeyConfig struct definition and serialization
- FieldDefinition struct definition and serialization
- DeclarativeSchemaDefinition struct definition and serialization
- Validation logic for required fields and configurations

### Environment & Setup
- Standard Rust test environment
- Serde test utilities

### Mocking Strategy
- No external dependencies to mock
- Use simple string values for testing

### Key Test Scenarios
1. **SchemaType Extension**: Verify HashRange variant is properly added and serializes correctly
2. **KeyConfig Serialization**: Verify KeyConfig serializes to correct JSON format
3. **FieldDefinition Serialization**: Verify FieldDefinition serializes to correct JSON format
4. **DeclarativeSchemaDefinition Serialization**: Verify main struct serializes to correct JSON format
5. **HashRange Validation**: Verify HashRange schemas require key configuration
6. **Field Validation**: Verify field definitions are properly validated
7. **JSON Round-trip**: Verify all structs can be serialized and deserialized correctly

### Success Criteria
- All tests pass
- All structs serialize/deserialize correctly
- Validation logic works as expected
- No compilation errors or warnings
- Clear error messages for invalid configurations
