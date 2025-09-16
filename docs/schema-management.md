# Schema Management

This document covers fold db's schema management system built on the core principle of **schema immutability**.

## Table of Contents

1. [Schema Immutability](#schema-immutability)
2. [Schema Structure](#schema-structure)
3. [Simplified Schema Formats](#simplified-schema-formats) ⭐ **NEW**
4. [Field Types](#field-types)
5. [Permission Policies](#permission-policies)
6. [Payment Configuration](#payment-configuration)
7. [Schema States and Lifecycle](#schema-states-and-lifecycle)
8. [Migration Patterns](#migration-patterns)
9. [Best Practices](#best-practices)

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