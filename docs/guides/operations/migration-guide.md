# Schema Migration Guide

This guide provides comprehensive strategies for migrating between schemas in fold db, where schemas are immutable once created.

## Table of Contents

1. [Migration Principles](#migration-principles)
2. [Migration Patterns](#migration-patterns)
3. [Migration Process](#migration-process)
4. [Migration Strategies](#migration-strategies)
5. [Best Practices](#best-practices)
6. [Common Scenarios](#common-scenarios)

## Migration Principles

### Core Concept
Since schemas are immutable in fold db, any structural changes require creating a new schema with a different name and migrating data between schemas.

### Why Migration is Necessary
- **Schema Immutability**: Once created, schemas cannot be modified
- **Data Consistency**: Ensures existing data remains valid
- **Version Control**: Clear versioning through distinct schema names
- **Gradual Transition**: Allows phased migration without downtime

## Migration Patterns

### 1. Versioned Schema Pattern
Use sequential version numbers in schema names:

```json
// Original schema
{
  "name": "UserProfileV1",
  "fields": {
    "username": {"field_type": "Single"},
    "email": {"field_type": "Single"}
  }
}

// Enhanced version
{
  "name": "UserProfileV2", 
  "fields": {
    "username": {"field_type": "Single"},
    "email": {"field_type": "Single"},
    "phone": {"field_type": "Single"},
    "created_at": {"field_type": "Single"}
  }
}
```

### 2. Feature-Specific Pattern
Create schemas based on feature requirements:

```json
// Base schema
{
  "name": "UserProfile",
  "fields": {
    "username": {"field_type": "Single"},
    "email": {"field_type": "Single"}
  }
}

// Extended schema for premium features
{
  "name": "UserProfileWithPreferences",
  "fields": {
    "username": {"field_type": "Single"},
    "email": {"field_type": "Single"},
    "theme": {"field_type": "Single"},
    "notifications": {"field_type": "Single"},
    "preferences": {"field_type": "Range"}
  }
}
```

### 3. Domain-Specific Pattern
Separate schemas by business domain:

```json
// Identity data
{
  "name": "UserIdentity",
  "fields": {
    "username": {"field_type": "Single"},
    "email": {"field_type": "Single"}
  }
}

// Profile data
{
  "name": "UserProfile",
  "fields": {
    "bio": {"field_type": "Single"},
    "location": {"field_type": "Single"},
    "profile_data": {"field_type": "Range"}
  }
}
```

## Migration Process

### Step 1: Create New Schema
Design and deploy the new schema with required changes:

```bash
curl -X POST http://localhost:9001/api/schema \
  -H "Content-Type: application/json" \
  -d @user_profile_v2.json
```

### Step 2: Deploy Application Updates
Update your application to handle both old and new schemas during the transition:

```rust
// Rust example - handle both schemas
let user_data = match schema_version {
    "UserProfileV1" => fetch_user_v1(username).await?,
    "UserProfileV2" => fetch_user_v2(username).await?,
    _ => return Err("Unsupported schema version"),
};
```

### Step 3: Migrate Data
Copy and transform data from old to new schema:

```bash
# Query data from old schema
curl -X POST http://localhost:9001/api/query \
  -H "Content-Type: application/json" \
  -d '{
    "schema": "UserProfileV1",
    "fields": ["username", "email"]
  }'

# Transform and insert into new schema
curl -X POST http://localhost:9001/api/mutation \
  -H "Content-Type: application/json" \
  -d '{
    "schema": "UserProfileV2",
    "mutation_type": "create",
    "data": {
      "username": "alice",
      "email": "alice@example.com",
      "phone": "+1234567890",
      "created_at": "2024-01-15T10:30:00Z"
    }
  }'
```

### Step 4: Update References
Switch application logic to use the new schema:

```bash
# Before
datafold_cli query --schema UserProfileV1 --fields username,email

# After  
datafold_cli query --schema UserProfileV2 --fields username,email,phone,created_at
```

### Step 5: Deprecate Old Schema
Block the old schema when migration is complete:

```bash
curl -X POST http://localhost:9001/api/schema/UserProfileV1/block
```

## Migration Strategies

### Zero-Downtime Migration

1. **Dual-Write Phase**: Write to both old and new schemas
2. **Gradual Read Migration**: Gradually shift reads to new schema
3. **Validation Phase**: Verify data consistency between schemas
4. **Cutover**: Complete switch to new schema
5. **Cleanup**: Block old schema after verification

### Batch Migration

1. **Offline Migration**: Migrate data during maintenance window
2. **Verification**: Validate migrated data
3. **Application Update**: Deploy application changes
4. **Go-Live**: Switch to new schema

### Incremental Migration

1. **User-Based**: Migrate users in batches
2. **Feature-Based**: Migrate features incrementally
3. **Time-Based**: Migrate data by time periods

## Best Practices

### Development Phase
- Use versioned schema names consistently
- Test migration scripts thoroughly
- Document migration paths between versions
- Plan rollback strategies

### Production Phase
- Implement data validation checks
- Monitor system performance during migration
- Maintain backward compatibility during transitions
- Use feature flags to control migration rollout

### Maintenance Phase
- Archive old schemas rather than deleting
- Maintain historical data access
- Document migration history
- Monitor for orphaned data

## Common Scenarios

### Adding Fields
```bash
# Create new schema with additional fields
POST /api/schema -d @schema_with_new_fields.json

# Migrate existing data with default values
# Update application to use new schema
```

### Changing Field Types
```bash
# Create new schema with updated field types
POST /api/schema -d @schema_with_new_types.json

# Transform data during migration
# Validate transformed data
```

### Restructuring Data
```bash
# Design new schema structure
# Create transformation logic
# Migrate data with transformation
# Update application logic
```

### Splitting Schemas
```bash
# Create multiple new schemas
# Distribute data across new schemas
# Update application to query multiple schemas
```

### Merging Schemas
```bash
# Create unified schema
# Combine data from multiple sources
# Update application to use single schema
```

## Migration Tools

### Data Export/Import
```bash
# Export from old schema
datafold_cli export --schema UserProfileV1 --output data.json

# Transform data (external script)
python transform_data.py data.json transformed_data.json

# Import to new schema
datafold_cli import --schema UserProfileV2 --input transformed_data.json
```

### Validation Scripts
```bash
# Validate data consistency
datafold_cli validate --old-schema UserProfileV1 --new-schema UserProfileV2

# Check migration completeness
datafold_cli migration-status --from UserProfileV1 --to UserProfileV2
```

This migration guide ensures smooth transitions between schema versions while maintaining data integrity and system availability.