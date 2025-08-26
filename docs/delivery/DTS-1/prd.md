# PBI-DTS-1: Core Declarative Transform Data Structures

[View in Backlog](../backlog.md#user-content-dts-1)

## Overview

This PBI implements the foundational data structures needed to support both procedural and declarative transform types in the DataFold system. It extends the existing transform system to handle declarative schema definitions while maintaining backward compatibility with procedural transforms. The declarative transforms will seamlessly integrate with the existing transform queue and orchestration system.

## Problem Statement

Currently, the DataFold transform system only supports procedural transforms written in a custom DSL. Users want to define transforms declaratively using JSON schema definitions that automatically generate and maintain data structures. The system needs to support both transform types seamlessly without breaking existing functionality, with declarative transforms automatically queued and executed through the existing transform infrastructure.

## User Stories

- **As a developer**, I want to define transforms using declarative JSON schema definitions instead of writing procedural DSL code
- **As a developer**, I want the system to automatically generate the underlying procedural logic needed to execute declarative transforms
- **As a developer**, I want both procedural and declarative transforms to coexist in the same system without conflicts
- **As a developer**, I want the existing transform system to continue working unchanged while new declarative capabilities are added
- **As a developer**, I want declarative transforms to automatically queue and execute when source data changes
- **As a developer**, I want declarative transforms to use the same queue, orchestration, and execution system as procedural transforms

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

Create the core structure for declarative transforms that matches the schema-generation-transforms specification:

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

Modify the existing `JsonTransform` struct to support both transform types using the exact structure from schema-generation-transforms:

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

### 4. Integration with Existing Transform System

Ensure declarative transforms integrate seamlessly with the existing transform infrastructure:

#### **4.1 Transform Registration**
- Declarative transforms are stored in the same transform registry as procedural transforms
- Each declarative transform gets a unique transform ID (e.g., `"blogs_by_word.declarative"`)
- Field-to-transform mappings are created for automatic triggering

#### **4.2 Queue Integration**
- Declarative transforms use the exact same `QueueItem` structure as procedural transforms
- Transforms are automatically added to the TransformOrchestrator's execution queue when source data changes
- Same queuing system handles both procedural and declarative transforms seamlessly

#### **4.3 Execution Flow**
- TransformOrchestrator processes declarative transforms through the same execution flow
- ExecutionCoordinator automatically handles both transform types
- Same monitoring, queuing, execution, and result handling for all transforms

#### **4.4 Iterator Stack Integration**
- **Leverage existing infrastructure**: Use proven `IteratorStack`, `ChainParser`, and `ExecutionEngine` components
- **Parse declarative expressions**: Convert expressions like `"blogpost.map().content.split_by_word().map()"` to existing `ParsedChain` format
- **Execute through existing pipeline**: Use `IteratorStack::from_chain()` and `ExecutionEngine::execute_chains()` methods
- **Maintain performance**: Leverage existing optimizations, caching, and field alignment validation
- **Ensure consistency**: Use same validation logic and error handling as existing procedural transforms

### 5. Maintain Backward Compatibility

Ensure existing procedural transforms continue to work by:
- Providing default values for new fields
- Maintaining the same serialization format for procedural transforms
- Adding migration logic if needed
- Existing procedural transforms continue to use the same execution path unchanged

## UX/UI Considerations

This PBI is focused on backend data structures and doesn't require UI changes. However, the implementation should consider:

- Clear error messages when declarative transforms are malformed
- Validation feedback for schema definition syntax
- Documentation examples for both transform types
- Logging that shows transform type information for debugging

## Acceptance Criteria

1. **TransformKind Enum**: `TransformKind` enum implemented with `Procedural` and `Declarative` variants
2. **DeclarativeSchemaDefinition**: Complete struct with all required fields and proper serialization, matching schema-generation-transforms specification
3. **KeyConfig and FieldDefinition**: Supporting structures for HashRange schemas and field mappings
4. **JsonTransform Updates**: Modified to support both transform types via `TransformKind` using the exact structure from schema-generation-transforms
5. **Backward Compatibility**: Existing procedural transforms continue to work unchanged
6. **Serialization Tests**: Comprehensive tests verify both transform types serialize/deserialize correctly
7. **Validation**: Basic validation ensures declarative transforms have required fields
8. **Documentation**: Clear examples and usage patterns documented
9. **Integration**: Declarative transforms can be registered and stored in the existing transform registry
10. **Queue Compatibility**: Declarative transforms use the same QueueItem structure as procedural transforms
11. **Iterator Stack Integration**: Declarative transforms properly use existing iterator stack infrastructure
12. **Field Alignment Validation**: Existing field alignment validation works for declarative transforms
13. **Execution Engine Integration**: Declarative transforms execute through existing execution engine

## Key Integration Points with Iterator Stack Infrastructure

The implementation must ensure that declarative transforms integrate with:

1. **Transform Registry**: Can be stored and retrieved like procedural transforms
2. **Queue System**: Use the same QueueItem structure and queuing mechanisms
3. **Orchestration**: Are processed by the same TransformOrchestrator
4. **Execution**: Can be executed through the existing execution pipeline
5. **Monitoring**: Are monitored and queued automatically when source fields change
6. **Iterator Stack**: Use existing `IteratorStack` for scope management and execution
7. **Chain Parser**: Leverage existing `ChainParser` for parsing declarative expressions
8. **Execution Engine**: Integrate with existing `ExecutionEngine` for runtime processing
9. **Field Alignment**: Use existing field alignment validation for iterator expressions
10. **Error Handling**: Leverage existing error types and validation logic

### Iterator Stack Integration Benefits

By leveraging the existing iterator stack infrastructure, declarative transforms gain:

- **Proven Performance**: Access to existing optimizations, caching, and streaming
- **Rich Operations**: Support for Map, Split, Filter, Sort, Limit, Offset operations
- **Consistent Behavior**: Same field alignment and validation logic as procedural transforms
- **Maintainability**: Reuse of tested, well-documented components
- **Extensibility**: Easy to add new iterator operations as needed
- **Testing**: Leverage existing test coverage and validation

## Dependencies

- Existing transform system architecture
- Current `JsonTransform` and `Transform` types
- Serde serialization framework
- Existing transform queue and orchestration system
- **Iterator Stack Infrastructure**: `src/schema/indexing/` components (IteratorStack, ChainParser, ExecutionEngine)
- **Field Alignment System**: Existing validation and error handling components

## Open Questions

1. Should we add validation for declarative schema definitions at the data structure level?
2. Do we need to support additional schema types beyond "Single" and "HashRange"?
3. Should field type inference be handled at the data structure level or during parsing?
4. How should we handle the automatic transform ID generation for declarative transforms?

## Related Tasks

- [DTS-2: Declarative Transform Parser](./DTS-2/prd.md)
- [DTS-3: Declarative Transform Manager](./DTS-3/prd.md)
- [DTS-4: Declarative Transform Compiler](./DTS-4/prd.md)

## Implementation Notes

### File Locations

- **Core Types**: `src/schema/types/json_schema.rs`
- **Transform Types**: `src/schema/types/transform.rs`
- **Iterator Stack Integration**: `src/schema/indexing/` components
- **Tests**: `tests/unit/schema/declarative_transforms.rs`

### Migration Strategy

1. Add new fields with default values to maintain backward compatibility
2. Implement new serialization logic for declarative transforms
3. Add validation for declarative transform structures using existing iterator stack validation
4. Update tests to cover both transform types and iterator stack integration
5. Ensure declarative transforms can be stored in the existing transform registry
6. **Integrate with existing iterator stack infrastructure** for execution and validation

### Testing Strategy

- Unit tests for each new data structure
- Serialization/deserialization tests for both transform types
- Validation tests for declarative transform requirements using existing field alignment logic
- Backward compatibility tests for existing procedural transforms
- Integration tests to verify declarative transforms can be registered and stored
- **Iterator stack integration tests** to verify proper use of existing infrastructure
- **Field alignment validation tests** using existing validation components
- **Performance tests** to ensure no degradation from iterator stack integration

### Key Integration Points

The implementation must ensure that declarative transforms integrate with:

1. **Transform Registry**: Can be stored and retrieved like procedural transforms
2. **Queue System**: Use the same QueueItem structure and queuing mechanisms
3. **Orchestration**: Are processed by the same TransformOrchestrator
4. **Execution**: Can be executed through the existing execution pipeline
5. **Monitoring**: Are monitored and queued automatically when source fields change

### Example Usage

The system should support declarative transforms like this:

```json
{
  "name": "blogs_by_word",
  "schema_type": "HashRange",
  "key": {
      "hash_field": "blogpost.map().content.split_by_word().map()",
      "range_field": "blogpost.map().publish_date"
  },
  "fields": {
    "blog": { "atom_uuid": "blogpost.map().$atom_uuid" },
    "author": { "atom_uuid": "blogpost.map().author.$atom_uuid" }
  }
}
```

This declarative definition should automatically become a transform that:
- Gets registered with the existing transform registry
- Uses the same queue and orchestration system
- Is automatically queued when source data changes
- **Executes through the existing iterator stack infrastructure**:
  - Parses expressions using existing `ChainParser`
  - Creates `IteratorStack` with proper scope management
  - Executes through existing `ExecutionEngine` with field alignment validation
  - Leverages existing performance optimizations and caching
