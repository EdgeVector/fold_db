# DataFold Scripts and Examples

This directory contains example scripts, management tools, and integration tests for DataFold.

## Scripts

### Integration Tests

- **`../tests/integration_test_http.py`** - Comprehensive HTTP API integration test
  - Starts the HTTP server
  - Loads and approves schemas
  - Creates mutations
  - Queries and validates data
  - Cleans up resources
  - Usage: `python3 tests/integration_test_http.py`

### Data Management

- **`manage_blogposts.py`** - Blog post management via HTTP API
  - Creates sample blog posts using curl
  - Demonstrates Range schema mutations
  - Queries and displays blog posts
  - Usage: `python3 scripts/manage_blogposts.py`

- **`manage_user_activity.py`** - User activity tracking via HTTP API
  - Creates sample user activity data
  - Demonstrates HashRange schema usage
  - Usage: `python3 scripts/manage_user_activity.py`

### Utility Scripts

- **`cleanup_db_locks.sh`** - Clean up database lock files
- **`fix_tests.sh`** - Fix common test issues
- **`generate_coverage.sh`** - Generate test coverage reports
- **`install-hooks.sh`** - Install git hooks for pre-commit checks
- **`migrate_logging.py`** - Migrate logging configuration
- **`sync_ts_bindings.sh`** - Sync TypeScript bindings from Rust types to frontend
  - Generates TypeScript types from Rust using ts-rs
  - Copies generated types to frontend source directory
  - Ensures frontend types stay in sync with backend
  - Usage: `./scripts/sync_ts_bindings.sh`

## Integration Test

The HTTP integration test (`../tests/integration_test_http.py`) provides a comprehensive example of the complete DataFold workflow:

### Test Flow

1. **Start Server**: Automatically starts the HTTP server using `run_http_server.sh`
2. **Load Schemas**: Calls `/api/schemas/load` to load schemas from `available_schemas/`
3. **Approve Schema**: Approves the BlogPost schema via `/api/schema/BlogPost/approve`
4. **Create Mutation**: Creates a test blog post using `/api/mutation` with:
   ```json
   {
     "type": "mutation",
     "schema": "BlogPost",
     "mutation_type": "create",
     "fields_and_values": {
       "title": "Integration Test Blog Post",
       "author": "Integration Test Suite",
       "publish_date": "2025-09-30T14:43:28Z",
       "tags": ["test", "integration", "automation"]
     },
     "key_value": {
       "hash": null,
       "range": "2025-09-30T14:43:28Z"
     }
   }
   ```
5. **Query Data**: Queries the data back and validates it matches
6. **Cleanup**: Stops the server and cleans up processes

### Running the Integration Test

```bash
# From the project root directory
python3 tests/integration_test_http.py
```

The test will output detailed progress and a summary:
- ✅ PASS for successful tests
- ❌ FAIL with detailed error messages
- Final summary with pass/fail counts

## Example Schemas

### BlogPost Schema (Range Schema)

A simple blog post schema with time-based ordering:

- **Range Key**: `publish_date` - Orders posts by publication date
- **Fields**:
  - `title` - Blog post title
  - `content` - Post content
  - `author` - Author name
  - `publish_date` - Publication timestamp
  - `tags` - Array of tags

### UserActivity Schema (HashRange Schema)

Tracks user activities with efficient querying by user:

- **Hash Key**: `user_id` - Groups activities by user
- **Range Key**: `timestamp` - Orders activities by time
- **Fields**:
  - `user_id` - User identifier
  - `action` - Action performed
  - `resource` - Resource accessed
  - `timestamp` - Activity timestamp
  - `metadata` - Additional context

## HTTP API Endpoints

All scripts use the following HTTP API endpoints:

### Schema Management
- `POST /api/schemas/load` - Load schemas from directories
- `GET /api/schema/{name}` - Get schema details
- `POST /api/schema/{name}/approve` - Approve a schema
- `POST /api/schema/{name}/block` - Block a schema

### Data Operations
- `POST /api/mutation` - Create, update, or delete data
- `POST /api/query` - Query data

### Mutation Format

```json
{
  "type": "mutation",
  "schema": "SchemaName",
  "mutation_type": "create",  // or "update", "delete"
  "fields_and_values": {
    "field1": "value1",
    "field2": "value2"
  },
  "key_value": {
    "hash": "hash_value",      // null for Range schemas
    "range": "range_value"     // null for Single schemas
  }
}
```

### Query Format

```json
{
  "type": "query",
  "schema": "SchemaName",
  "fields": ["field1", "field2", "field3"],
  "filter": {  // Optional
    "hash": "hash_value",
    "range": "range_value"
  }
}
```

## Best Practices

1. **Always approve schemas** before creating mutations
2. **Use proper key_value format** matching your schema type (Single, Range, or HashRange)
3. **Include all required fields** in mutations
4. **Clean up test data** after running scripts
5. **Check server logs** (`server.log`) for debugging

## Development

To add a new integration test or management script:

1. Follow the pattern in `integration_test_http.py`
2. Use curl via subprocess for HTTP requests
3. Validate responses and provide clear error messages
4. Clean up resources (stop server, cleanup processes)
5. Add documentation to this README
