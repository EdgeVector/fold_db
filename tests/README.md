# DataFold Integration Tests

This directory contains integration tests for the DataFold database system.

## Tests

### HTTP API Integration Test

**File**: `integration_test_http.py`

A comprehensive integration test that validates the complete HTTP API workflow from server startup to data querying.

#### What It Tests

1. **Server Startup**: Automatically starts the HTTP server using `run_http_server.sh`
2. **Schema Loading**: Loads schemas from the `available_schemas/` directory via `/api/schemas/load`
3. **Schema Discovery**: Verifies all schemas from `available_schemas/` are discovered and accessible via `/api/schemas`
4. **Schema Approval**: Approves the BlogPost schema via `/api/schema/BlogPost/approve`
5. **Mutation Creation**: Creates a test blog post using the `/api/mutation` endpoint
6. **Data Query**: Queries the created data via `/api/query` and validates the results
7. **Cleanup**: Stops the server and cleans up all processes

#### Running the Test

```bash
# From the project root directory
python3 tests/integration_test_http.py
```

#### Expected Output

```
================================================================================
DataFold HTTP Server Integration Test
================================================================================
Date: 2025-09-30 14:49:13
Base URL: http://localhost:9001
================================================================================

✅ PASS: Load schemas
✅ PASS: Verify schemas discovered
✅ PASS: Approve schema
✅ PASS: Create mutation
✅ PASS: Query data

================================================================================
TEST SUMMARY
================================================================================
Total tests: 5
Passed: 5
Failed: 0
================================================================================
```

#### Test Details

**Mutation Format Used**:
```json
{
  "type": "mutation",
  "schema": "BlogPost",
  "mutation_type": "create",
  "fields_and_values": {
    "title": "Integration Test Blog Post",
    "content": "Test content...",
    "author": "Integration Test Suite",
    "publish_date": "2025-09-30T14:44:28Z",
    "tags": ["test", "integration", "automation"]
  },
  "key_value": {
    "hash": null,
    "range": "2025-09-30T14:44:28Z"
  }
}
```

**Query Format Used**:
```json
{
  "type": "query",
  "schema": "BlogPost",
  "fields": ["title", "author", "publish_date", "tags", "content"]
}
```

#### Exit Codes

- `0`: All tests passed
- `1`: One or more tests failed

#### Requirements

- Python 3.6 or higher
- Rust and Cargo (for building the server)
- `curl` command available in PATH
- BlogPost schema in `available_schemas/BlogPost.json`

## Unit Tests (Rust)

For Rust unit tests, use:

```bash
# Run all tests
cargo test --workspace

# Run with output
cargo test --workspace -- --nocapture

# Run specific test
cargo test test_name
```

## Linting

Run linting checks:

```bash
cargo clippy
```

Fix linting issues automatically where possible:

```bash
cargo clippy --fix
```

## Test Coverage

Generate test coverage report:

```bash
./scripts/generate_coverage.sh
```

## Adding New Tests

When adding new integration tests:

1. Follow the pattern in `integration_test_http.py`
2. Use clear test names and descriptions
3. Provide detailed error messages
4. Always clean up resources (servers, processes, files)
5. Document the test in this README
6. Ensure tests are idempotent (can be run multiple times)
7. Exit with appropriate exit codes (0 for success, non-zero for failure)

## Continuous Integration

These tests are designed to be run in CI/CD pipelines:

- Tests start and stop their own servers
- No manual setup required
- Clear exit codes for CI systems
- Detailed output for debugging failures
