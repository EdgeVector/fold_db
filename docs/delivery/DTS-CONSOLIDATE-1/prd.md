# PBI DTS-CONSOLIDATE-1: Consolidate Transform Executor Modules

[View in Backlog](../backlog.md#user-content-dts-consolidate-1)

## Overview

This PBI consolidates the three separate executor modules (`single_executor.rs`, `range_executor.rs`, `hash_range_executor.rs`) into a unified `executor.rs` module. This eliminates 50%+ code duplication and simplifies the transform execution architecture while preserving all existing functionality.

## Problem Statement

The declarative transform system currently has three separate executor modules that follow nearly identical execution patterns:

- **`single_executor.rs`** - 219 lines, handles Single schema execution
- **`range_executor.rs`** - 153 lines, handles Range schema execution  
- **`hash_range_executor.rs`** - 124 lines, handles HashRange schema execution

**Total: 496 lines with ~70% duplication**

### Issues with Current Architecture

1. **Massive Code Duplication**: All three executors follow the same 8-step pattern:
   - `log_schema_execution_start()`
   - Schema validation
   - Expression collection
   - Batch parsing
   - Field alignment validation
   - ExecutionEngine setup
   - Execution
   - Result aggregation

2. **Complex Call Chains**: Unnecessary indirection through separate modules for similar logic

3. **Maintenance Burden**: Changes to execution logic must be made in 3+ places

4. **Architectural Complexity**: Multiple execution paths for essentially the same process

5. **Over-Engineering**: Separate timing structs and validation functions for similar operations

## User Stories

- **As a developer**, I want a unified executor pattern so I can maintain execution logic in one place
- **As a developer**, I want to eliminate code duplication so I can reduce the codebase by 50%+
- **As a developer**, I want simplified architecture so I can understand and modify transform execution more easily
- **As a developer**, I want to preserve all existing functionality so I don't break existing transform behavior
- **As a developer**, I want unified execution patterns so I can add new schema types more easily

## Technical Approach

### 1. Unified Execution Pattern

Replace the three separate executors with a single unified execution pattern in `executor.rs`:

```rust
impl TransformExecutor {
    /// Unified execution method that handles all schema types
    fn execute_declarative_transform_unified(
        schema: &DeclarativeSchemaDefinition,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        // Single execution path with schema-type branching
        match &schema.schema_type {
            SchemaType::Single => self.execute_single_pattern(schema, input_values),
            SchemaType::Range { range_key } => self.execute_range_pattern(schema, input_values, range_key),
            SchemaType::HashRange => self.execute_hashrange_pattern(schema, input_values),
        }
    }
}
```

### 2. Consolidate Common Logic

Extract the common 8-step execution pattern into shared methods:

```rust
impl TransformExecutor {
    /// Common execution pattern used by all schema types
    fn execute_with_common_pattern<F>(
        schema: &DeclarativeSchemaDefinition,
        input_values: &HashMap<String, JsonValue>,
        schema_type_name: &str,
        custom_logic: F,
    ) -> Result<JsonValue, SchemaError>
    where
        F: FnOnce(&DeclarativeSchemaDefinition, &HashMap<String, JsonValue>) -> Result<JsonValue, SchemaError>,
    {
        // 1. Log execution start
        // 2. Validate schema
        // 3. Collect expressions
        // 4. Parse expressions
        // 5. Validate alignment
        // 6. Execute with custom logic
        // 7. Aggregate results
    }
}
```

### 3. Schema-Specific Logic

Implement schema-specific logic as focused methods:

```rust
impl TransformExecutor {
    fn execute_single_pattern(&self, schema: &DeclarativeSchemaDefinition, input_values: &HashMap<String, JsonValue>) -> Result<JsonValue, SchemaError>
    fn execute_range_pattern(&self, schema: &DeclarativeSchemaDefinition, input_values: &HashMap<String, JsonValue>, range_key: &str) -> Result<JsonValue, SchemaError>
    fn execute_hashrange_pattern(&self, schema: &DeclarativeSchemaDefinition, input_values: &HashMap<String, JsonValue>) -> Result<JsonValue, SchemaError>
}
```

### 4. File Structure Changes

```
src/transform/
├── executor.rs (consolidated - ~300 lines, down from 496)
├── single_executor.rs (DELETED)
├── range_executor.rs (DELETED)
└── hash_range_executor.rs (DELETED)
```

## UX/UI Considerations

This PBI is focused on backend consolidation and doesn't require UI changes. The implementation should consider:

- No impact on existing transform execution functionality
- Simplified architecture for future development
- Easier maintenance and debugging of transform execution
- Preserved performance characteristics

## Acceptance Criteria

1. **Module Consolidation**: All three executor modules are merged into `executor.rs`
2. **Code Reduction**: Total lines reduced by 50%+ (from 496 to ~300 lines)
3. **Functionality Preservation**: All existing transform execution functionality works identically
4. **Clean Compilation**: Code compiles successfully after consolidation
5. **Test Coverage**: All existing tests pass without modification
6. **Unified Pattern**: Single execution pattern handles all schema types
7. **Simplified Architecture**: Reduced complexity and indirection in execution paths
8. **Performance**: No performance degradation in transform execution

## Dependencies

- Existing TransformExecutor implementation
- Current transform system architecture
- Shared utilities and coordination modules
- All existing tests and validation

## Open Questions

1. Should the unified executor maintain the same public API surface?
2. Are there any performance considerations for the unified execution pattern?
3. Should we preserve the existing timing and monitoring capabilities?

## Related Tasks

- [DTS-CONSOLIDATE-1-1: Analyze current executor patterns and create consolidation plan](./DTS-CONSOLIDATE-1-1.md)
- [DTS-CONSOLIDATE-1-2: Implement unified execution pattern in executor.rs](./DTS-CONSOLIDATE-1-2.md)
- [DTS-CONSOLIDATE-1-3: Delete separate executor modules and update imports](./DTS-CONSOLIDATE-1-3.md)
- [DTS-CONSOLIDATE-1-4: Update tests and verify functionality preservation](./DTS-CONSOLIDATE-1-4.md)

## Implementation Notes

### Files to Modify

- `src/transform/executor.rs` - Add unified execution pattern
- `src/transform/mod.rs` - Remove module declarations for deleted executors

### Files to Delete

- `src/transform/single_executor.rs` - Merge into executor.rs
- `src/transform/range_executor.rs` - Merge into executor.rs  
- `src/transform/hash_range_executor.rs` - Merge into executor.rs

### Key Benefits

- **50%+ Code Reduction**: From 496 lines to ~300 lines
- **Eliminated Duplication**: Single source of truth for execution logic
- **Simplified Maintenance**: Changes only need to be made in one place
- **Improved Architecture**: Clear, unified execution pattern
- **Preserved Functionality**: All existing behavior maintained
- **Better Testability**: Single execution path to test and validate
