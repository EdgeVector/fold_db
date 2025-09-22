# Universal Key Migration Guide

This guide helps you migrate existing schemas to use the new universal `key` configuration format introduced in SKC-1.

## Overview

The universal key configuration provides a consistent way to define keys across all schema types (Single, Range, and HashRange), simplifying your codebase and enabling better performance.

## Quick Reference

| Schema Type | Legacy Format | Universal Format | Required Fields |
|-------------|---------------|------------------|-----------------|
| Single | No key support | `key: { hash_field?, range_field? }` | None |
| Range | `range_key: "field"` | `key: { range_field: "field", hash_field? }` | `range_field` |
| HashRange | `key: { hash_field, range_field }` | `key: { hash_field, range_field }` | Both |

## Migration Examples

### 1. Single Schema Migration

**Before (No Key Support):**
```json
{
  "name": "UserProfile",
  "schema_type": "Single",
  "fields": {
    "user_id": {},
    "name": {},
    "email": {}
  }
}
```

**After (With Universal Key):**
```json
{
  "name": "UserProfile",
  "schema_type": "Single",
  "key": {
    "hash_field": "user_id"
  },
  "fields": {
    "user_id": {},
    "name": {},
    "email": {}
  }
}
```

**Benefits:**
- Enables consistent key-based operations
- Improves query performance
- Future-proofs your schema

### 2. Range Schema Migration

**Before (Legacy range_key):**
```json
{
  "name": "TimeSeriesData",
  "schema_type": "Range",
  "range_key": "timestamp",
  "fields": {
    "timestamp": {},
    "value": {},
    "metadata": {}
  }
}
```

**After (Universal Key):**
```json
{
  "name": "TimeSeriesData",
  "schema_type": "Range",
  "key": {
    "range_field": "timestamp"
  },
  "fields": {
    "timestamp": {},
    "value": {},
    "metadata": {}
  }
}
```

**Note:** Legacy `range_key` format continues to work, but universal `key` is recommended for new schemas.

### 3. HashRange Schema Migration

**Before (Already Universal):**
```json
{
  "name": "BlogPostWordIndex",
  "schema_type": "HashRange",
  "key": {
    "hash_field": "word",
    "range_field": "publish_date"
  },
  "fields": {
    "word": {},
    "publish_date": {},
    "content": {}
  }
}
```

**After (No Changes Needed):**
HashRange schemas already use the universal key format. No migration required.

## Migration Steps

### Step 1: Identify Your Schema Types

1. **Single Schemas**: Add optional `key` configuration
2. **Range Schemas**: Convert `range_key` to `key.range_field`
3. **HashRange Schemas**: Already compatible

### Step 2: Update Schema Definitions

For each schema, add or update the key configuration:

```bash
# Example: Update a Single schema
curl -X POST http://localhost:9001/api/schema \
  -H "Content-Type: application/json" \
  -d '{
    "name": "UserProfileV2",
    "schema_type": "Single",
    "key": {
      "hash_field": "user_id"
    },
    "fields": {
      "user_id": {},
      "name": {},
      "email": {}
    }
  }'
```

### Step 3: Update Application Code

Update your application code to use the new universal key helpers:

**JavaScript/TypeScript:**
```javascript
// Before: Schema-specific logic
if (schema.schema_type === 'Range') {
  const rangeKey = schema.range_key;
} else if (schema.schema_type === 'HashRange') {
  const hashKey = schema.key.hash_field;
  const rangeKey = schema.key.range_field;
}

// After: Universal key helpers
import { getHashKey, getRangeKey } from './utils/rangeSchemaHelpers.js';

const hashKey = getHashKey(schema);
const rangeKey = getRangeKey(schema);
```

**Rust:**
```rust
// Before: Schema-specific extraction
let range_key = match schema.schema_type {
    SchemaType::Range { range_key } => Some(range_key),
    SchemaType::HashRange { key } => Some(key.range_field.clone()),
    _ => None,
};

// After: Universal key extraction
let (hash_key, range_key) = extract_unified_keys(&schema);
```

### Step 4: Test Your Changes

1. **Verify Schema Loading**: Ensure schemas load without errors
2. **Test Key Operations**: Verify key-based queries work correctly
3. **Check Performance**: Monitor query performance improvements
4. **Validate Results**: Ensure query results maintain expected format

### Step 5: Deploy Gradually

1. **Deploy Schema Updates**: Update schemas in non-production first
2. **Update Application Code**: Deploy code changes that use universal keys
3. **Monitor Performance**: Watch for improvements in key-based operations
4. **Complete Migration**: Migrate remaining schemas

## Field Processing and Mutation Workflow

Universal key adoption is only complete when the normalized payload travels cleanly from mutation entrypoints through
AtomManager. The pipeline now relies on two reusable helpers:

- **MutationService builder** – constructs a `FieldValueSetRequest` whose `value` body always contains `{ hash, range, fields }`
  populated via schema metadata. See the [MutationService reference](reference/fold_db_core/mutation_service.md).
