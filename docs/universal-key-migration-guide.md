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
