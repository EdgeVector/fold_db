# PBI-NTS-1: Native Rust Data Types for Transforms

[View in Backlog](../backlog.md#user-content-NTS-1)

## Overview

This PBI implements the foundational native Rust data types that will replace JSON passing throughout the transform system. By eliminating JSON serialization/deserialization overhead and providing compile-time type safety, this establishes the core foundation for a high-performance, maintainable transform system.

## Problem Statement

The current transform system relies heavily on JSON (`JsonValue`) for data passing between components, which creates several critical issues:

1. **Performance Overhead**: Constant JSON serialization/deserialization in hot paths
2. **Type Safety Issues**: Runtime type validation instead of compile-time checking
3. **Complexity**: Multiple conversion layers between different JSON representations
4. **Debugging Difficulty**: Silent failures and complex error messages
5. **Memory Inefficiency**: Multiple copies of the same data in different formats

## User Stories

- **As a developer**, I want native Rust types for field values so I can eliminate JSON conversion overhead
- **As a developer**, I want compile-time type safety so I can catch errors during development instead of runtime
- **As a developer**, I want clear, typed data structures so I can debug transform issues more easily
- **As a developer**, I want consistent field definitions so I can validate data reliably
- **As a developer**, I want transform specifications with native types so I can execute transforms efficiently

## Technical Approach

### Core Data Types

1. **FieldValue Enum**: Native representation of field values
   - `String(String)`, `Number(f64)`, `Integer(i64)`, `Boolean(bool)`
   - `Array(Vec<FieldValue>)`, `Object(HashMap<String, FieldValue>)`, `Null`
   - Type-safe operations and conversions

2. **FieldType Enum**: Type definitions for validation
   - `String`, `Number`, `Integer`, `Boolean`
   - `Array(Box<FieldType>)`, `Object(HashMap<String, FieldType>)`
   - Compile-time type matching

3. **FieldDefinition Struct**: Schema field definitions
   - Name, type, required flag, default value
   - Validation methods with typed errors
   - Default value generation

4. **TransformSpec Struct**: Transform specifications
   - Input/output field definitions
   - Transform type (Map, Filter, Reduce, Chain)
   - Native type operations

### Implementation Strategy

1. **Create Core Types Module**: `src/transform/native/types.rs`
2. **Implement Field Definitions**: `src/transform/native/field_definition.rs`
3. **Add Transform Specifications**: `src/transform/native/transform_spec.rs`
4. **JSON Boundary Conversion**: Only at API boundaries
5. **Comprehensive Testing**: Unit tests for all type operations

## UX/UI Considerations

- **API Compatibility**: Maintain existing JSON APIs for external consumers
- **Error Messages**: Clear, typed error messages for better debugging
- **Performance**: Significant improvement in transform execution speed
- **Developer Experience**: Better IntelliSense and compile-time error detection

## Acceptance Criteria

1. **FieldValue and FieldType enums implemented** with comprehensive type safety
2. **FieldDefinition struct with validation** methods and error handling
3. **TransformSpec with native types** supporting all transform operations
4. **Comprehensive unit tests** verify type safety and performance
5. **JSON conversion only at API boundaries** - no internal JSON passing
6. **All existing functionality preserved** - backward compatibility maintained
7. **Performance benchmarks** show significant improvement over JSON-based system
8. **Type safety validation** catches errors at compile-time
9. **Clear error messages** for type mismatches and validation failures
10. **Documentation** for all new types and their usage

## Dependencies

- **Rust 1.70+**: For advanced enum features and serde support
- **serde**: For JSON boundary conversion only
- **thiserror**: For typed error handling
- **tokio**: For async operations in future phases

## Open Questions

1. **Migration Strategy**: How to gradually migrate existing JSON-based code?
2. **Performance Targets**: What specific performance improvements should we target?
3. **Error Handling**: Should we use custom error types or standard library errors?
4. **Testing Strategy**: What level of test coverage is required for type safety?

## Related Tasks

- [NTS-1-1: Implement FieldValue and FieldType enums](./NTS-1-1.md)
- [NTS-1-2: Implement FieldDefinition struct with validation](./NTS-1-2.md)
- [NTS-1-3: Implement TransformSpec with native types](./NTS-1-3.md)
- [NTS-1-4: Add comprehensive unit tests](./NTS-1-4.md)
- [NTS-1-5: Implement JSON boundary conversion utilities](./NTS-1-5.md)
- [NTS-1-6: Add performance benchmarks](./NTS-1-6.md)
- [NTS-1-7: Update documentation](./NTS-1-7.md)
- [NTS-1-8: E2E CoS Test](./NTS-1-8.md)