- **AtomManager resolver** – ingests the request, resolves universal keys with `resolve_universal_keys`, and publishes a
  `FieldValueSetResponse` that echoes the normalized snapshot. See the
  [Field Processing reference](reference/fold_db_core/field_processing.md).

### Sequence Overview

```text
Mutation caller
   │
   │ normalized_field_value_request (hash, range, fields)
   ▼
MutationService ── FieldValueSetRequest ──▶ Message Bus ──▶ AtomManager
                                                        │
                                                        ├─ resolve_universal_keys → KeySnapshot
                                                        └─ FieldValueSetResponse (hash, range, fields)
```

1. `MutationService::normalized_field_value_request` resolves schema keys, sorts payload fields, and attaches a
   `MutationContext` when incremental metadata is present.
2. The message bus transports the serialized `FieldValueSetRequest` with a correlation identifier and signer metadata.
3. `AtomManager::handle_fieldvalueset_request` reuses the normalized snapshot when creating molecules and when emitting the
   `FieldValueSetResponse` so downstream consumers never see divergent key data.

### Normalized Payload Anatomy

Every `FieldValueSetRequest` now carries the same JSON structure inside `value`:

```json
{
  "hash": "technology",
  "range": "2025-01-15",
  "fields": {
    "word": "technology",
    "publish_date": "2025-01-15",
    "content": "AI updates"
  }
}
```

The `fields` object mirrors the output of `shape_unified_result`, so dotted-path extractions are already flattened and safe for
AtomManager, transform pipelines, and analytics subscribers.

## Backward Compatibility

### Legacy Support

- ✅ **Legacy Range schemas** with `range_key` continue to work
- ✅ **Schemas without keys** continue to function normally
- ✅ **Existing queries** maintain their behavior
- ✅ **No breaking changes** to existing APIs

### Migration Timeline

- **Phase 1**: Universal key support added (current)
- **Phase 2**: Legacy formats deprecated (future)
- **Phase 3**: Legacy formats removed (future)

## Best Practices

### 1. Choose Descriptive Key Names

```json
// Good: Descriptive and clear
"key": {
  "hash_field": "user_id",
  "range_field": "created_at"
}

// Avoid: Generic or unclear names
"key": {
  "hash_field": "id",
  "range_field": "time"
}
```

### 2. Document Your Key Strategy

```json
{
  "name": "UserActivity",
  "schema_type": "HashRange",
  "key": {
    "hash_field": "user_id",  // Partition by user
    "range_field": "timestamp" // Order by time
  },
  "fields": {
    "user_id": {},
    "timestamp": {},
    "activity_type": {},
    "details": {}
  }
}
```

### 3. Test Key-Based Operations

```bash
# Test hash-based queries
curl -X POST http://localhost:9001/api/query \
  -H "Content-Type: application/json" \
  -d '{
    "schema": "UserActivity",
    "filter": {
      "hash_filter": {
        "Key": "user123"
      }
    }
  }'

# Test range-based queries
curl -X POST http://localhost:9001/api/query \
  -H "Content-Type: application/json" \
  -d '{
    "schema": "UserActivity",
    "filter": {
      "range_filter": {
        "timestamp": {
          "KeyRange": {
            "start": "2025-01-01",
            "end": "2025-01-31"
          }
        }
      }
    }
  }'
```

### 4. Monitor Performance

- **Query Performance**: Monitor improvements in key-based queries
- **Storage Efficiency**: Check for better data organization
- **Memory Usage**: Verify reduced memory overhead

### 5. HashRange Query Examples

**Hash-Filtered Query:**
```bash
curl -X POST http://localhost:9001/api/query \
  -H "Content-Type: application/json" \
  -d '{
    "schema": "BlogPostWordIndex",
    "fields": ["word", "publish_date", "content"],
    "filter": {
      "hash_filter": {
        "Key": "technology"
      }
    }
  }'
```

**Response Format (hash->range->fields):**
```json
{
  "technology": {
    "2025-01-15": {
      "word": "technology",
      "publish_date": "2025-01-15",
      "content": "AI and machine learning..."
    },
    "2025-01-20": {
      "word": "technology",
      "publish_date": "2025-01-20",
      "content": "Quantum computing advances..."
    }
  }
}
```

**Unfiltered Query (first 10 hash keys):**
```bash
curl -X POST http://localhost:9001/api/query \
  -H "Content-Type: application/json" \
  -d '{
    "schema": "BlogPostWordIndex",
    "fields": ["word", "publish_date", "content"]
  }'
```

## Mutation Processor Migration

The mutation processor has been updated to support universal key configuration across all schema types. This section covers migrating your mutations to use the new key system.

### Mutation Requirements by Schema Type

#### HashRange Schemas
HashRange mutations must include values for both hash and range fields as defined in the schema's key configuration:

```json
{
  "schema_name": "BlogPostWordIndex",
  "mutation_type": "Update",
  "fields_and_values": {
    "word": "technology",           // hash_field
    "publish_date": "2025-01-15",  // range_field
    "content": "AI advances..."
  }
}
```

#### Range Schemas
Range mutations must include the range field value. Hash field is optional:

