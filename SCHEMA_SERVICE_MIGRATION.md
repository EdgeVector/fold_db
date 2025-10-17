# Schema Service Database Migration

## Summary

The schema service has been successfully migrated from file-based storage to a sled database for persistent schema storage. This provides improved durability, better performance, and a more robust storage solution.

## Changes Made

### 1. Core Schema Service (`src/schema_service/server.rs`)

**Before:**
- Schemas were stored in JSON files in a directory (default: `available_schemas`)
- `load_schemas()` read files from disk
- `add_schema()` wrote new schemas to JSON files

**After:**
- Schemas are stored in a sled database (default: `schema_registry`)
- `load_schemas()` reads from the sled database
- `add_schema()` writes to the sled database with automatic flushing
- Database operations use `serde_json` for serialization/deserialization

**Key Updates:**
```rust
pub struct SchemaServiceState {
    schemas: Arc<Mutex<HashMap<String, Value>>>,
    db: sled::Db,                    // Added
    schemas_tree: sled::Tree,        // Added
}
```

### 2. CLI Binary (`src/bin/schema_service.rs`)

**Before:**
```bash
--schemas-dir available_schemas
```

**After:**
```bash
--db-path schema_registry
```

The binary now accepts a database path instead of a directory path.

### 3. Tests Updated

All tests have been updated to use temporary sled databases:

- `src/schema_service/server.rs` - Unit tests
- `tests/schema_service_closeness_test.rs` - Integration tests (14 tests)
- `src/datafold_node/schema_client.rs` - Client tests

Tests now use temporary directories with database paths like:
```rust
let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();
let state = SchemaServiceState::new(db_path)?;
```

### 4. Startup Script (`run_http_server.sh`)

Updated to use the new database path argument:
```bash
# Before
cargo run --bin schema_service -- --port 9002 --schemas-dir available_schemas

# After
cargo run --bin schema_service -- --port 9002 --db-path schema_registry
```

### 5. Documentation (`SCHEMA_SERVICE.md`)

- Updated architecture description
- Added new POST endpoint documentation for adding schemas
- Added migration instructions
- Updated configuration examples
- Added benefits of database storage

### 6. Migration Utility (`scripts/migrate_schemas_to_db.py`)

A new Python script to migrate existing JSON schema files to the database:

**Features:**
- Reads all `.json` files from a directory
- Posts them to the schema service via HTTP API
- Handles success, similarity conflicts, and errors
- Provides a detailed summary report

**Usage:**
```bash
# Start the schema service
cargo run --bin schema_service &

# Run migration
python3 scripts/migrate_schemas_to_db.py --schemas-dir available_schemas
```

## Benefits

1. **Persistent Storage**: Schemas are durably stored in a sled database
2. **Better Performance**: Database operations are faster than file I/O
3. **ACID Properties**: Sled provides atomic operations and crash recovery
4. **Consistency**: All database writes are immediately flushed
5. **Schema Validation**: Existing similarity detection and field mapper logic preserved
6. **Easy Migration**: Migration script handles conversion from JSON files

## Backwards Compatibility

The schema service API remains **unchanged**:
- All HTTP endpoints work the same way
- Schema format is identical
- Clients don't need any modifications

The only visible change is the CLI argument from `--schemas-dir` to `--db-path`.

## Database Structure

The sled database uses a simple key-value structure:

```
Tree: "schemas"
├─ "SchemaName1" → {serialized JSON schema}
├─ "SchemaName2" → {serialized JSON schema}
└─ "SchemaName3" → {serialized JSON schema}
```

Each schema is stored as a JSON-serialized blob with the schema name as the key.

## Testing

All tests pass successfully:
- ✅ 5 schema service unit tests
- ✅ 14 schema service closeness tests
- ✅ 2 schema client tests
- ✅ All 225 project tests pass
- ✅ No clippy warnings
- ✅ No linter errors

## Migration Steps for Users

If you're upgrading from the file-based system:

1. **Start the new schema service:**
   ```bash
   cargo run --bin schema_service -- --db-path schema_registry
   ```

2. **Migrate existing schemas:**
   ```bash
   python3 scripts/migrate_schemas_to_db.py --schemas-dir available_schemas
   ```

3. **Verify migration:**
   ```bash
   curl http://127.0.0.1:9002/api/schemas
   ```

4. **Update any custom scripts** that reference `--schemas-dir` to use `--db-path`

## Rollback

If you need to roll back:

1. The old JSON files in `available_schemas` are not deleted by the migration
2. Simply use an older version of the schema service binary
3. Or export schemas from the database back to JSON files using the API

## Future Improvements

Possible enhancements:
- Add export endpoint to dump all schemas to JSON
- Add backup/restore functionality
- Add database compaction utilities
- Support for schema versioning in the database

