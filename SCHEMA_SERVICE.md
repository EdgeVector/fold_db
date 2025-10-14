# Schema Service

## Overview

The schema service is a standalone HTTP service that provides schema discovery and management functionality. It runs independently from the main DataFold node and serves schemas via HTTP API.

## Architecture

### Components

1. **Schema Service** (`src/schema_service/`)
   - HTTP server running on port 9002 (default)
   - Reads schemas from the `available_schemas` directory
   - Provides REST API endpoints for schema retrieval

2. **Schema Service Client** (`src/datafold_node/schema_client.rs`)
   - HTTP client for communicating with the schema service
   - Used by DataFold node to fetch schemas

3. **Binary Entry Point** (`src/bin/schema_service.rs`)
   - Standalone binary for running the schema service
   - Configurable port and schema directory

## API Endpoints

### GET `/api/health`
Health check endpoint

**Response:**
```json
{
  "status": "healthy"
}
```

### GET `/api/schemas`
List all available schemas

**Response:**
```json
{
  "schemas": ["User", "Product", "Order", ...]
}
```

### GET `/api/schema/{name}`
Get a specific schema by name

**Response:**
```json
{
  "name": "User",
  "definition": { /* schema definition */ }
}
```

### POST `/api/schemas/reload`
Reload schemas from the directory

**Response:**
```json
{
  "success": true,
  "schemas_loaded": 10
}
```

## Configuration

### Schema Service

Command-line options:
```bash
cargo run --bin schema_service -- --port 9002 --schemas-dir available_schemas
```

### DataFold Node

The node can be configured to use the schema service via command-line or config:

Command-line:
```bash
cargo run --bin datafold_http_server -- --port 9001 --schema-service-url "http://127.0.0.1:9002"
```

Config file (`config/node_config.json`):
```json
{
  "storage_path": "data",
  "default_trust_distance": 1,
  "schema_service_url": "http://127.0.0.1:9002"
}
```

## Startup Flow

The `run_http_server.sh` script handles the startup sequence:

1. Build Rust backend
2. Generate OpenAPI spec
3. Build React frontend
4. **Start schema service** on port 9002
5. Wait for schema service to be ready
6. **Start main HTTP server** on port 9001 with schema service URL
7. HTTP server loads schemas from schema service on startup

## Benefits

- **Separation of Concerns**: Schema management is isolated from the main node
- **Centralized Schema Discovery**: Single source of truth for available schemas
- **Flexible Deployment**: Schema service can run on a different machine/container
- **Hot Reload**: Schemas can be reloaded without restarting the main node

## Ports

- Schema Service: **9002** (default)
- Main HTTP Server: **9001** (default)
- P2P Network: **9000** (default)

## Testing

All existing tests pass with the new schema service implementation. The node creation tests have been updated to include the `schema_service_url` configuration field.

