# Schema Management

This document covers fold db's schema management system built on the core principle of **schema immutability**.

## Table of Contents

1. [Schema Immutability](#schema-immutability)
2. [Schema Structure](#schema-structure)
3. [Universal Key Configuration](#universal-key-configuration) ⭐ **NEW**
4. [Simplified Schema Formats](#simplified-schema-formats) ⭐ **NEW**
5. [HashRange Query Processing](#hashrange-query-processing) ⭐ **NEW**
6. [Mutation Processing](#mutation-processing) ⭐ **NEW**
7. [Field Types](#field-types)
8. [Permission Policies](#permission-policies)
9. [Payment Configuration](#payment-configuration)
10. [Schema States and Lifecycle](#schema-states-and-lifecycle)
11. [Migration Patterns](#migration-patterns)
12. [Best Practices](#best-practices)

## Schema Immutability

> **Core Principle**: Schemas in fold db are immutable once created. This ensures data consistency, integrity guarantees, and predictable behavior.

### Why Schema Immutability?

- **Data Consistency**: Schema structure cannot change unexpectedly
- **Integrity Guarantees**: Existing data remains valid and accessible  
- **Predictable Behavior**: Applications can rely on stable schema contracts
- **Version Control**: Clear versioning through distinct schema names

### Key Rules

1. **No Updates**: Once stored, schema structure cannot be modified
2. **No Field Changes**: Field definitions, types, and constraints are permanent
3. **No Permission Modifications**: Permission policies are locked after creation
4. **Immutable Names**: Schema names serve as permanent identifiers

### When You Need Changes

To modify schema structure, **create a new schema with a different name**:

```bash
# Instead of updating existing schema
POST /api/schema {"name": "UserProfileV2", ...}

# Original schema remains unchanged
GET /api/schema/UserProfile  # Still available
```

## Schema Structure

A schema defines the structure, permissions, and behavior of data:

```json
{
  "name": "SchemaName",
  "fields": {
    "field_name": {
      "field_type": "Single|Collection|Range",
      "permission_policy": {
        "read_policy": "permission_requirement",
        "write_policy": "permission_requirement"
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "trust_distance_scaling": "scaling_config"
      },
      "writable": true
    }
  },
  "payment_config": {
    "base_multiplier": 1.0,
    "min_payment_threshold": 0
  }
}
```

### Schema Loading

**Via HTTP API:**
```bash
curl -X POST http://localhost:9001/api/schema \
  -H "Content-Type: application/json" \
  -d @schema.json
```

**Via CLI:**
```bash
datafold_cli load-schema schema.json
```

**Via TCP:**
```json
{
  "operation": "create_schema",
  "params": {
    "schema": { /* schema definition */ }
  }
}
```

## Universal Key Configuration

FoldDB now supports a universal `key` configuration that works across all schema types (Single, Range, and HashRange), providing consistent key management while maintaining full backward compatibility.

### Key Configuration Structure

The universal `key` configuration uses a consistent structure across all schema types:

```json
{
  "key": {
    "hash_field": "field_name_or_expression",
    "range_field": "field_name_or_expression"
  }
}
```

### Schema Type Requirements

#### Single Schema
- **Key**: Optional
- **Usage**: When present, can include either `hash_field`, `range_field`, or both
- **Purpose**: Enables consistent key-based operations across schema types

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

#### Range Schema
- **Key**: Optional (legacy `range_key` still supported)
- **Requirements**: If `key` is present, `range_field` is required
- **Migration**: Legacy `range_key` field continues to work

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

#### HashRange Schema
- **Key**: Required
- **Requirements**: Both `hash_field` and `range_field` must be specified
- **Purpose**: Enables efficient partitioning and ordering

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

### Migration from Legacy Formats

#### From Legacy Range Schema
Legacy Range schemas using `range_key` continue to work without changes:

```json
// Legacy format (still supported)
{
  "name": "LegacyRange",
  "schema_type": "Range",
  "range_key": "timestamp",
  "fields": {
    "timestamp": {},
    "value": {}
  }
}

// New universal format (recommended)
{
  "name": "ModernRange",
  "schema_type": "Range", 
  "key": {
    "range_field": "timestamp"
  },
  "fields": {
    "timestamp": {},
    "value": {}
  }
}
```

#### From Schemas Without Keys
Schemas without key configuration can be enhanced by adding the universal `key`:

```json
// Before: No key configuration
{
  "name": "SimpleSchema",
  "schema_type": "Single",
  "fields": {
    "id": {},
    "name": {}
  }
}

// After: With universal key
{
  "name": "EnhancedSchema",
  "schema_type": "Single",
  "key": {
    "hash_field": "id"
  },
  "fields": {
    "id": {},
    "name": {}
  }
}
```

### Benefits of Universal Key Configuration

- ✅ **Consistent API**: Same key structure across all schema types
- ✅ **Backward Compatible**: Legacy formats continue to work
- ✅ **Future-Proof**: Unified approach for new features
- ✅ **Simplified Logic**: Reduces code complexity in applications
- ✅ **Better Performance**: Optimized key-based operations

### Best Practices

1. **Use Universal Keys**: Prefer the new `key` format over legacy `range_key`
2. **Consistent Naming**: Use descriptive field names for keys
3. **Migration Strategy**: Gradually migrate existing schemas
4. **Documentation**: Document key fields and their purposes
5. **Testing**: Verify key-based operations work as expected

## Simplified Schema Formats

FoldDB now supports simplified schema formats that reduce boilerplate by up to 90% while maintaining full backward compatibility.

### Format Types

#### 1. Ultra-Minimal Format (Regular Schemas)
Use empty field objects `{}` to get default configurations:

```json
{
  "name": "UserProfile",
  "schema_type": "Single",
  "fields": {
    "id": {},
    "name": {},
    "email": {},
    "avatar": {}
  },
  "payment_config": {
    "base_multiplier": 1.0,
    "min_payment_threshold": 0
  }
}
```

#### 2. String Expression Format (Declarative Transforms)
Use string expressions for field mappings:

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

#### 3. Mixed Format
Combine simplified and verbose formats in the same schema:

```json
{
  "name": "MixedSchema",
  "schema_type": "Single",
  "fields": {
    "simple_field": "Source.map().id",
    "complex_field": {
      "atom_uuid": "Source.map().metadata.tags",
      "field_type": "Single",
      "permission_policy": {
        "read_policy": {"Distance": 0},
        "write_policy": {"Distance": 1}
      }
    },
    "empty_field": {}
  }
}
```

### Benefits

- ✅ **90% Less Boilerplate**: Dramatically reduced schema size
- ✅ **Better Readability**: Cleaner, more intuitive definitions
- ✅ **Faster Development**: Quick schema creation and iteration
- ✅ **Backward Compatible**: All existing schemas continue to work
- ✅ **Flexible**: Mix simplified and verbose formats as needed

### Default Values

When using simplified formats, the following defaults are applied:

**Regular Schema Fields:**
- `field_type`: `"Single"`
- `permission_policy`: `{"read_policy": {"Distance": 0}, "write_policy": {"Distance": 1}}`
- `payment_config`: `{"base_multiplier": 1.0, "trust_distance_scaling": "None"}`
- `molecule_uuid`: `null`
- `field_mappers`: `{}`
- `transform`: `null`

**Declarative Transform Fields:**
- `atom_uuid`: String expression value
- `field_type`: `null` (inherited from schema context)

### Migration Guide

**From Verbose to Simplified:**

1. **Regular Schemas**: Replace verbose field definitions with `{}`
2. **Declarative Transforms**: Convert `{"atom_uuid": "expression"}` to `"expression"`
3. **Mixed Approach**: Gradually migrate fields as needed

**Example Migration:**
```bash
# Before (verbose)
"fields": {
  "id": {
    "permission_policy": {"read_policy": {"Distance": 0}, "write_policy": {"Distance": 1}},
    "molecule_uuid": null,
    "payment_config": {"base_multiplier": 1.0, "trust_distance_scaling": "None"},
    "field_mappers": {},
    "field_type": "Single",
    "transform": null
  }
}

# After (simplified)
"fields": {
  "id": {}
}
```

## Field Types

### Single Fields
Store scalar values (strings, numbers, booleans).

```json
{
  "username": {
    "field_type": "Single",
    "permission_policy": {
      "read_policy": {"NoRequirement": null},
      "write_policy": {"Distance": 1}
    }
  }
}
```

### Collection Fields
Store arrays of values.

```json
{
  "tags": {
    "field_type": "Collection",
    "permission_policy": {
      "read_policy": {"NoRequirement": null},
      "write_policy": {"Distance": 0}
    }
  }
}
```

### Range Fields
Store ranges of values with start/end points.

```json
{
  "availability": {
    "field_type": "Range",
    "permission_policy": {
      "read_policy": {"NoRequirement": null},
      "write_policy": {"Distance": 2}
    }
  }
}
```

## Permission Policies

### Policy Types

- **NoRequirement**: No restrictions
- **Distance**: Requires specific trust distance
- **Explicit**: Requires explicit permission grants

### Examples

```json
{
  "permission_policy": {
    "read_policy": {"NoRequirement": null},
    "write_policy": {"Distance": 1},
    "explicit_read_policy": {"Explicit": ["alice", "bob"]},
    "explicit_write_policy": {"Explicit": ["admin"]}
  }
}
```

## Payment Configuration

### Field-Level Payments

```json
{
  "payment_config": {
    "base_multiplier": 1.5,
    "trust_distance_scaling": {"Linear": 0.1},
    "min_payment": 100
  }
}
```

### Schema-Level Payments

```json
{
  "payment_config": {
    "base_multiplier": 1.0,
    "min_payment_threshold": 50
  }
}
```

## Schema States and Lifecycle

### Schema States

- **Available**: Schema exists but not active for operations
- **Approved**: Schema is active and can be used for queries/mutations
- **Blocked**: Schema is disabled and cannot be used

### State Management

```bash
# Approve schema for use
POST /api/schema/{name}/approve

# Block schema from use  
POST /api/schema/{name}/block

# Check schema state
GET /api/schema/{name}/state
```

### Lifecycle Operations

```bash
# List schemas with states
GET /api/schemas

# Load schema from available_schemas directory
POST /api/schema/{name}/load

# Unload schema (make unavailable)
DELETE /api/schema/{name}
```

## Migration Patterns

For comprehensive migration strategies, patterns, and step-by-step processes, see the [Migration Guide](migration-guide.md).

**Quick Migration Overview:**
1. **Create New Schema** → Design with required changes
2. **Deploy App Updates** → Handle both old and new schemas
3. **Migrate Data** → Transform data from old to new schema
4. **Switch References** → Update app to use new schema
5. **Deprecate Old** → Block old schema when migration complete

## HashRange Query Processing

HashRange schemas support efficient querying with universal key configuration, enabling both hash-based partitioning and range-based ordering.

### Query Structure

HashRange queries return results in a consistent `hash->range->fields` format:

```json
{
  "hash_key_1": {
    "range_key_1": {
      "field1": "value1",
      "field2": "value2"
    },
    "range_key_2": {
      "field1": "value3",
      "field2": "value4"
    }
  },
  "hash_key_2": {
    "range_key_1": {
      "field1": "value5",
      "field2": "value6"
    }
  }
}
```

### Universal Key Configuration

HashRange schemas **require** both `hash_field` and `range_field` in their key configuration:

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

### Query Methods

#### 1. Hash-Filtered Queries

Query specific hash keys with their range data:

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

**Response:**
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

#### 2. Unfiltered Queries

Get the first 10 hash keys and their data:

```bash
curl -X POST http://localhost:9001/api/query \
  -H "Content-Type: application/json" \
  -d '{
    "schema": "BlogPostWordIndex",
    "fields": ["word", "publish_date", "content"]
  }'
```

**Response:**
```json
{
  "technology": {
    "2025-01-15": {
      "word": "technology",
      "publish_date": "2025-01-15",
      "content": "AI and machine learning..."
    }
  },
  "science": {
    "2025-01-10": {
      "word": "science",
      "publish_date": "2025-01-10",
      "content": "New discoveries in physics..."
    }
  }
}
```

### Error Handling

#### Missing Key Configuration

```json
{
  "error": "HashRange schema 'BlogPostWordIndex' requires key configuration"
}
```

**Solution:** Ensure HashRange schemas include both `hash_field` and `range_field`:

```json
{
  "schema_type": "HashRange",
  "key": {
    "hash_field": "word",
    "range_field": "publish_date"
  }
}
```

#### Empty Key Fields

```json
{
  "error": "HashRange schema 'BlogPostWordIndex' requires non-empty hash_field in key configuration"
}
```

**Solution:** Provide non-empty values for both key fields:

```json
{
  "key": {
    "hash_field": "word",      // Must not be empty
    "range_field": "publish_date"  // Must not be empty
  }
}
```

#### Invalid Hash Filter

```json
{
  "error": "Hash filter must be an object"
}
```

**Solution:** Use proper hash filter format:

```json
{
  "filter": {
    "hash_filter": {
      "Key": "technology"  // Must be an object with 'Key' field
    }
  }
}
```

### Performance Considerations

- **Hash Partitioning**: Efficient data distribution across hash keys
- **Range Ordering**: Fast range-based queries within hash partitions
- **Memory Usage**: Optimized storage for large datasets
- **Query Speed**: Sub-linear lookup times for hash-filtered queries

### Migration from Legacy HashRange

Legacy HashRange schemas continue to work without changes. The universal key configuration provides:

- ✅ **Consistent API**: Same key structure as other schema types
- ✅ **Better Performance**: Optimized key extraction and query processing
- ✅ **Clear Error Messages**: Detailed validation feedback
- ✅ **Future-Proof**: Unified approach for new features

## Mutation Processing

FoldDB's mutation processor supports universal key configuration across all schema types, providing consistent mutation handling while maintaining full backward compatibility.

### Universal Key Mutation Support

The mutation processor automatically extracts hash and range values from mutations using the schema's universal key configuration:

```json
{
  "schema_name": "BlogPostWordIndex",
  "mutation_type": "Update",
  "fields_and_values": {
    "word": "technology",
    "publish_date": "2025-01-15",
    "content": "AI and machine learning advances..."
  }
}
```

### Schema Type Support

#### HashRange Schemas
HashRange schemas require both `hash_field` and `range_field` in their key configuration:

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

**Mutation Requirements:**
- Must include values for both hash and range fields
- Hash field value determines the partition
- Range field value determines the sort order within the partition

#### Range Schemas
Range schemas can use either universal key configuration or legacy `range_key`:

**Universal Key Configuration:**
```json
{
  "name": "UserActivity",
  "schema_type": "Range",
  "key": {
    "hash_field": "",
    "range_field": "timestamp"
  },
  "fields": {
    "timestamp": {},
    "action": {},
    "user_id": {}
  }
}
```

**Legacy Format (Backward Compatible):**
```json
{
  "name": "UserActivity",
  "schema_type": "Range",
  "range_key": "timestamp",
  "fields": {
    "timestamp": {},
    "action": {},
    "user_id": {}
  }
}
```

#### Single Schemas
Single schemas can optionally use universal key configuration for indexing hints:

```json
{
  "name": "UserProfile",
  "schema_type": "Single",
  "key": {
    "hash_field": "user_id",
    "range_field": "created_at"
  },
  "fields": {
    "user_id": {},
    "name": {},
    "email": {},
    "created_at": {}
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

#### Empty Key Fields
```json
{
  "error": "HashRange schema 'BlogPostWordIndex' requires non-empty hash_field in key configuration"
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

### Backward Compatibility

Legacy mutation formats continue to work without changes:

- **Legacy Range Schemas**: Mutations using `range_key` field names work unchanged
- **Existing HashRange Schemas**: Mutations continue to work with existing field names
- **Single Schema Mutations**: No changes required for existing mutations

### Migration Examples

#### Updating HashRange Mutations
**Before (Legacy):**
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

**After (Universal Key):**
```json
{
  "schema_name": "BlogPostWordIndex",
  "fields_and_values": {
    "word": "technology",
    "publish_date": "2025-01-15",
    "content": "AI advances..."
  }
}
```

#### Updating Range Mutations
**Before (Legacy):**
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
    "timestamp": "2025-01-15T10:30:00Z",
    "action": "login",
    "user_id": "user123"
  }
}
```

### Performance Considerations

- **Key Extraction**: Universal key extraction is optimized for performance
- **Field Validation**: Early validation prevents unnecessary processing
- **Error Handling**: Clear error messages reduce debugging time
- **Backward Compatibility**: No performance impact on legacy mutations

### Troubleshooting

#### Common Issues

1. **"Missing key configuration"**
   - Ensure HashRange schemas have a `key` field
   - Verify key configuration is properly formatted

2. **"Missing hash/range field"**
   - Check that mutation includes values for required key fields
   - Verify field names match the schema's key configuration

3. **"Empty key fields"**
   - Ensure key configuration has non-empty field names
   - Check for typos in field names

4. **Legacy mutations failing**
   - Verify schema still supports legacy field names
   - Check if schema was updated to require universal key configuration

## Best Practices

### Development
- Use versioned schema names (V1, V2, V3)
- Test schema designs thoroughly before production
- Document migration paths between versions

### Production
- Plan migration strategies in advance
- Maintain backward compatibility during transitions
- Use semantic versioning in schema names

### Maintenance
- Block deprecated schemas rather than deleting
- Maintain data access for historical purposes
- Monitor usage before deprecating schemas

### Performance
- Design fields with appropriate permission policies
- Use payment configs to manage resource usage
- Consider query patterns when designing schema structure