```json
{
  "schema_name": "UserActivity",
  "mutation_type": "Update",
  "fields_and_values": {
    "timestamp": "2025-01-15T10:30:00Z",  // range_field
    "action": "login",
    "user_id": "user123"
  }
}
```

#### Single Schemas
Single mutations can optionally include key field values for indexing hints:

```json
{
  "schema_name": "UserProfile",
  "mutation_type": "Update",
  "fields_and_values": {
    "user_id": "user123",           // hash_field (optional)
    "name": "John Doe",
    "email": "john@example.com",
    "created_at": "2025-01-15"      // range_field (optional)
  }
}
```

### Migration Examples

#### HashRange Mutation Migration
**Before (Legacy Field Names):**
```json
{
  "schema_name": "BlogPostWordIndex",
  "fields_and_values": {
    "hash_key": "technology",
    "range_key": "2025-01-15",
    "content": "AI advances..."
  }
}
```

**After (Universal Key Field Names):**
```json
{
  "schema_name": "BlogPostWordIndex",
  "fields_and_values": {
    "word": "technology",           // matches hash_field in schema
    "publish_date": "2025-01-15",   // matches range_field in schema
    "content": "AI advances..."
  }
}
```

#### Range Mutation Migration
**Before (Legacy range_key):**
```json
{
  "schema_name": "UserActivity",
  "fields_and_values": {
    "range_key": "2025-01-15T10:30:00Z",
    "action": "login",
    "user_id": "user123"
  }
}
```

**After (Universal Key):**
```json
{
  "schema_name": "UserActivity",
  "fields_and_values": {
    "timestamp": "2025-01-15T10:30:00Z",  // matches range_field in schema
    "action": "login",
    "user_id": "user123"
  }
}
```

### Error Handling

The mutation processor provides clear error messages for invalid configurations:

#### Missing Key Configuration
```json
{
  "error": "HashRange schema 'BlogPostWordIndex' requires key configuration"
}
```

#### Missing Required Fields
```json
{
  "error": "HashRange schema mutation missing hash field 'word'"
}
```

```json
{
  "error": "Range schema mutation missing range field 'timestamp'"
}
```

#### Empty Key Fields
```json
{
  "error": "HashRange schema 'BlogPostWordIndex' requires non-empty hash_field in key configuration"
}
```

### Backward Compatibility

Legacy mutations continue to work without changes:

- **Legacy Range Schemas**: Mutations using `range_key` field names work unchanged
- **Existing HashRange Schemas**: Mutations continue to work with existing field names
- **Single Schema Mutations**: No changes required for existing mutations

### Migration Checklist

- [ ] **Identify Schema Types**: Determine which schemas use HashRange, Range, or Single
- [ ] **Check Key Configuration**: Verify schemas have proper `key` configuration
- [ ] **Update Field Names**: Change mutation field names to match schema key configuration
- [ ] **Test Mutations**: Verify mutations work with new field names
- [ ] **Update Documentation**: Update API documentation and examples
- [ ] **Monitor Errors**: Watch for mutation validation errors

### Performance Benefits

- **Optimized Key Extraction**: Universal key extraction is faster than legacy methods
- **Early Validation**: Field validation happens before processing
- **Clear Error Messages**: Reduced debugging time with descriptive errors
- **Consistent API**: Unified approach across all schema types

## Troubleshooting

### Common Issues

#### Missing Key Configuration

- **Symptom**: Errors such as `HashRange schema 'BlogPostWordIndex' requires hash key value` appear when constructing the
  normalized payload.
- **Resolution**: Ensure the schema's universal key configuration lists both `hash_field` and `range_field` for HashRange
  schemas, and that the mutation payload provides the required values. See the
  [MutationService reference](reference/fold_db_core/mutation_service.md#failure-modes) for validation details.

#### Dotted-Path Resolution Failures

- **Symptom**: AtomManager logs report `Failed to extract keys for schema 'UserActivity': activities.0.timestamp missing` or
  similar dotted-path errors.
- **Resolution**: Verify that the mutation payload includes the nested structure referenced by the dotted path. The
  [Field Processing reference](reference/fold_db_core/field_processing.md#troubleshooting-signals) explains how
  `resolve_universal_keys` identifies the failing segment and how to inspect the published request.

#### Inconsistent Payload Snapshots

- **Symptom**: Downstream processors observe mismatched hash/range metadata compared to stored atoms.
- **Resolution**: Confirm every mutation entry point uses `MutationService::normalized_field_value_request` (or
  `FieldValueSetRequest::from_normalized_parts`) instead of manual JSON assembly. Cross-check the event body with the
  [normalized payload anatomy](#normalized-payload-anatomy) section above.

### Getting Help

- **Documentation**: See [Schema Management Guide](schema-management.md)
- **Examples**: Check `available_schemas/` directory for examples
- **Testing**: Use the test suite to verify your schemas

## Summary

The universal key configuration provides:

- ✅ **Consistent API** across all schema types
- ✅ **Better Performance** for key-based operations
- ✅ **Simplified Code** with unified key handling
- ✅ **Future-Proof** design for new features
- ✅ **Backward Compatible** with existing schemas

Start migrating your schemas today to take advantage of these improvements!
