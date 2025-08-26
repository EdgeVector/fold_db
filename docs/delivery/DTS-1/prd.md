# PBI-DTS-1: Core Declarative Transform Data Structures

[View in Backlog](../backlog.md#user-content-dts-1)

## Overview

This PBI implements the foundational data structures needed to support both procedural and declarative transform types in the DataFold system. It extends the existing transform system to handle declarative schema definitions while maintaining backward compatibility with procedural transforms.

## Problem Statement

Currently, the DataFold transform system only supports procedural transforms written in a custom DSL. Users want to define transforms declaratively using JSON schema definitions that automatically generate and maintain data structures. The system needs to support both transform types seamlessly without breaking existing functionality.

## User Stories

- **As a developer**, I want to define transforms using declarative JSON schema definitions instead of writing procedural DSL code
- **As a developer**, I want the system to automatically generate the underlying procedural logic needed to execute declarative transforms
- **As a developer**, I want both procedural and declarative transforms to coexist in the same system without conflicts
- **As a developer**, I want the existing transform system to continue working unchanged while new declarative capabilities are added

## Technical Approach

### 1. Extend Transform Type System

Create a new `TransformKind` enum that supports both procedural and declarative transforms:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransformKind {
    Procedural { logic: String },
    Declarative { schema: DeclarativeSchemaDefinition },
}
```

### 2. Implement Declarative Schema Definition

Create the core structure for declarative transforms:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeSchemaDefinition {
    /// Schema name (same as transform name)
    pub name: String,
    /// Schema type ("Single" | "HashRange")
    pub schema_type: String,
    /// Key configuration (required when schema_type == "HashRange")
    pub key: Option<KeyConfig>,
    /// Field definitions with their mapping expressions
    pub fields: std::collections::HashMap<String, FieldDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyConfig {
    /// Hash field expression for the key
    pub hash_field: String,
    /// Range field expression for the key
    pub range_field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Atom UUID field expression (for reference fields)
    pub atom_uuid: Option<String>,
    /// Field type (inferred from context)
    pub field_type: Option<String>,
}
```

### 3. Update JsonTransform Structure

Modify the existing `JsonTransform` struct to support both transform types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonTransform {
    /// Explicit list of input fields in `Schema.field` format
    #[serde(default)]
    pub inputs: Vec<String>,

    /// Output field for this transform in `Schema.field` format
    pub output: String,

    /// Transform kind: either procedural DSL logic or a declarative schema
    #[serde(flatten)]
    pub kind: TransformKind,
}
```

### 4. Maintain Backward Compatibility

Ensure existing procedural transforms continue to work by:
- Providing default values for new fields
- Maintaining the same serialization format for procedural transforms
- Adding migration logic if needed

## UX/UI Considerations

This PBI is focused on backend data structures and doesn't require UI changes. However, the implementation should consider:

- Clear error messages when declarative transforms are malformed
- Validation feedback for schema definition syntax
- Documentation examples for both transform types

## Acceptance Criteria

1. **TransformKind Enum**: `TransformKind` enum implemented with `Procedural` and `Declarative` variants
2. **DeclarativeSchemaDefinition**: Complete struct with all required fields and proper serialization
3. **KeyConfig and FieldDefinition**: Supporting structures for HashRange schemas and field mappings
4. **JsonTransform Updates**: Modified to support both transform types via `TransformKind`
5. **Backward Compatibility**: Existing procedural transforms continue to work unchanged
6. **Serialization Tests**: Comprehensive tests verify both transform types serialize/deserialize correctly
7. **Validation**: Basic validation ensures declarative transforms have required fields
8. **Documentation**: Clear examples and usage patterns documented

## Dependencies

- Existing transform system architecture
- Current `JsonTransform` and `Transform` types
- Serde serialization framework

## Open Questions

1. Should we add validation for declarative schema definitions at the data structure level?
2. Do we need to support additional schema types beyond "Single" and "HashRange"?
3. Should field type inference be handled at the data structure level or during parsing?

## Related Tasks

- [DTS-2: Declarative Transform Parser](./DTS-2/prd.md)
- [DTS-3: Declarative Transform Manager](./DTS-3/prd.md)
- [DTS-4: Declarative Transform Compiler](./DTS-4/prd.md)

## Implementation Notes

### File Locations

- **Core Types**: `src/schema/types/json_schema.rs`
- **Transform Types**: `src/schema/types/transform.rs`
- **Tests**: `tests/unit/schema/declarative_transforms.rs`

### Migration Strategy

1. Add new fields with default values to maintain backward compatibility
2. Implement new serialization logic for declarative transforms
3. Add validation for declarative transform structures
4. Update tests to cover both transform types

### Testing Strategy

- Unit tests for each new data structure
- Serialization/deserialization tests for both transform types
- Validation tests for declarative transform requirements
- Backward compatibility tests for existing procedural transforms
