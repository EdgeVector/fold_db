# PBI-NTS-2: Native Schema Registry

[View in Backlog](../backlog.md#user-content-NTS-2)

## Overview

This PBI implements a native schema registry that manages schemas using compile-time type safety instead of runtime JSON validation. This eliminates the need for JSON parsing in schema operations and provides better performance and reliability.

## Problem Statement

The current schema system relies on JSON parsing and validation at runtime, which creates several issues:

1. **Runtime Validation**: Schema validation happens at runtime instead of compile-time
2. **JSON Parsing Overhead**: Every schema operation requires JSON parsing
3. **Type Safety Issues**: Schema fields are not type-safe at compile-time
4. **Complex Validation**: Complex JSON validation logic that's hard to maintain
5. **Performance Impact**: Schema operations are slower due to JSON overhead

## User Stories

- **As a developer**, I want native schema types so I can validate schemas at compile-time
- **As a developer**, I want a schema registry with native types so I can manage schemas efficiently
- **As a developer**, I want typed field definitions so I can catch schema errors during development
- **As a developer**, I want async schema operations so I can handle concurrent schema access
- **As a developer**, I want backward compatibility so existing schemas continue to work

## Technical Approach

### Native Schema Types

1. **NativeSchema Struct**: Native representation of schemas
   - Name, fields map, key configuration
   - Transform specifications
   - Validation methods

2. **KeyConfig Enum**: Key configuration for different schema types
   - `Single { key_field: String }`
   - `Range { hash_field: String, range_field: String }`
   - `HashRange { hash_field: String, range_field: String }`

3. **NativeSchemaRegistry**: Async schema management
   - Schema registration and retrieval
   - Field definition management
   - Concurrent access with RwLock

### Implementation Strategy

1. **Create Native Schema Module**: `src/schema/native/schema.rs`
2. **Implement Schema Registry**: `src/schema/native/registry.rs`
3. **Add Schema Validation**: Native type validation
4. **Async Operations**: Tokio-based async operations
5. **Backward Compatibility**: JSON schema conversion utilities

## UX/UI Considerations

- **API Compatibility**: Maintain existing schema APIs
- **Performance**: Faster schema operations
- **Error Handling**: Clear, typed error messages
- **Developer Experience**: Better IntelliSense for schema fields

## Acceptance Criteria

1. **NativeSchema struct with typed fields** implemented
2. **NativeSchemaRegistry with async operations** implemented
3. **Schema validation with native types** working correctly
4. **Field definition management** with type safety
5. **Comprehensive integration tests** verify schema operations
6. **Backward compatibility with existing schemas** maintained
7. **Performance improvements** measured and documented
8. **Async operations** handle concurrent access safely
9. **Error handling** provides clear, typed error messages
10. **Documentation** covers all schema operations

## Dependencies

- **NTS-1**: Native data types must be implemented first
- **tokio**: For async operations
- **serde**: For JSON boundary conversion
- **thiserror**: For typed error handling

## Open Questions

1. **Migration Strategy**: How to migrate existing JSON schemas?
2. **Performance Targets**: What specific performance improvements should we target?
3. **Concurrency**: How to handle concurrent schema modifications?
4. **Validation**: Should validation be strict or permissive?

## Related Tasks

- [NTS-2-1: Implement NativeSchema struct](./NTS-2-1.md)
- [NTS-2-2: Implement NativeSchemaRegistry](./NTS-2-2.md)
- [NTS-2-3: Add schema validation with native types](./NTS-2-3.md)
- [NTS-2-4: Add comprehensive integration tests](./NTS-2-4.md)
- [NTS-2-5: Implement backward compatibility](./NTS-2-5.md)
- [NTS-2-6: Add performance benchmarks](./NTS-2-6.md)
- [NTS-2-7: Update documentation](./NTS-2-7.md)
- [NTS-2-8: E2E CoS Test](./NTS-2-8.md)
