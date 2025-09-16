# Simplified Schema Format Implementation

## Overview

This document outlines the implementation approach for supporting simplified schema formats across the FoldDB system, including both declarative transforms and regular schemas. This enables cleaner, more readable schema definitions while maintaining full backward compatibility.

## Current vs. Desired Format

### Current Format (Required)
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

### Desired Simplified Format
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

### 2. Regular Schema Simplification

#### Current Format (Verbose - 99 lines)
```json
{
  "name": "BlogPost",
  "schema_type": {
    "Range": {
      "range_key": "publish_date"
    }
  },
  "fields": {
    "title": {
      "permission_policy": {
        "read_policy": { "Distance": 0 },
        "write_policy": { "Distance": 1 }
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "trust_distance_scaling": "None",
        "min_payment": null
      },
      "field_type": "Range",
      "field_mappers": {},
      "transform": null,
      "writable": true
    },
    "content": {
      "permission_policy": {
        "read_policy": { "Distance": 0 },
        "write_policy": { "Distance": 1 }
      },
      "payment_config": {
        "base_multiplier": 2.0,
        "trust_distance_scaling": {
          "Linear": {
            "slope": 1.0,
            "intercept": 0.0,
            "min_factor": 1.0
          }
        },
        "min_payment": 10
      },
      "field_type": "Range",
      "field_mappers": {},
      "transform": null,
      "writable": true
    }
  },
  "payment_config": {
    "base_multiplier": 1.0,
    "min_payment_threshold": 0
  }
}
```

#### Simplified Format (Ultra-Minimal - 16 lines)
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

## Implementation Strategy

### 1. JsonSchemaField Default Values Implementation

To support ultra-minimal schemas with empty field objects `{}`, the `JsonSchemaField` struct needs default values for all required fields:

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

fn default_permission_policy() -> JsonPermissionPolicy {
    JsonPermissionPolicy {
        read_policy: TrustDistance::Distance(0),
        write_policy: TrustDistance::Distance(1),
    }
}

fn default_payment_config() -> JsonFieldPaymentConfig {
    JsonFieldPaymentConfig {
        base_multiplier: 1.0,
        trust_distance_scaling: TrustDistanceScaling::None,
        min_payment: None,
    }
}
```

#### Benefits of Default Values
- **Ultra-minimal schemas**: Fields can be empty objects `{}`
- **Backward compatibility**: Existing schemas continue to work
- **Sensible defaults**: System provides reasonable default configurations
- **Reduced boilerplate**: 90% reduction in schema size

### 2. Custom Deserialization for DeclarativeSchemaDefinition

The solution follows the existing pattern used in `Transform` and `JsonTransform` structs, which already support multiple formats through custom deserialization.

#### Implementation Approach

**File**: `src/schema/types/json_schema.rs`

```rust
impl<'de> serde::Deserialize<'de> for DeclarativeSchemaDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum FieldValue {
            // Simplified format: string expression
            String(String),
            // Current format: FieldDefinition object
            Object(FieldDefinition),
        }

        #[derive(serde::Deserialize)]
        struct Helper {
            name: String,
            schema_type: crate::schema::types::schema::SchemaType,
            #[serde(skip_serializing_if = "Option::is_none")]
            key: Option<KeyConfig>,
            fields: HashMap<String, FieldValue>,
        }

        let helper = Helper::deserialize(deserializer)?;
        
        // Convert simplified format to FieldDefinition format
        let mut field_definitions = HashMap::new();
        for (field_name, field_value) in helper.fields {
            let field_def = match field_value {
                FieldValue::String(expression) => {
                    // Convert string to FieldDefinition with atom_uuid
                    FieldDefinition {
                        atom_uuid: Some(expression),
                        field_type: None,
                    }
                }
                FieldValue::Object(field_def) => field_def,
            };
            field_definitions.insert(field_name, field_def);
        }

        Ok(DeclarativeSchemaDefinition {
            name: helper.name,
            schema_type: helper.schema_type,
            key: helper.key,
            fields: field_definitions,
        })
    }
}
```

### 2. Backward Compatibility

The implementation maintains full backward compatibility:

- **Existing schemas** with `FieldDefinition` objects continue to work unchanged
- **New schemas** can use the simplified string format
- **Mixed schemas** can combine both formats within the same schema

### 3. Validation Requirements

The simplified format must still meet all existing validation requirements:


#### FieldDefinition Validation Rules
- At least one of `atom_uuid` or `field_type` must be defined
- `atom_uuid` expressions must be valid iterator stack expressions
- Field expressions cannot start/end with dots or contain consecutive dots

#### Conversion Logic
When converting from string to `FieldDefinition`:
```rust
FieldDefinition {
    atom_uuid: Some(expression), // String becomes atom_uuid
    field_type: None,             // No field_type specified
}
```

This satisfies the validation requirement that "at least one property must be defined."

### 4. Error Handling

The deserialization will provide clear error messages for invalid formats:

```rust
// Invalid expression
"content": "BlogPost.map().content." // Error: cannot end with dot

