# API Reference

Fold DB provides multiple interfaces for interacting with the system: Frontend API Clients, CLI, HTTP REST API, and TCP protocol. This document provides comprehensive reference for all available operations.

## Table of Contents

1. [Frontend API Clients](#frontend-api-clients) ⭐ **Recommended for React Applications**
2. [CLI Interface](#cli-interface)
3. [HTTP REST API](#http-rest-api)
4. [TCP Protocol](#tcp-protocol)
5. [Request/Response Formats](#requestresponse-formats)
6. [Authentication](#authentication)
7. [Error Handling](#error-handling)

## Frontend API Clients

**🎯 This is the recommended approach for React applications.** The unified API client architecture provides type-safe, standardized access to all Datafold operations with built-in caching, error handling, and authentication.

### Quick Start

```typescript
import { schemaClient, securityClient, systemClient } from '../api/clients';

// Get all schemas with automatic caching and error handling
const response = await schemaClient.getSchemas();
if (response.success) {
  const schemas = response.data; // Fully typed
}
```

### Available Clients

#### Schema Client
**Purpose:** Schema management and SCHEMA-002 compliance
**File:** [`src/api/clients/schemaClient.ts`](src/datafold_node/static-react/src/api/clients/schemaClient.ts)

```typescript
import { schemaClient } from '../api/clients';

// Schema operations
await schemaClient.getSchemas();                    // List all schemas
await schemaClient.getSchema('users');              // Get specific schema
await schemaClient.getSchemasByState('approved');   // Filter by state
await schemaClient.getSchemaStatus();               // Get status overview

// State management (SCHEMA-002 compliance)
await schemaClient.approveSchema('users');          // Approve schema
await schemaClient.blockSchema('temp_data');        // Block schema
await schemaClient.loadSchema('users');             // Load into memory
await schemaClient.unloadSchema('temp_data');       // Unload from memory

// Validation
await schemaClient.validateSchemaForOperation('users', 'mutation');
```

#### Security Client
**Purpose:** Authentication, key management, and cryptographic operations
**File:** [`src/api/clients/securityClient.ts`](src/datafold_node/static-react/src/api/clients/securityClient.ts)

```typescript
import { securityClient } from '../api/clients';

// Message verification (cached for performance)
const verification = await securityClient.verifyMessage(signedMessage);

// Key management
await securityClient.registerPublicKey(keyRequest);
await securityClient.getSystemPublicKey();          // Cached for 1 hour
await securityClient.getSecurityStatus();

// Validation helpers
securityClient.validatePublicKeyFormat(publicKey);
securityClient.validateSignedMessage(signedMessage);
```

#### System Client
**Purpose:** System operations, logging, and database management
**File:** [`src/api/clients/systemClient.ts`](src/datafold_node/static-react/src/api/clients/systemClient.ts)

```typescript
import { systemClient } from '../api/clients';

// System monitoring
await systemClient.getSystemStatus();               // Health status (30s cache)
await systemClient.getLogs();                       // System logs

// Database operations (destructive - use with caution)
await systemClient.resetDatabase(true);             // Requires confirmation

// Real-time logging
systemClient.createLogStream(logEntry => {
  console.log('New log:', logEntry);
});
```

#### Transform Client
**Purpose:** Data transformation and queue management
**File:** [`src/api/clients/transformClient.ts`](src/datafold_node/static-react/src/api/clients/transformClient.ts)

```typescript
import { transformClient } from '../api/clients';

// Transform operations
await transformClient.getTransforms();              // List all transforms
await transformClient.getTransform('transform-123'); // Get specific transform
await transformClient.getQueue();                   // Queue status

// Queue management
await transformClient.addToQueue('transform-123');
await transformClient.removeFromQueue('transform-123');
```

#### Ingestion Client
**Purpose:** AI-powered data ingestion and schema generation
**File:** [`src/api/clients/ingestionClient.ts`](src/datafold_node/static-react/src/api/clients/ingestionClient.ts)

```typescript
import { ingestionClient } from '../api/clients';

// Ingestion operations
await ingestionClient.getStatus();                  // Service status
await ingestionClient.validateData(jsonData);       // Structure validation
await ingestionClient.processIngestion(data, {      // AI processing (60s timeout)
  autoExecute: true,
  trustDistance: 1,
  pubKey: 'user-key'
});

// OpenRouter AI configuration
await ingestionClient.getConfig();
await ingestionClient.saveConfig(openRouterConfig);
```

### Client Features

#### Automatic Caching
All clients include intelligent caching with operation-specific TTLs:
- **System Status**: 30 seconds
- **Schema Data**: 5 minutes
- **System Public Key**: 1 hour
- **Verification Results**: 5 minutes

```typescript
// Cache is automatic, but you can control it
const response = await schemaClient.getSchemas({
  cacheTtl: 60000,      // Custom cache duration
  cacheable: false      // Disable caching for this request
});

// Cache management
const stats = schemaClient.getCacheStats();
schemaClient.clearCache();
```

#### Error Handling
Comprehensive error handling with user-friendly messages:

```typescript
import {
  isNetworkError,
  isAuthenticationError,
  isSchemaStateError
} from '../api/core/errors';

try {
  const response = await schemaClient.getSchema('users');
} catch (error) {
  if (isAuthenticationError(error)) {
    redirectToLogin();
  } else if (isSchemaStateError(error)) {
    showMessage(`Schema "${error.schemaName}" is ${error.currentState}`);
  } else if (isNetworkError(error)) {
    showMessage('Network connection failed');
  } else {
    showMessage(error.toUserMessage());
  }
}
```

#### TypeScript Support
Full type safety with comprehensive interfaces:

```typescript
import type {
  SchemaData,
  SystemStatusResponse,
  Transform,
  EnhancedApiResponse
} from '../api/clients';

const handleSchemas = async (): Promise<SchemaData[]> => {
  const response: EnhancedApiResponse<SchemaData[]> = await schemaClient.getSchemas();
  
  if (response.success) {
    return response.data; // Fully typed as SchemaData[]
  }
  
  throw new Error('Failed to load schemas');
};
```

#### Batch Operations
Efficient batch processing for multiple operations:

```typescript
import { createApiClient } from '../api/core/client';

const client = createApiClient();
const responses = await client.batch([
  { id: 'schemas', method: 'GET', url: '/schemas' },
  { id: 'status', method: 'GET', url: '/system/status' },
  { id: 'transforms', method: 'GET', url: '/transforms' }
]);
```

#### Request Deduplication
Automatic deduplication prevents duplicate concurrent requests:

```typescript
// Multiple components calling simultaneously - only one HTTP request made
const [response1, response2] = await Promise.all([
  schemaClient.getSchemas(),
  schemaClient.getSchemas()  // Shares response from first request
]);
```

### Configuration

All clients use centralized configuration from [`constants/api.ts`](src/datafold_node/static-react/src/constants/api.ts):

```typescript
// Timeouts (operation-specific)
API_TIMEOUTS.QUICK         // 5s  - System status, basic gets
API_TIMEOUTS.STANDARD      // 8s  - Schema reads, transforms
API_TIMEOUTS.MUTATION      // 15s - Mutations, data changes
API_TIMEOUTS.AI_PROCESSING // 60s - AI operations

// Retries (operation-specific)
API_RETRIES.NONE          // 0 - Destructive operations
API_RETRIES.LIMITED       // 1 - State changes
API_RETRIES.STANDARD      // 2 - Most operations
API_RETRIES.CRITICAL      // 3 - System-critical calls

// Cache TTLs (data-specific)
API_CACHE_TTL.IMMEDIATE   // 30s - Frequently changing data
API_CACHE_TTL.STANDARD    // 5m  - Stable data
API_CACHE_TTL.LONG        // 1h  - Rarely changing data
```

### Migration from Direct fetch()

**Before (Direct fetch):**
```typescript
const response = await fetch('/api/schemas', {
  method: 'GET',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${token}`
  }
});

if (!response.ok) {
  throw new Error(`HTTP ${response.status}`);
}

const data = await response.json();
```

**After (Unified Client):**
```typescript
const response = await schemaClient.getSchemas();

if (response.success) {
  const schemas = response.data; // Fully typed
}
// Error handling, authentication, caching, and retries are automatic
```

### Documentation Links

- **[Architecture Documentation](docs/delivery/API-STD-1/api-client-architecture.md)**: Detailed technical architecture
- **[Developer Guide](docs/delivery/API-STD-1/developer-guide.md)**: Usage examples and best practices
- **[Source Code](src/datafold_node/static-react/src/api/clients/)**: Implementation details

---

## CLI Interface

### Installation

The CLI tool is built as part of the main build process:

```bash
cargo build --release --workspace
# Binary available at target/release/datafold_cli
```

### Global Options

```bash
datafold_cli [OPTIONS] <COMMAND>

OPTIONS:
    -c, --config <PATH>    Configuration file path [default: config/node_config.json]
    -h, --help            Print help information
    -V, --version         Print version information
```

### Schema Commands

#### load-schema
Load a schema definition into the node.

```bash
datafold_cli load-schema <SCHEMA_FILE>

ARGUMENTS:
    <SCHEMA_FILE>    Path to schema JSON file

EXAMPLES:
    datafold_cli load-schema schemas/user_profile.json
    datafold_cli load-schema -c custom_config.json schemas/analytics.json
```

#### list-schemas
List all loaded schemas in the node.

```bash
datafold_cli list-schemas [OPTIONS]

OPTIONS:
    --format <FORMAT>    Output format [default: table] [possible values: table, json, yaml]

EXAMPLES:
    datafold_cli list-schemas
    datafold_cli list-schemas --format json
```

#### get-schema
Get detailed information about a specific schema.

```bash
datafold_cli get-schema <SCHEMA_NAME>

ARGUMENTS:
    <SCHEMA_NAME>    Name of the schema to retrieve

EXAMPLES:
    datafold_cli get-schema UserProfile
    datafold_cli get-schema EventAnalytics --format json
```

### Simplified Schema Formats

FoldDB now supports simplified schema formats that reduce boilerplate by up to 90% while maintaining full backward compatibility.

#### Declarative Transform Schemas

**Simplified Format (Recommended):**
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

**Verbose Format (Legacy):**
```json
{
  "name": "BlogPostWordIndex",
  "schema_type": "HashRange",
  "key": {
    "hash_field": "BlogPost.map().content.split_by_word().map()",
    "range_field": "BlogPost.map().publish_date"
  },
  "fields": {
    "content": { "atom_uuid": "BlogPost.map().content" },
    "author": { "atom_uuid": "BlogPost.map().author" },
    "title": { "atom_uuid": "BlogPost.map().title" },
    "tags": { "atom_uuid": "BlogPost.map().tags" }
  }
}
```

#### Regular Schemas

**Simplified Format (Ultra-minimal):**
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

**Mixed Format (Best of both worlds):**
```json
{
  "name": "MixedSchema",
  "schema_type": "Single",
  "fields": {
    "simple_field": "Source.map().id",
    "complex_field": {
      "atom_uuid": "Source.map().metadata.tags",
      "field_type": "Single"
    },
    "empty_field": {}
  }
}
```

#### Format Support

- ✅ **String Expressions**: `"field": "Source.map().expression"`
- ✅ **Empty Objects**: `"field": {}` (uses defaults)
- ✅ **Mixed Formats**: Combine string and object formats
- ✅ **Backward Compatibility**: All existing schemas continue to work
- ✅ **Custom Deserialization**: Automatic conversion of string expressions

#### Migration

**From Verbose to Simplified:**
```bash
# Convert declarative transform schemas
sed -i 's/"atom_uuid": "\([^"]*\)"/\1/g' schema.json

# Convert regular schemas (replace verbose field definitions with {})
# Manual conversion recommended for complex schemas
```

**Examples:**
```bash
# Load simplified schema
datafold_cli load-schema schemas/simplified_user_profile.json

# List schemas (shows both formats)
datafold_cli list-schemas --format json
```

#### unload-schema
Unload a schema from the node.

```bash
datafold_cli unload-schema <SCHEMA_NAME>

ARGUMENTS:
    <SCHEMA_NAME>    Name of the schema to unload

EXAMPLES:
    datafold_cli unload-schema UserProfile
```

### Data Commands

#### query
Execute a query against a schema.

```bash
datafold_cli query [OPTIONS] --schema <SCHEMA>

OPTIONS:
    -s, --schema <SCHEMA>           Schema name
    -f, --fields <FIELDS>           Comma-separated list of fields
    -w, --where <FILTER>            Filter condition (JSON)
    -l, --limit <LIMIT>             Maximum number of results
    -o, --output <FORMAT>           Output format [default: table]

EXAMPLES:
    datafold_cli query --schema UserProfile --fields username,email
    datafold_cli query --schema UserProfile --fields username --where '{"username":"alice"}'
    datafold_cli query --schema EventAnalytics --fields event_name,metrics_by_timeframe --where '{"field":"metrics_by_timeframe","range_filter":{"KeyPrefix":"2024-01-01"}}'
```

#### mutate
Execute a mutation (create, update, delete) against a schema.

```bash
datafold_cli mutate [OPTIONS] --schema <SCHEMA> --operation <OPERATION>

OPTIONS:
    -s, --schema <SCHEMA>           Schema name
    -o, --operation <OPERATION>     Operation type [possible values: create, update, delete]
    -d, --data <DATA>               Data payload (JSON)
    -w, --where <FILTER>            Filter for update/delete operations (JSON)

EXAMPLES:
    # Create
    datafold_cli mutate --schema UserProfile --operation create --data '{"username":"bob","email":"bob@example.com"}'
    
    # Update
    datafold_cli mutate --schema UserProfile --operation update --where '{"username":"bob"}' --data '{"email":"newemail@example.com"}'
    
    # Delete
    datafold_cli mutate --schema UserProfile --operation delete --where '{"username":"bob"}'
```

### Network Commands

#### discover-nodes
Discover peers on the network.

```bash
datafold_cli discover-nodes [OPTIONS]

OPTIONS:
    --timeout <SECONDS>    Discovery timeout [default: 10]

EXAMPLES:
    datafold_cli discover-nodes
    datafold_cli discover-nodes --timeout 30
```

#### connect-node
Connect to a specific peer node.

```bash
datafold_cli connect-node <NODE_ID> <ADDRESS>

ARGUMENTS:
    <NODE_ID>     Peer node identifier
    <ADDRESS>     Peer address (multiaddr format)

EXAMPLES:
    datafold_cli connect-node 12D3KooWGK8YLjL... /ip4/192.168.1.100/tcp/9000
```

### Transform Commands

#### register-transform
Register a new transform function.

```bash
datafold_cli register-transform <TRANSFORM_FILE>

ARGUMENTS:
    <TRANSFORM_FILE>    Path to transform definition JSON file

EXAMPLES:
    datafold_cli register-transform transforms/user_status.json
```

#### list-transforms
List all registered transforms.

```bash
datafold_cli list-transforms [OPTIONS]

OPTIONS:
    --schema <SCHEMA>    Filter by schema name

EXAMPLES:
    datafold_cli list-transforms
    datafold_cli list-transforms --schema UserProfile
```

## HTTP REST API

### Base Configuration

**Default URL**: `http://localhost:9001`
**Content-Type**: `application/json` for all POST/PUT requests

### Schema Endpoints

#### POST /api/schema
Load a new schema into the node.

**Note**: Schemas are immutable once created. This endpoint creates new schemas only. To change schema structure, create a new schema with a different name.

**Request Body:**
```json
{
  "name": "SchemaName",
  "fields": {
    "field_name": {
      "field_type": "Single|Collection|Range",
      "permission_policy": {...},
      "payment_config": {...}
    }
  },
  "payment_config": {...}
}
```

**Response:**
```json
{
  "success": true,
  "message": "Schema loaded successfully",
  "schema_name": "SchemaName"
}
```

**Example:**
```bash
curl -X POST http://localhost:9001/api/schema \
  -H "Content-Type: application/json" \
  -d @schema.json
```

#### GET /api/schemas
List all loaded schemas.

**Response:**
```json
{
  "schemas": [
    {
      "name": "UserProfile",
      "fields": 5,
      "loaded_at": "2024-01-15T10:30:00Z"
    },
    {
      "name": "EventAnalytics", 
      "fields": 4,
      "loaded_at": "2024-01-15T11:00:00Z"
    }
  ]
}
```

#### GET /api/schema/{schema_name}
Get detailed information about a specific schema.

**Response:**
```json
{
  "name": "UserProfile",
  "fields": {
    "username": {
      "field_type": "Single",
      "permission_policy": {...},
      "payment_config": {...}
    }
  },
  "payment_config": {...},
  "loaded_at": "2024-01-15T10:30:00Z"
}
```

#### DELETE /api/schema/{schema_name}
Unload a schema from the node.

**Note**: This removes the schema from memory but does not delete any stored data. See [Schema Immutability](schema-management.md#schema-immutability) for details.

**Response:**
```json
{
  "success": true,
  "message": "Schema unloaded successfully"
}
```

### Data Endpoints

#### POST /api/execute
Execute a query or mutation operation.

**Request Body:**
```json
{
  "operation": "{\"type\":\"query|mutation\",\"schema\":\"SchemaName\",\"fields\":[...],\"filter\":{...}}"
}
```

**Query Example:**
```bash
curl -X POST http://localhost:9001/api/execute \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "{\"type\":\"query\",\"schema\":\"UserProfile\",\"fields\":[\"username\",\"email\"],\"filter\":{\"username\":\"alice\"}}"
  }'
```

**Mutation Example:**
```bash
curl -X POST http://localhost:9001/api/execute \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "{\"type\":\"mutation\",\"schema\":\"UserProfile\",\"operation\":\"create\",\"data\":{\"username\":\"bob\",\"email\":\"bob@example.com\"}}"
  }'
```

**Response:**
```json
{
  "results": [
    {
      "username": "alice",
      "email": "alice@example.com"
    }
  ],
  "errors": [],
  "metadata": {
    "execution_time_ms": 15,
    "rows_affected": 1
  }
}
```

#### POST /api/batch
Execute multiple operations in a single request.

**Request Body:**
```json
{
  "operations": [
    {
      "type": "query",
      "schema": "UserProfile",
      "fields": ["username"]
    },
    {
      "type": "mutation",
      "schema": "UserProfile",
      "operation": "create",
      "data": {"username": "charlie", "email": "charlie@example.com"}
    }
  ]
}
```

**Response:**
```json
{
  "results": [
    {
      "operation_index": 0,
      "results": [...],
      "errors": []
    },
    {
      "operation_index": 1, 
      "results": [...],
      "errors": []
    }
  ]
}
```

### Network Endpoints

#### POST /api/network/start
Initialize and start the networking layer.

**Request Body:**
```json
{
  "port": 9000,
  "enable_mdns": true,
  "bootstrap_peers": [
    "/ip4/192.168.1.100/tcp/9000/p2p/12D3KooWGK8YLjL..."
  ]
}
```

**Response:**
```json
{
  "success": true,
  "node_id": "12D3KooWABC123...",
  "listening_addresses": [
    "/ip4/192.168.1.50/tcp/9000"
  ]
}
```

#### POST /api/network/discover
Discover peers on the local network.

**Response:**
```json
{
  "peers": [
    {
      "node_id": "12D3KooWGK8YLjL...",
      "addresses": ["/ip4/192.168.1.100/tcp/9000"],
      "discovered_at": "2024-01-15T10:30:00Z"
    }
  ]
}
```

#### POST /api/network/connect
Connect to a specific peer node.

**Request Body:**
```json
{
  "node_id": "12D3KooWGK8YLjL...",
  "address": "/ip4/192.168.1.100/tcp/9000"
}
```

**Response:**
```json
{
  "success": true,
  "connected_at": "2024-01-15T10:35:00Z"
}
```

#### GET /api/network/status
Get current network status and connected peers.

**Response:**
```json
{
  "node_id": "12D3KooWABC123...",
  "listening_addresses": ["/ip4/192.168.1.50/tcp/9000"],
  "connected_peers": [
    {
      "node_id": "12D3KooWGK8YLjL...",
      "address": "/ip4/192.168.1.100/tcp/9000",
      "connected_at": "2024-01-15T10:35:00Z"
    }
  ],
  "network_active": true
}
```

#### POST /api/network/request-schema
Request a schema from a peer node.

**Request Body:**
```json
{
  "peer_id": "12D3KooWGK8YLjL...",
  "schema_name": "UserProfile"
}
```

### Transform Endpoints

#### POST /api/transform/register
Register a new transform function.

**Request Body:**
```json
{
  "name": "user_status_transform",
  "inputs": ["age"],
  "logic": "if age >= 18 { return \"adult\" } else { return \"minor\" }",
  "output": "UserProfile.status"
}
```

**Response:**
```json
{
  "success": true,
  "transform_id": "transform_123",
  "registered_at": "2024-01-15T10:40:00Z"
}
```

#### GET /api/transforms
List all registered transforms.

**Response:**
```json
{
  "transforms": [
    {
      "id": "transform_123",
      "name": "user_status_transform",
      "schema": "UserProfile",
      "output_field": "status",
      "registered_at": "2024-01-15T10:40:00Z"
    }
  ]
}
```

#### DELETE /api/transform/{transform_id}
Unregister a transform function.

**Response:**
```json
{
  "success": true,
  "message": "Transform unregistered successfully"
}
```

### System Endpoints

#### GET /api/health
Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:45:00Z",
  "services": {
    "database": "healthy",
    "network": "healthy",
    "transforms": "healthy"
  }
}
```

#### GET /api/status
Comprehensive system status.

**Response:**
```json
{
  "node_id": "12D3KooWABC123...",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "schemas_loaded": 3,
  "transforms_registered": 5,
  "connected_peers": 2,
  "storage": {
    "path": "data/db",
    "size_bytes": 1048576
  }
}
```

#### GET /api/metrics
System performance metrics.

**Response:**
```json
{
  "operations": {
    "queries_total": 1250,
    "mutations_total": 340,
    "avg_response_time_ms": 25
  },
  "resources": {
    "memory_usage_bytes": 67108864,
    "cpu_usage_percent": 15.5
  },
  "network": {
    "bytes_sent": 2048576,
    "bytes_received": 1536000
  }
}
```

#### POST /api/system/shutdown
Gracefully shutdown the node.

**Response:**
```json
{
  "success": true,
  "message": "Shutdown initiated"
}
```

### Log Endpoints

#### GET /api/logs/stream
Stream real-time logs (Server-Sent Events).

**Response:**
```
data: {"timestamp":"2024-01-15T10:50:00Z","level":"INFO","message":"Query executed successfully"}

data: {"timestamp":"2024-01-15T10:50:01Z","level":"DEBUG","message":"Transform triggered for field: age"}
```

#### POST /api/logs/features
Update log level for specific features.

**Request Body:**
```json
{
  "feature": "transform",
  "level": "TRACE"
}
```

**Response:**
```json
{
  "success": true,
  "feature": "transform",
  "new_level": "TRACE"
}
```

#### POST /api/logs/reload
Reload logging configuration.

**Response:**
```json
{
  "success": true,
  "message": "Logging configuration reloaded"
}
```

### Permission Endpoints

#### POST /api/permissions/trust-distance
Set trust distance for peers.

**Request Body:**
```json
{
  "default_distance": 1,
  "peer_distances": {
    "12D3KooWGK8YLjL...": 0,
    "12D3KooWABC123...": 2
  }
}
```

#### POST /api/permissions/explicit
Grant explicit permissions.

**Request Body:**
```json
{
  "schema": "UserProfile",
  "field": "email",
  "permission": "read",
  "public_key": "ed25519:ABC123...",
  "expires_at": "2024-12-31T23:59:59Z"
}
```

### Payment Endpoints

#### POST /api/payments/lightning/invoice
Generate Lightning Network invoice.

**Request Body:**
```json
{
  "amount_sats": 1000,
  "description": "Access to UserProfile.email field",
  "expiry": 3600
}
```

**Response:**
```json
{
  "payment_request": "lnbc10u1p...",
  "payment_hash": "abc123...",
  "expires_at": "2024-01-15T11:50:00Z"
}
```

#### POST /api/payments/verify
Verify payment for operation access.

**Request Body:**
```json
{
  "payment_hash": "abc123...",
  "operation": "query",
  "schema": "UserProfile",
  "fields": ["email"]
}
```

**Response:**
```json
{
  "verified": true,
  "access_granted": true,
  "expires_at": "2024-01-15T12:00:00Z"
}
```

## TCP Protocol

### Connection

**Default Port**: 9000
**Protocol**: Binary with length-prefixed JSON messages

### Message Format

All messages use the following binary format:
1. **Length Prefix**: 4 bytes (u32, little-endian) indicating JSON payload length
2. **JSON Payload**: UTF-8 encoded JSON message

### Request Format

```json
{
  "app_id": "client-application-name",
  "operation": "operation-type",
  "params": {
    // Operation-specific parameters
  },
  "signature": "optional-signature",
  "timestamp": 1234567890
}
```

### Response Format

```json
{
  "results": [...],
  "errors": [...],
  "metadata": {
    "execution_time_ms": 15
  }
}
```

### Supported Operations

#### list_schemas
List all loaded schemas.

**Request:**
```json
{
  "app_id": "my-app",
  "operation": "list_schemas",
  "params": {}
}
```

**Response:**
```json
{
  "results": [
    {"name": "UserProfile", "fields": 5},
    {"name": "EventAnalytics", "fields": 4}
  ],
  "errors": []
}
```

#### get_schema
Get schema details.

**Request:**
```json
{
  "app_id": "my-app",
  "operation": "get_schema",
  "params": {
    "schema_name": "UserProfile"
  }
}
```

#### create_schema
Load a new schema.

**Request:**
```json
{
  "app_id": "my-app",
  "operation": "create_schema",
  "params": {
    "schema": {
      "name": "UserProfile",
      "fields": {...}
    }
  }
}
```

#### query
Execute a query.

**Request:**
```json
{
  "app_id": "my-app",
  "operation": "query",
  "params": {
    "schema": "UserProfile",
    "fields": ["username", "email"],
    "filter": {
      "username": "alice"
    }
  }
}
```

#### mutation
Execute a mutation with universal key configuration support.

**Request:**
```json
{
  "app_id": "my-app",
  "operation": "mutation",
  "params": {
    "schema": "UserProfile",
    "mutation_type": "create",
    "data": {
      "username": "bob",
      "email": "bob@example.com"
    }
  }
}
```

**Universal Key Configuration Support:**

The mutation processor automatically extracts hash and range values from mutations using the schema's universal key configuration. Field names in mutations must match the schema's key configuration:

**HashRange Schema Example:**
```json
{
  "app_id": "my-app",
  "operation": "mutation",
  "params": {
    "schema": "BlogPostWordIndex",
    "mutation_type": "create",
    "data": {
      "word": "technology",           // hash_field from schema key
      "publish_date": "2025-01-15",  // range_field from schema key
      "content": "AI advances..."
    }
  }
}
```

**Range Schema Example:**
```json
{
  "app_id": "my-app",
  "operation": "mutation",
  "params": {
    "schema": "UserActivity",
    "mutation_type": "create",
    "data": {
      "timestamp": "2025-01-15T10:30:00Z",  // range_field from schema key
      "action": "login",
      "user_id": "user123"
    }
  }
}
```

**Error Handling:**
The mutation processor provides clear error messages for invalid configurations:

- `"HashRange schema 'SchemaName' requires key configuration"`
- `"HashRange schema mutation missing hash field 'field_name'"`
- `"Range schema mutation missing range field 'field_name'"`
- `"HashRange schema 'SchemaName' requires non-empty hash_field in key configuration"`

**Backward Compatibility:**
Legacy mutations continue to work without changes for existing schemas.

#### discover_nodes
Discover network peers.

**Request:**
```json
{
  "app_id": "my-app",
  "operation": "discover_nodes",
  "params": {
    "timeout_seconds": 10
  }
}
```

### Python Client Example

```python
import socket
import json
import struct

class FoldDBClient:
    def __init__(self, host='localhost', port=9000):
        self.host = host
        self.port = port
        self.sock = None
    
    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect((self.host, self.port))
    
    def disconnect(self):
        if self.sock:
            self.sock.close()
            self.sock = None
    
    def send_request(self, operation, params, app_id="python-client"):
        request = {
            "app_id": app_id,
            "operation": operation,
            "params": params
        }
        
        # Serialize and send
        request_json = json.dumps(request).encode('utf-8')
        self.sock.sendall(struct.pack('<I', len(request_json)))
        self.sock.sendall(request_json)
        
        # Receive response
        response_len = struct.unpack('<I', self.sock.recv(4))[0]
        response_json = self.sock.recv(response_len)
        
        return json.loads(response_json.decode('utf-8'))
    
    def query(self, schema, fields, filter_=None):
        params = {
            "schema": schema,
            "fields": fields
        }
        if filter_:
            params["filter"] = filter_
        
        return self.send_request("query", params)
    
    def create(self, schema, data):
        params = {
            "schema": schema,
            "mutation_type": "create",
            "data": data
        }
        return self.send_request("mutation", params)

# Usage example
client = FoldDBClient()
client.connect()

# Query users
result = client.query("UserProfile", ["username", "email"])
print(result)

# Create user
result = client.create("UserProfile", {
    "username": "alice",
    "email": "alice@example.com"
})
print(result)

client.disconnect()
```

## Request/Response Formats

### Query Request

```json
{
  "type": "query",
  "schema": "SchemaName",
  "fields": ["field1", "field2"],
  "filter": {
    "field": "field_name",
    "operator": "eq|gt|lt|gte|lte|ne",
    "value": "value"
  }
}
```

### Range Query Request

```json
{
  "type": "query",
  "schema": "SchemaName",
  "fields": ["field1", "range_field"],
  "filter": {
    "field": "range_field",
    "range_filter": {
      "Key": "specific_key" |
      "KeyPrefix": "prefix" |
      "KeyRange": {"start": "start_key", "end": "end_key"} |
      "Keys": ["key1", "key2"] |
      "KeyPattern": "pattern*" |
      "Value": "value"
    }
  }
}
```

### Mutation Request

```json
{
  "type": "mutation",
  "schema": "SchemaName",
  "operation": "create|update|delete",
  "data": {
    "field1": "value1",
    "field2": "value2"
  },
  "filter": {
    "field": "field_name",
    "value": "filter_value"
  }
}
```

### Standard Response

```json
{
  "results": [
    {
      "field1": "value1",
      "field2": "value2"
    }
  ],
  "errors": [
    {
      "code": "PERMISSION_DENIED",
      "message": "Insufficient permissions for field: email",
      "field": "email"
    }
  ],
  "metadata": {
    "execution_time_ms": 25,
    "rows_affected": 1,
    "total_fee_sats": 100
  }
}
```

## Authentication

### API Key Authentication (HTTP)

```bash
curl -H "Authorization: Bearer your-api-key" \
  http://localhost:9001/api/schemas
```

### Signature-Based Authentication (HTTP)

```bash
curl -H "X-Signature: ed25519:signature-hash" \
  -H "X-Public-Key: ed25519:public-key" \
  -H "X-Timestamp: 1609459200" \
  http://localhost:9001/api/schemas
```

### Public Key Authentication (TCP)

```json
{
  "app_id": "my-app",
  "operation": "query",
  "params": {...},
  "public_key": "ed25519:public-key",
  "signature": "ed25519:signature",
  "timestamp": 1609459200
}
```

## Error Handling

### Error Categories

#### Schema Errors
- `SCHEMA_NOT_FOUND`: Requested schema does not exist
- `SCHEMA_VALIDATION_FAILED`: Schema definition is invalid
- `SCHEMA_ALREADY_EXISTS`: Schema with same name already loaded

#### Permission Errors
- `PERMISSION_DENIED`: Insufficient permissions for operation
- `TRUST_DISTANCE_EXCEEDED`: Required trust distance not met
- `EXPLICIT_PERMISSION_REQUIRED`: Explicit permission needed

#### Payment Errors
- `PAYMENT_REQUIRED`: Payment needed for operation
- `INSUFFICIENT_PAYMENT`: Payment amount too low
- `PAYMENT_EXPIRED`: Payment has expired

#### Network Errors
- `PEER_NOT_FOUND`: Requested peer not available
- `CONNECTION_FAILED`: Failed to connect to peer
- `NETWORK_TIMEOUT`: Network operation timed out

#### Data Errors
- `FIELD_NOT_FOUND`: Requested field does not exist
- `INVALID_FILTER`: Filter syntax is invalid
- `TYPE_MISMATCH`: Data type does not match field type

### Error Response Format

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": {
      "field": "field_name",
      "expected": "expected_value",
      "actual": "actual_value"
    },
    "retry_after": 30
  }
}
```

### HTTP Status Codes

- `200 OK`: Request succeeded
- `400 Bad Request`: Invalid request format
- `401 Unauthorized`: Authentication required (currently disabled in development mode)
- `403 Forbidden`: Permission denied
- `404 Not Found`: Resource not found
- `402 Payment Required`: Payment needed
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Server error
- `503 Service Unavailable`: Service temporarily unavailable

---

**Next**: See [Deployment Guide](./deployment-guide.md) for deployment patterns and configuration.