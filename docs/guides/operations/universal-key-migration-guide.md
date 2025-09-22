# Universal Key Migration Guide

This guide helps you migrate existing schemas to use the new universal `key` configuration format introduced in SKC-1 and documents how the normalized `{ hash, range, fields }` payload now flows through the system.

## Overview

The universal key configuration provides a consistent way to define keys across all schema types (Single, Range, and HashRange), simplifying your codebase and enabling better performance. The same configuration now powers runtime helpers in `MutationService` and `AtomManager`, ensuring every mutation produces a deterministic payload.

## Universal Key Processing Workflow

The diagram below shows how schema metadata and helper utilities collaborate when a client issues a mutation:

```text
Client mutation
      │
      ▼
MutationService::normalized_field_value_request
      │  (loads schema → resolves keys → assembles payload)
      ▼
FieldValueSetRequest { hash, range, fields }
      │
      ▼
AtomManager::handle_fieldvalueset_request
      │  (persists atom → stores KeySnapshot → emits FieldValueSet event)
      ▼
Downstream consumers (transforms, analytics, message bus)
```

Key responsibilities:

- **`MutationService`** loads the schema, runs the universal key helpers, and returns both the serialized `FieldValueSetRequest` and a `NormalizedFieldContext` summary for logging or reuse.
- **`AtomManager`** receives the request, stores a `KeySnapshot` containing the normalized fields map, and publishes `FieldValueSet` events without recomputing keys.
- **Downstream services** (transform manager, message bus constructors, and analytics workers) rely on the normalized payload to avoid schema-specific conditionals.

See the reference documentation for deeper implementation details:

- [MutationService reference](../../reference/fold_db_core/mutation_service.md)
- [Field processing reference](../../reference/fold_db_core/field_processing.md)

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

### Step 4: Update Mutation Pipelines

Adopt the normalized payload builder in every service that issues `FieldValueSetRequest` messages. Each payload must contain:

```json
{
  "schema_name": "BlogPost",
  "field_name": "content",
  "fields": {
    "content": "..."
  },
  "hash": "optional-hash-value",
  "range": "optional-range-value",
  "context": {
    "mutation_hash": "trace id or correlation value"
  }
}
```

The `fields` object is always present, even when the key fields are populated separately. The helper also normalizes empty strings to `null` so downstream systems never see blank hashes or ranges.

### Step 5: Test Your Changes

1. **Verify Schema Loading**: Ensure schemas load without errors.
2. **Test Key Operations**: Verify key-based queries work correctly.
3. **Mutation Regression Tests**: Use the universal key regression suite (`tests/unit/field_processing/*`, `tests/integration/mutation_range_workflow_test.rs`) to confirm normalized payloads are emitted.
4. **Validate Results**: Ensure query results maintain expected format.

### Step 6: Deploy Gradually

1. **Deploy Schema Updates**: Update schemas in non-production first.
2. **Update Application Code**: Deploy code changes that use universal keys.
3. **Monitor Performance**: Watch for improvements in key-based operations.
4. **Complete Migration**: Migrate remaining schemas.

## Backward Compatibility

### Legacy Support

- ✅ **Legacy Range schemas** with `range_key` continue to work
- ✅ **Schemas without keys** continue to function normally
- ✅ **Existing queries** maintain their behavior
- ✅ **No breaking changes** to existing APIs

### Migration Timeline

- **Phase 1**: Universal key support added (current)
- **Phase 2**: Normalized payload adoption across MutationService and AtomManager (SKC-6).
- **Phase 3**: Documentation and troubleshooting coverage (this task).

## Troubleshooting

| Symptom | Likely Cause | Resolution |
|---------|--------------|------------|
| `SchemaError::InvalidData("Schema 'X' not found")` surfaced by MutationService | Schema name typo or schema not approved/loaded | Confirm the schema exists and is approved before issuing mutations. |
| `Missing hash key value for normalized request` log from MutationService | HashRange schema mutation omitted the configured hash key | Include the configured `key.hash_field` in the mutation payload or supply it via the helper parameters. |
| `Failed to extract keys` error during AtomManager processing | Dotted path key configuration does not match payload structure | Verify dotted path expressions using the examples in [Field processing reference](../../reference/fold_db_core/field_processing.md#dotted-path-resolution). |
| Downstream consumer receives empty string for `hash`/`range` | Callers manually constructed payloads without helper normalization | Refactor callers to use `MutationService::normalized_field_value_request` or `FieldValueSetRequest::from_normalized_parts`. |

When in doubt, enable debug logging for `MutationService` and `AtomManager` to trace the normalized context summary emitted with every request. The summary mirrors the `{ hash, range, fields }` triplet used by downstream services.
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

**Issue**: Schema validation errors after adding `key`
```json
{
  "error": "Range schema requires range_field in key configuration"
}
```

**Solution**: Ensure Range schemas include `range_field`:
```json
{
  "schema_type": "Range",
  "key": {
    "range_field": "timestamp"  // Required for Range schemas
  }
}
```

**Issue**: Legacy `range_key` not working
```json
{
  "error": "Unknown field: range_key"
}
```

**Solution**: Legacy `range_key` is still supported. Check schema parsing:
```json
{
  "schema_type": "Range",
  "range_key": "timestamp"  // Still works
}
```

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