// Invalid mixed format (if needed)
"content": { "atom_uuid": "BlogPost.map().content", "invalid_field": "value" }
// Error: unknown field 'invalid_field'
```

## Implementation Steps

### Step 1: Add Default Values to JsonSchemaField
- Add `#[serde(default = "...")]` attributes to required fields
- Implement default functions for `permission_policy` and `payment_config`
- Test with existing schemas to ensure no regression
- Verify ultra-minimal schemas work with empty field objects `{}`

### Step 2: Modify DeclarativeSchemaDefinition Deserialization
- Add custom `Deserialize` implementation
- Support both string expressions and `FieldDefinition` objects
- Convert string expressions to `FieldDefinition` with `atom_uuid`
- Test with existing schemas to ensure no regression

### Step 3: Update Tests
- Add tests for simplified format
- Add tests for mixed format support
- Add tests for backward compatibility
- Add tests for ultra-minimal schemas with empty field objects
- Verify backward compatibility

### Step 5: Documentation Updates
- Update schema documentation
- Add examples of both formats
- Document migration path (if needed)

## Testing Strategy

### Unit Tests Required

1. **Simplified Format Tests**
   ```rust
   #[test]
   fn test_declarative_schema_simplified_format() {
       let json = r#"
       {
         "name": "TestSchema",
         "schema_type": "HashRange",
         "key": {
           "hash_field": "Source.map().content",
           "range_field": "Source.map().date"
         },
         "fields": {
           "content": "Source.map().content",
           "author": "Source.map().author"
         }
       }
       "#;
       
       let schema: DeclarativeSchemaDefinition = serde_json::from_str(json).unwrap();
       // Verify conversion to FieldDefinition format
   }
   ```

2. **Mixed Format Tests**
   ```rust
   #[test]
   fn test_declarative_schema_mixed_format() {
       let json = r#"
       {
         "fields": {
           "content": "Source.map().content",           // Simplified
           "author": { "atom_uuid": "Source.map().author" } // Current
         }
       }
       "#;
       // Test both formats in same schema
   }
   ```

3. **Backward Compatibility Tests**
   ```rust
   #[test]
   fn test_declarative_schema_backward_compatibility() {
       // Test existing schemas still work
   }
   ```

4. **Ultra-Minimal Schema Tests**
   ```rust
   #[test]
   fn test_ultra_minimal_schema_with_empty_fields() {
       let json = r#"
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
           "author": {}
         }
       }
       "#;
       
       let schema: JsonSchemaDefinition = serde_json::from_str(json).unwrap();
       // Verify default values are applied
       assert_eq!(schema.fields.len(), 3);
       for field in schema.fields.values() {
           assert_eq!(field.field_type, FieldType::Single); // Default field type
           assert_eq!(field.payment_config.base_multiplier, 1.0); // Default payment config
           assert_eq!(field.permission_policy.read_policy, TrustDistance::Distance(0)); // Default permissions
       }
   }
   ```

### Integration Tests

- Test with actual transform execution
- Verify field resolution works correctly
- Test schema validation and error handling

## Benefits

1. **Improved Developer Experience**
   - Simpler, more readable schema definitions
   - Reduced boilerplate for common use cases (90% size reduction)
   - Easier to write and maintain
   - Explicit transform type indication
   - Ultra-minimal schemas with empty field objects `{}`

2. **Backward Compatibility**
   - No breaking changes to existing schemas
   - Gradual migration path available
   - Existing tooling continues to work

3. **Consistency**
   - Follows existing patterns in the codebase
   - Similar to how `Transform` handles multiple formats
   - Maintains validation and error handling

## Risks and Mitigations

### Risk: Breaking Changes
**Mitigation**: Extensive testing with existing schemas and gradual rollout

### Risk: Validation Complexity
**Mitigation**: Reuse existing validation logic, only change deserialization

### Risk: Performance Impact
**Mitigation**: Minimal overhead - conversion happens once during deserialization

## Migration Path

### For New Schemas
- Use simplified format immediately
- No migration required

### For Existing Schemas
- Continue using current format (no changes needed)
- Optionally migrate to simplified format for better readability

### For Tools and Libraries
- Update to handle both formats
- Provide conversion utilities if needed

## Conclusion

This implementation provides a clean, backward-compatible way to support simplified field definitions in declarative transforms while maintaining all existing functionality and validation requirements. The approach follows established patterns in the codebase and provides a clear migration path for improved developer experience.
