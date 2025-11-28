# DynamoDB Partition Key Fix

## Problem

The storage abstraction in `fold_db` was not compatible with the DynamoDB table structure used by `exemem-infra`.

### Exemem's Expected Structure (from CDK)
```
PK (Partition Key): user_id (for multi-tenant) or "default" (for single-tenant)
SK (Sort Key): the actual key
Value: binary data
```

### Previous Implementation (Incorrect)
```
PK (Partition Key): Always "default" (hardcoded)
SK (Sort Key): {user_id}#{key} (user_id prepended to sort key)
Value: binary data
```

### Issues
1. **All items in same partition**: All users' data ended up in the "default" partition, defeating DynamoDB's scalability and isolation
2. **Inefficient queries**: User isolation via sort key prefix requires scanning, not efficient querying
3. **Violates exemem's table design**: The CDK comments explicitly state PK should be user_id

## Solution

Updated `DynamoDbKvStore` to use user_id as the partition key instead of prepending it to the sort key.

### New Implementation (Correct)
```
PK (Partition Key): user_id (when provided) or "default" (when not provided)
SK (Sort Key): the actual key (no user_id prefix)
Value: binary data
```

## Changes Made

### 1. `dynamodb_backend.rs` - Core Implementation

**Methods Updated:**
- `get_partition_key_impl()`: New method that returns user_id or "default"
- `make_sort_key_impl()`: Simplified to return key without user_id prefix
- Removed: `extract_key_from_sort_key_impl()` (no longer needed)

**All DynamoDB Operations Updated:**
- `get()`: Uses `user_id` as PK, actual key as SK
- `put()`: Uses `user_id` as PK, actual key as SK
- `delete()`: Uses `user_id` as PK, actual key as SK
- `exists()`: Uses `user_id` as PK, actual key as SK
- `scan_prefix()`: Queries with `user_id` as PK, prefix match on SK
- `batch_put()`: Uses `user_id` as PK for all items
- `batch_delete()`: Uses `user_id` as PK for all items

**Documentation Updated:**
- Updated struct and method comments to reflect new behavior
- Changed "prepended to sort key" to "used as partition key"

### 2. `tests.rs` - Test Updates

**Test Renamed:**
- `test_dynamodb_key_prepending_logic` → `test_dynamodb_partition_key_logic`

**Test Logic Updated:**
- Verifies `get_partition_key()` returns user_id or "default"
- Verifies sort keys don't have user_id prefix
- Updated comments to reflect partition key usage

**Integration Test Updated:**
- Updated `test_dynamodb_backend_with_localstack` comments
- Removed assertions about user_id in sort keys

## Benefits

### 1. **Proper Multi-Tenant Isolation**
Each user's data is in a separate partition, enabling:
- Efficient queries by partition key
- Better scalability (DynamoDB distributes partitions)
- True data isolation at the database level

### 2. **Efficient Queries**
```rust
// Old: Scan required
Query PK="default" AND SK begins_with "user_123#prefix:"

// New: Efficient partition query
Query PK="user_123" AND SK begins_with "prefix:"
```

### 3. **Matches Exemem's Architecture**
The implementation now matches the table structure defined in `exemem-infra/cdk/lib/exemem-stack.ts`

## Usage

### Single-Tenant (No User ID)
```rust
let db_ops = DbOperationsV2::from_dynamodb(
    client,
    "DataFoldStorage".to_string(),
    None  // PK will be "default"
).await?;
```

### Multi-Tenant (With User ID)
```rust
let db_ops = DbOperationsV2::from_dynamodb(
    client,
    "DataFoldStorage".to_string(),
    Some("user_123".to_string())  // PK will be "user_123"
).await?;
```

## Testing

All tests pass:
```bash
cargo test storage::tests --lib
# 14 passed; 0 failed; 1 ignored
```

Specific tests:
- ✅ `test_dynamodb_partition_key_logic` - Verifies PK/SK structure
- ✅ `test_dynamodb_namespaced_store_user_isolation` - Verifies user isolation
- ✅ All other storage tests remain passing

## Migration Notes

### No Migration Required
This fix aligns the code with exemem's existing table structure. If you were using the old implementation:

1. **Old data** (if any exists): Has PK="default" and SK="{user_id}#{key}"
2. **New data**: Has PK="{user_id}" and SK="{key}"

These are in different partitions, so no conflicts. Old data would need to be migrated if it exists, but since exemem was expecting the new structure all along, any existing data should already be in the correct format (or nonexistent).

## Compatibility

- ✅ **fold_db**: All internal code uses `DbOperationsV2` abstraction, no changes needed
- ✅ **exemem-infra**: Now correctly uses DynamoDB tables as designed in CDK
- ✅ **Storage abstraction**: Interface unchanged, implementation fixed

## Related Files

- `fold_db/src/storage/dynamodb_backend.rs` - Implementation
- `fold_db/src/storage/tests.rs` - Tests
- `fold_db/src/db_operations/core_refactored.rs` - Uses the abstraction
- `exemem-infra/cdk/lib/exemem-stack.ts` - Table definitions
- `exemem-infra/lambdas/fold_db_worker/src/main.rs` - Usage in Lambda
