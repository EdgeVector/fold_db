# PBI-SSF-1: Simplified Schema Format Implementation

[View in Backlog](../backlog.md#user-content-SSF-1)

## Overview

This PBI implements simplified schema formats for both declarative transforms and regular schemas, enabling developers to write cleaner, more readable schema definitions with 90% less boilerplate while maintaining full backward compatibility.

## Problem Statement

Currently, schema definitions in FoldDB are verbose and contain significant boilerplate:

1. **Declarative Transform Schemas**: Require explicit `FieldDefinition` objects with `atom_uuid` properties even for simple field mappings
2. **Regular Schemas**: Contain extensive configuration for permissions, payment policies, and field types that often use default values
3. **Developer Experience**: The verbose format makes schemas harder to read, write, and maintain
4. **Schema Size**: Current schemas can be 99+ lines when simplified versions could be 16 lines

## User Stories

### Primary User Story
**As a developer**, I want simplified schema formats so I can write cleaner, more readable schema definitions with 90% less boilerplate while maintaining full backward compatibility.

### Supporting User Stories
- **As a developer**, I want to use string expressions directly in field definitions so I can avoid wrapping simple mappings in `FieldDefinition` objects
- **As a developer**, I want ultra-minimal schemas with empty field objects `{}` so I can define schemas with sensible defaults
- **As a developer**, I want mixed format support so I can gradually migrate existing schemas or combine formats within the same schema

## Technical Approach

### 1. JsonSchemaField Default Values Implementation

Add default values to the `JsonSchemaField` struct to support ultra-minimal schemas:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaField {
    #[serde(default = "default_permission_policy")]
    pub permission_policy: JsonPermissionPolicy,
    #[serde(default)]
    pub molecule_uuid: Option<String>,
    #[serde(default = "default_payment_config")]
    pub payment_config: JsonFieldPaymentConfig,
    #[serde(default)]
    pub field_mappers: HashMap<String, String>,
    #[serde(default = "default_field_type")]
    pub field_type: FieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<JsonTransform>,
}
```

### 2. Custom Deserialization Implementation

Implement custom deserialization to support both string expressions and `FieldDefinition` objects:

```rust
impl<'de> serde::Deserialize<'de> for DeclarativeSchemaDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Support both string expressions and FieldDefinition objects
        // Convert strings to FieldDefinition with atom_uuid
    }
}
```

## UX/UI Considerations

### Schema Definition Examples

**Before (Verbose)**:
```json
{
  "name": "BlogPostWordIndex",
  "schema_type": "HashRange",
  "key": {
    "hash_field": "BlogPost.map().content.split_by_word().map()",
    "range_field": "BlogPost.map().publish_date"
  },
  "fields": {
    "content": { "atom_uuid": "BlogPost.map().content" },
    "author": { "atom_uuid": "BlogPost.map().author" },
    "title": { "atom_uuid": "BlogPost.map().title" },
    "tags": { "atom_uuid": "BlogPost.map().tags" }
  }
}
```

**After (Simplified)**:
```json
{
  "name": "BlogPostWordIndex",
  "schema_type": "HashRange",
  "key": {
    "hash_field": "BlogPost.map().content.split_by_word().map()",
    "range_field": "BlogPost.map().publish_date"
  },
  "fields": {
    "content": "BlogPost.map().content",
    "author": "BlogPost.map().author", 
    "title": "BlogPost.map().title",
    "tags": "BlogPost.map().tags"
  }
}
```

**Ultra-Minimal Regular Schema**:
```json
{
  "name": "BlogPost",
  "schema_type": {
    "Range": {
      "range_key": "publish_date"
    }
  },
  "fields": {
    "title": {},
    "content": {},
    "author": {},
    "publish_date": {},
    "tags": {}
  }
}
```

## Acceptance Criteria

### Core Functionality
1. **JsonSchemaField Default Values**: All required fields have sensible defaults, enabling empty field objects `{}` ✅ **COMPLETED**
2. **Custom Deserialization**: Supports both string expressions and `FieldDefinition` objects in the same schema ✅ **COMPLETED**
3. **Backward Compatibility**: All existing schemas continue to work without modification ✅ **VERIFIED**
4. **Mixed Format Support**: Schemas can combine simplified and verbose formats within the same definition

### Validation Requirements
1. **Field Validation**: At least one of `atom_uuid` or `field_type` must be defined (satisfied by string conversion)
2. **Expression Validation**: String expressions must be valid iterator stack expressions
3. **Error Handling**: Clear error messages for invalid formats

### Testing Requirements
1. **Unit Tests**: Comprehensive tests for simplified format parsing
2. **Backward Compatibility Tests**: Verify existing schemas continue to work
3. **Mixed Format Tests**: Test schemas combining both formats
4. **Ultra-Minimal Tests**: Test schemas with empty field objects
5. **Error Handling Tests**: Test invalid expression formats

### Documentation Requirements
1. **Schema Documentation**: Updated with examples of both formats
2. **Migration Guide**: Document how to migrate from verbose to simplified format
3. **API Documentation**: Updated to reflect new deserialization capabilities

## Dependencies

### Technical Dependencies
- **JsonSchemaField struct**: Requires modification to add default values
- **Serde deserialization**: Requires custom implementation for mixed format support

### System Dependencies
- **Existing Transform System**: Must continue to work unchanged
- **Schema Validation**: Must work with both old and new formats
- **Documentation System**: Must be updated to reflect new capabilities

## Open Questions

1. **Performance Impact**: What is the performance overhead of custom deserialization?
2. **Migration Strategy**: Should we provide automated migration tools for existing schemas?
3. **Future Extensibility**: How should we handle additional transform types beyond "declarative"?
4. **Validation Complexity**: Are there edge cases in mixed format validation we need to consider?

## Related Tasks

This PBI will be broken down into the following tasks:

1. **SSF-1-1**: Add default values to JsonSchemaField struct ✅ **COMPLETED**
2. **SSF-1-2**: ~~Add transform_type field to DeclarativeSchemaDefinition~~ ❌ **CANCELLED**
3. **SSF-1-3**: Implement custom deserialization for mixed format support ✅ **COMPLETED**
4. **SSF-1-4**: Add comprehensive unit tests for simplified formats ✅ **COMPLETED**
5. **SSF-1-5**: Update documentation with new format examples ✅ **COMPLETED**
6. **SSF-1-6**: E2E CoS Test - Verify simplified schemas work end-to-end ✅ **COMPLETED**

## Benefits

1. **Improved Developer Experience**: 90% reduction in schema size and complexity
2. **Better Readability**: Cleaner, more intuitive schema definitions
3. **Backward Compatibility**: No breaking changes to existing schemas
4. **Consistency**: Follows existing patterns in the codebase
5. **Maintainability**: Easier to write and maintain schema definitions

## Risks and Mitigations

### Risk: Breaking Changes
**Mitigation**: Extensive testing with existing schemas and gradual rollout

### Risk: Validation Complexity
**Mitigation**: Reuse existing validation logic, only change deserialization

### Risk: Performance Impact
**Mitigation**: Minimal overhead - conversion happens once during deserialization

### Risk: Mixed Format Confusion
**Mitigation**: Clear documentation and examples of both formats
