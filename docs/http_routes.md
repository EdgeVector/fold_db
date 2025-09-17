# DataFold HTTP Server Routes Documentation

This document provides a complete reference for all HTTP routes available in the DataFold HTTP server. The server provides REST API endpoints for schemas, queries, mutations, ingestion, and system management, plus serves the React UI.

## Base Configuration

- **Base URL**: `http://localhost:9001` (default)
- **API Base Path**: `/api`
- **Static Files**: Served from root `/`
- **CORS**: Enabled for all origins, methods, and headers

---

## Data Types Reference

### Core Data Types

#### Schema Definition Structure
```typescript
interface Schema {
  name: string;                    // Schema identifier
  state: 'available' | 'approved' | 'blocked' | 'loading' | 'error';
  fields?: Record<string, SchemaField>;
  metadata?: {
    createdAt?: string;
    updatedAt?: string;
    version?: string;
    description?: string;
    tags?: string[];
  };
  rangeInfo?: {
    isRangeSchema: boolean;
    rangeField?: {
      name: string;
      type: string;
      minValue?: number;
      maxValue?: number;
    };
  };
}
```

#### Schema Field Structure
```typescript
interface SchemaField {
  field_type: 'Single' | 'Range';
  permission_policy: {
    read_policy: { NoRequirement: null } | { Distance: number };
    write_policy: { NoRequirement: null } | { Distance: number };
    explicit_read_policy?: { counts_by_pub_key: Record<string, number> };
    explicit_write_policy?: { counts_by_pub_key: Record<string, number> };
  };
  payment_config: {
    base_multiplier: number;
    trust_distance_scaling: { None: null } | {
      Linear: { slope: number; intercept: number; min_factor: number; }
    } | {
      Exponential: { base: number; scale: number; min_factor: number; }
    };
    min_payment?: number;
  };
  field_mappers: Record<string, any>;
}
```

#### Query Structure
```typescript
interface QueryRequest {
  type: 'query';
  schema: string;                  // Schema name
  fields: string[];               // Field names to retrieve
  filter?: {
    field?: string;               // Field to filter on
    range_filter?: {
      Key: string;                // Single key
    } | {
      KeyPrefix: string;          // Key prefix match
    } | {
      KeyRange: { start: string; end: string; }; // Key range
    } | {
      Keys: string[];             // Multiple specific keys
    } | {
      KeyPattern: string;         // Wildcard pattern (e.g., "store:*")
    } | {
      Value: string;              // Value match
    };
  };
}
```

#### Mutation Structure
```typescript
interface MutationRequest {
  type: 'mutation';
  schema: string;                 // Schema name
  mutation_type: 'create' | 'update' | 'delete';
  data: Record<string, any>;      // Data to mutate
  filter?: Record<string, any>;   // Filter for update/delete operations
}
```

#### Signed Message Structure (for mutations)
```typescript
interface SignedMessage {
  payload: string;                // Base64-encoded mutation JSON
  signature: string;              // Ed25519 signature
  public_key_id: string;          // Public key identifier
  timestamp: number;              // Unix timestamp
}
```

---

## Schema Management Routes

### List Schemas
- **Endpoint**: `GET /api/schemas`
- **Description**: List all schemas with their current states
- **Response**: JSON array with schema names and states
- **Example**: `GET /api/schemas`

### Get Schema Status (Unified)
- **Endpoint**: `GET /api/schemas/status`
- **Description**: Get comprehensive status of all schemas
- **Response**: Detailed schema status report
- **Example**: `GET /api/schemas/status`

### Refresh Schemas (Unified)
- **Endpoint**: `POST /api/schemas/refresh`
- **Description**: Refresh schemas from all sources
- **Response**: Refresh operation report
- **Example**: `POST /api/schemas/refresh`

### List Available Schemas
- **Endpoint**: `GET /api/schemas/available`
- **Description**: List all schemas in any state (available, approved, blocked)
- **Response Type**: `{ success: boolean; data: string[]; }`
- **Response Example**:
  ```json
  {
    "success": true,
    "data": ["UserProfile", "ProductCatalog", "EventAnalytics"]
  }
  ```

### Add Schema to Available
- **Endpoint**: `POST /api/schemas/available/add`
- **Description**: Add a new schema to the available_schemas directory
- **Request Body Type**: Complete schema definition with optional `custom_name`
- **Request Body Example**:
  ```json
  {
    "name": "CustomSchema",
    "custom_name": "MyCustomSchema",
    "fields": {
      "id": {
        "field_type": "Single",
        "permission_policy": {
          "read_policy": { "NoRequirement": null },
          "write_policy": { "Distance": 0 }
        },
        "payment_config": {
          "base_multiplier": 1.0,
          "trust_distance_scaling": { "None": null }
        },
        "field_mappers": {}
      }
    }
  }
  ```

### List Schemas by State
- **Endpoint**: `GET /api/schemas/by-state/{state}`
- **Description**: List schemas filtered by specific state
- **Path Parameters**:
  - `state`: `'available' | 'approved' | 'blocked'`
- **Response Type**: `{ success: boolean; data: Schema[]; }`
- **Example**: `GET /api/schemas/by-state/approved`

### Get Schema by Name
- **Endpoint**: `GET /api/schema/{name}`
- **Description**: Retrieve a specific schema by name
- **Path Parameters**:
  - `name`: string (Schema identifier)
- **Response Type**: `{ success: boolean; data: Schema; }`
- **Response Example**:
  ```json
  {
    "success": true,
    "data": {
      "name": "ProductCatalog",
      "state": "approved",
      "fields": { /* Schema fields */ },
      "schema_type": "Standard"
    }
  }
  ```

### Create Schema
- **Endpoint**: `POST /api/schema`
- **Description**: Create and load a new schema
- **Request Body Type**: Complete schema definition (see Schema Definition Structure above)
- **Response Type**: `{ success: boolean; data?: Schema; error?: string; }`

### Delete/Unload Schema
- **Endpoint**: `DELETE /api/schema/{name}`
- **Description**: Unload a schema (make it inactive)
- **Path Parameters**: 
  - `name`: Schema name
- **Example**: `DELETE /api/schema/ProductCatalog`

### Load Schema
- **Endpoint**: `POST /api/schema/{name}/load`
- **Description**: Load an existing but unloaded schema
- **Path Parameters**: 
  - `name`: Schema name
- **Example**: `POST /api/schema/ProductCatalog/load`

### Approve Schema
- **Endpoint**: `POST /api/schema/{name}/approve`
- **Description**: Approve a schema for queries and mutations
- **Path Parameters**: 
  - `name`: Schema name
- **Example**: `POST /api/schema/ProductCatalog/approve`

### Block Schema
- **Endpoint**: `POST /api/schema/{name}/block`
- **Description**: Block a schema from queries and mutations
- **Path Parameters**: 
  - `name`: Schema name
- **Example**: `POST /api/schema/ProductCatalog/block`

### Get Schema State
- **Endpoint**: `GET /api/schema/{name}/state`
- **Description**: Get the current state of a specific schema
- **Path Parameters**: 
  - `name`: Schema name
- **Response**: `{"schema": "name", "state": "approved|blocked|available"}`
- **Example**: `GET /api/schema/ProductCatalog/state`

---

## Query and Mutation Routes

### Execute Operation (Generic)
- **Endpoint**: `POST /api/execute`
- **Description**: Execute any operation (query or mutation)
- **Request Body Type**: `QueryRequest | MutationRequest | SignedMessage`
- **Response Type**: `{ success: boolean; data?: any; error?: string; }`

### Execute Query
- **Endpoint**: `POST /api/query`
- **Description**: Execute a query operation on approved schemas
- **Request Body Type**: `QueryRequest`
- **Authentication**: None required (uses "web-ui" as default pub_key)
- **Request Body Examples**:
  ```json
  {
    "type": "query",
    "schema": "ProductCatalog",
    "fields": ["name", "price", "category"],
    "filter": null
  }
  ```
  
  Range field query with specific key:
  ```json
  {
    "type": "query",
    "schema": "ProductCatalog",
    "fields": ["name", "inventory_by_location"],
    "filter": {
      "field": "inventory_by_location",
      "range_filter": {
        "Key": "warehouse:north"
      }
    }
  }
  ```
  
  Range field query with pattern matching:
  ```json
  {
    "type": "query",
    "schema": "ProductCatalog",
    "fields": ["name", "attributes"],
    "filter": {
      "field": "attributes",
      "range_filter": {
        "KeyPattern": "brand:*"
      }
    }
  }
  ```

### Execute Mutation
- **Endpoint**: `POST /api/mutation`
- **Description**: Execute a mutation operation
- **Request Body Type**: `MutationRequest`
- **Authentication**: None required (uses "web-ui" as default pub_key)
- **Request Body Structure**:
  ```json
  {
    "type": "mutation",
    "schema": "UserProfile",
    "mutation_type": "create",
    "data": {
      "username": "johndoe",
      "email": "john.doe@example.com",
      "full_name": "John Doe",
      "bio": "Software developer",
      "age": 35,
      "location": "San Francisco, CA"
    }
  }
  ```
- **Additional Examples**:
  
  Update mutation:
  ```json
  {
    "type": "mutation",
    "schema": "UserProfile",
    "mutation_type": "update",
    "filter": {
      "username": "johndoe"
    },
    "data": {
      "bio": "Senior software engineer",
      "location": "Austin, TX"
    }
  }
  ```
  
  Range field mutation:
  ```json
  {
    "type": "mutation",
    "schema": "ProductCatalog",
    "mutation_type": "create",
    "data": {
      "name": "Gaming Laptop",
      "category": "Electronics",
      "price": "1299.99",
      "inventory_by_location": {
        "warehouse:north": "25",
        "warehouse:south": "18",
        "store:downtown": "5"
      },
      "attributes": {
        "brand": "TechCorp",
        "model": "GX-2024",
        "cpu": "Intel i7-13700H",
        "warranty": "2 years"
      }
    }
  }
  ```

---

## Transform Routes

### List Transforms
- **Endpoint**: `GET /api/transforms`
- **Description**: List all available transforms
- **Response**: JSON object with transform IDs and details
- **Example**: `GET /api/transforms`

### Run Transform
- **Endpoint**: `POST /api/transform/{id}/run`
- **Description**: Execute a specific transform
- **Path Parameters**: 
  - `id`: Transform ID
- **Example**: `POST /api/transform/cleanup-users/run`

### Get Transform Queue
- **Endpoint**: `GET /api/transforms/queue`
- **Description**: Get information about the transform queue
- **Response**: Queue status and pending transforms
- **Example**: `GET /api/transforms/queue`

### Add to Transform Queue
- **Endpoint**: `POST /api/transforms/queue/{id}`
- **Description**: Add a transform to the processing queue
- **Path Parameters**: 
  - `id`: Transform ID
- **Example**: `POST /api/transforms/queue/cleanup-users`

---

## Ingestion Routes

### Process JSON Ingestion
- **Endpoint**: `POST /api/ingestion/process`
- **Description**: Process JSON data for ingestion into schemas
- **Request Body Type**:
  ```typescript
  {
    data: Record<string, any>;          // JSON data to ingest
    schema?: string;                    // Optional target schema
    validation_mode?: 'strict' | 'lenient'; // Validation mode
    transform?: {
      enabled: boolean;
      rules?: Array<{
        field: string;
        operation: 'map' | 'filter' | 'transform';
        params: Record<string, any>;
      }>;
    };
  }
  ```
- **Response Type**: `{ success: boolean; data?: { ingested_count: number; errors?: string[]; }; error?: string; }`

### Get Ingestion Status
- **Endpoint**: `GET /api/ingestion/status`
- **Description**: Get current status of the ingestion service
- **Response Type**:
  ```typescript
  {
    success: boolean;
    data: {
      status: 'active' | 'inactive' | 'error';
      configuration: {
        enabled: boolean;
        max_batch_size: number;
        timeout_ms: number;
      };
      stats: {
        total_processed: number;
        success_count: number;
        error_count: number;
        last_processed?: string; // ISO timestamp
      };
    };
  }
  ```

### Ingestion Health Check
- **Endpoint**: `GET /api/ingestion/health`
- **Description**: Health check for ingestion service
- **Response Type**: `{ status: 'healthy' | 'unhealthy' | 'unavailable'; details?: string; }`
- **Response Example**:
  ```json
  {
    "status": "healthy",
    "details": "All services operational"
  }
  ```

### Get Ingestion Config
- **Endpoint**: `GET /api/ingestion/config`
- **Description**: Get ingestion configuration (without sensitive data)
- **Response Type**:
  ```typescript
  {
    success: boolean;
    data: {
      enabled: boolean;
      max_file_size_mb: number;
      supported_formats: string[];
      validation_rules: {
        required_fields?: string[];
        field_types?: Record<string, string>;
      };
      processing_options: {
        batch_size: number;
        timeout_seconds: number;
        retry_attempts: number;
      };
    };
  }
  ```

### Validate JSON
- **Endpoint**: `POST /api/ingestion/validate`
- **Description**: Validate JSON data without processing
- **Request Body Type**: `{ data: Record<string, any>; schema?: string; }`
- **Response Type**:
  ```typescript
  {
    valid: boolean;
    message: string;
    errors?: Array<{
      field: string;
      issue: string;
      expected_type?: string;
      actual_type?: string;
    }>;
  }
  ```

### Get OpenRouter Config
- **Endpoint**: `GET /api/ingestion/openrouter-config`
- **Description**: Get OpenRouter API configuration
- **Response Type**:
  ```typescript
  {
    success: boolean;
    data: {
      model: string;
      api_key_configured: boolean;
      api_key_masked?: string; // e.g., "sk-***...***abc"
      rate_limits?: {
        requests_per_minute: number;
        tokens_per_minute: number;
      };
    };
  }
  ```

### Save OpenRouter Config
- **Endpoint**: `POST /api/ingestion/openrouter-config`
- **Description**: Save OpenRouter API configuration
- **Request Body Type**:
  ```typescript
  {
    api_key: string;                    // OpenRouter API key
    model: string;                      // Model identifier (e.g., "gpt-4")
    rate_limits?: {
      requests_per_minute?: number;
      tokens_per_minute?: number;
    };
  }
  ```
- **Response Type**: `{ success: boolean; message: string; }`

---

## Logging Routes

### List Logs
- **Endpoint**: `GET /api/logs`
- **Description**: Get current log entries (backward compatibility)
- **Response**: Array of log entries
- **Example**: `GET /api/logs`

### Stream Logs
- **Endpoint**: `GET /api/logs/stream`
- **Description**: Server-sent events stream of log entries
- **Response**: Event stream of log data
- **Example**: `GET /api/logs/stream`

### Get Log Config
- **Endpoint**: `GET /api/logs/config`
- **Description**: Get current logging configuration
- **Response**: Logging configuration details
- **Example**: `GET /api/logs/config`

### Reload Log Config
- **Endpoint**: `POST /api/logs/config/reload`
- **Description**: Reload logging configuration from file
- **Response**: Success/failure message
- **Example**: `POST /api/logs/config/reload`

### Get Log Features
- **Endpoint**: `GET /api/logs/features`
- **Description**: Get available log features and their current levels
- **Response**: Object with feature names and log levels
- **Example**: `GET /api/logs/features`

### Update Feature Log Level
- **Endpoint**: `PUT /api/logs/level`
- **Description**: Update log level for a specific feature at runtime
- **Body**: `{"feature": "schema", "level": "DEBUG"}`
- **Valid Levels**: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`
- **Example**: `PUT /api/logs/level`

---

## System Routes

### Get System Status
- **Endpoint**: `GET /api/system/status`
- **Description**: Get system status information
- **Response**: System uptime, version, and status
- **Example**: `GET /api/system/status`

### Reset Database
- **Endpoint**: `POST /api/system/reset-database`
- **Description**: Reset the database and restart the node (destructive)
- **Body**: `{"confirm": true}`
- **Warning**: This completely clears all data
- **Example**: `POST /api/system/reset-database`

---

## Security Routes

### Register System Public Key
- **Endpoint**: `POST /api/security/system-key`
- **Description**: Register the system-wide public key
- **Request Body Type**:
  ```typescript
  {
    public_key: string;                 // Base64-encoded Ed25519 public key (32 bytes)
    key_id: string;                    // Unique identifier for the key
    permissions?: string[];            // Array of permissions (e.g., ["read", "write", "admin"])
    metadata?: {
      description?: string;            // Human-readable description
      created_by?: string;            // Creator identifier
      expires_at?: string;            // ISO timestamp for expiration
      trust_level?: number;           // Trust level (0-10)
    };
  }
  ```
- **Request Body Example**:
  ```json
  {
    "public_key": "A1B2C3D4E5F6789012345678901234567890ABCDEF1234567890ABCDEF123456",
    "key_id": "system-main-key",
    "permissions": ["read", "write", "admin"],
    "metadata": {
      "description": "Main system public key for mutations",
      "created_by": "system-admin",
      "trust_level": 0
    }
  }
  ```
- **Response Type**: `{ success: boolean; data?: { key_id: string; registered_at: string; }; error?: string; }`

### Get System Public Key
- **Endpoint**: `GET /api/security/system-key`
- **Description**: Get the system public key information
- **Response Type**:
  ```typescript
  {
    success: boolean;
    data?: {
      public_key: string;              // Base64-encoded public key
      key_id: string;                 // Key identifier
      permissions: string[];          // Associated permissions
      registered_at: string;          // ISO timestamp
      metadata?: {
        description?: string;
        trust_level?: number;
        expires_at?: string;
      };
    };
    error?: string;                    // If key not found: "System public key not registered"
  }
  ```

### Remove System Public Key
- **Endpoint**: `DELETE /api/security/system-key`
- **Description**: Remove the system public key
- **Response Type**: `{ success: boolean; message?: string; error?: string; }`

### Verify Message
- **Endpoint**: `POST /api/security/verify`
- **Description**: Verify a signed message (for testing)
- **Request Body Type**:
  ```typescript
  {
    payload: string;                   // Base64-encoded message payload
    signature: string;                 // Hex-encoded Ed25519 signature
    public_key_id: string;            // Key identifier to verify against
    timestamp: number;                // Unix timestamp from message
  }
  ```
- **Response Type**:
  ```typescript
  {
    success: boolean;
    data?: {
      valid: boolean;                  // Whether signature is valid
      public_key_found: boolean;       // Whether public key was found
      timestamp_valid: boolean;        // Whether timestamp is within tolerance
      details: string;                // Verification details
    };
    error?: string;
  }
  ```

### Get Security Status
- **Endpoint**: `GET /api/security/status`
- **Description**: Get security configuration status
- **Response Type**:
  ```typescript
  {
    success: boolean;
    data: {
      system_key_registered: boolean;
      signature_verification_enabled: boolean;
      timestamp_tolerance_seconds: number;
      registered_keys_count: number;
      security_level: 'low' | 'medium' | 'high';
      features: {
        ed25519_signing: boolean;
        message_authentication: boolean;
        permission_checking: boolean;
      };
    };
  }
  ```

### Get Client Examples
- **Endpoint**: `GET /api/security/examples`
- **Description**: Get code examples for client integration
- **Response Type**:
  ```typescript
  {
    success: boolean;
    data: {
      examples: Record<string, {
        language: string;              // e.g., "javascript", "python", "rust"
        code: string;                 // Complete code example
        description: string;          // What the example demonstrates
      }>;
      common_patterns: {
        key_generation: string;
        message_signing: string;
        api_request: string;
      };
    };
  }
  ```

### Generate Demo Keypair
- **Endpoint**: `GET /api/security/demo-keypair`
- **Description**: Generate a demo keypair (development only)
- **Response Type**:
  ```typescript
  {
    success: boolean;
    data: {
      public_key: string;              // Base64-encoded public key
      secret_key: string;              // Base64-encoded secret key
      key_id: string;                 // Generated key identifier
      warning: string;                // Security warning message
    };
    error?: string;
  }
  ```
- **Warning**: For development/testing only. Never use in production.

### Protected Endpoint (Example)
- **Endpoint**: `POST /api/security/protected`
- **Description**: Example of a protected endpoint requiring signature
- **Request Body Type**: `SignedMessage` (see Signed Message Structure above)
- **Authentication**: Requires valid signature and 'read' permission
- **Response Type**: `{ success: boolean; data?: { message: string; authenticated_as: string; }; error?: string; }`

### Common Registration Issues

#### Public Key Format
- Must be exactly 32 bytes when base64-decoded
- Use standard base64 encoding (not URL-safe)
- Example: `"A1B2C3D4E5F6789012345678901234567890ABCDEF1234567890ABCDEF123456"`

#### Key Generation Example (JavaScript)
```javascript
// Using @noble/ed25519 library
import { getPublicKey, getSecretKey } from '@noble/ed25519';

const secretKey = getSecretKey(); // Generates random 32-byte secret
const publicKey = getPublicKey(secretKey); // Derives public key

const registrationPayload = {
  public_key: Buffer.from(publicKey).toString('base64'),
  key_id: 'my-app-key',
  permissions: ['read', 'write']
};
```

---

## Network Routes

### Initialize Network
- **Endpoint**: `POST /api/network/init`
- **Description**: Initialize network with configuration
- **Request Body Type**:
  ```typescript
  {
    listen_address: string;             // e.g., "/ip4/127.0.0.1/tcp/0"
    discovery_port?: number;            // UDP port for peer discovery
    max_connections?: number;           // Maximum peer connections
    enable_discovery?: boolean;         // Enable automatic peer discovery
    bootstrap_nodes?: string[];         // Initial peers to connect to
    network_id?: string;               // Network identifier
  }
  ```
- **Request Body Example**:
  ```json
  {
    "listen_address": "/ip4/127.0.0.1/tcp/0",
    "discovery_port": 1234,
    "max_connections": 50,
    "enable_discovery": true,
    "bootstrap_nodes": [
      "/ip4/192.168.1.100/tcp/9002",
      "/ip4/192.168.1.101/tcp/9002"
    ]
  }
  ```
- **Response Type**: `{ success: boolean; data?: { network_id: string; listen_address: string; }; error?: string; }`

### Start Network
- **Endpoint**: `POST /api/network/start`
- **Description**: Start the network service
- **Prerequisite**: Network must be initialized first
- **Response Type**: `{ success: boolean; data?: { status: 'started'; peer_id: string; }; error?: string; }`

### Stop Network
- **Endpoint**: `POST /api/network/stop`
- **Description**: Stop the network service
- **Response Type**: `{ success: boolean; data?: { status: 'stopped'; }; error?: string; }`

### Get Network Status
- **Endpoint**: `GET /api/network/status`
- **Description**: Get current network status
- **Response Type**:
  ```typescript
  {
    success: boolean;
    data: {
      status: 'inactive' | 'initializing' | 'active' | 'error';
      peer_id?: string;
      listen_addresses?: string[];
      connected_peers: Array<{
        peer_id: string;
        address: string;
        connection_duration: number; // seconds
        last_seen: string;          // ISO timestamp
      }>;
      discovery: {
        enabled: boolean;
        port?: number;
        discovered_peers: number;
      };
      stats: {
        total_connections: number;
        active_connections: number;
        bytes_sent: number;
        bytes_received: number;
      };
    };
  }
  ```

### Connect to Node
- **Endpoint**: `POST /api/network/connect`
- **Description**: Connect to a specific node
- **Request Body Type**: `{ node_id: string; address?: string; }`
- **Request Body Example**:
  ```json
  {
    "node_id": "12D3KooWBmwkafWE2fqfzS96VoTwfZjAcXw9coMf5E4dEcf2fhA3",
    "address": "/ip4/192.168.1.100/tcp/9002"
  }
  ```
- **Response Type**: `{ success: boolean; data?: { connection_status: 'connected' | 'failed'; }; error?: string; }`

### Discover Nodes
- **Endpoint**: `POST /api/network/discover`
- **Description**: Discover available nodes on the network
- **Request Body Type**: `{ timeout_seconds?: number; network_id?: string; }`
- **Response Type**:
  ```typescript
  {
    success: boolean;
    data: {
      discovered_nodes: Array<{
        peer_id: string;
        addresses: string[];
        distance?: number;          // Network distance/hops
        capabilities?: string[];    // Supported protocols
        last_seen: string;         // ISO timestamp
      }>;
      discovery_duration_ms: number;
    };
  }
  ```

### List Known Nodes
- **Endpoint**: `GET /api/network/nodes`
- **Description**: List all known nodes
- **Response Type**:
  ```typescript
  {
    success: boolean;
    data: {
      nodes: Array<{
        peer_id: string;
        addresses: string[];
        status: 'connected' | 'disconnected' | 'connecting';
        first_seen: string;        // ISO timestamp
        last_seen: string;         // ISO timestamp
        connection_attempts: number;
        metadata?: {
          version?: string;
          node_type?: string;
          capabilities?: string[];
        };
      }>;
      total_count: number;
    };
  }
  ```

---

## Static File Serving

### React UI
- **Endpoint**: `GET /` (and all unmatched routes)
- **Description**: Serves the built React UI
- **File Location**: `src/datafold_node/static-react/dist`
- **Index File**: `index.html`
- **Fallback**: All routes not matching API endpoints serve the React app

---

## Error Handling

### Standard Response Format
All API endpoints follow a consistent response format:

```typescript
interface ApiResponse<T = any> {
  success: boolean;
  data?: T;                           // Present on successful responses
  error?: string;                     // Present on error responses
  timestamp?: number;                 // Unix timestamp
  request_id?: string;                // For request tracking
}
```

### Success Response Examples
```json
{
  "success": true,
  "data": {
    "schemas": ["UserProfile", "ProductCatalog"]
  },
  "timestamp": 1672531200
}
```

### Error Response Examples
```json
{
  "success": false,
  "error": "Schema 'InvalidSchema' not found",
  "timestamp": 1672531200,
  "request_id": "req_abc123"
}
```

### Detailed Error Responses
For validation errors and complex failures:

```typescript
interface DetailedErrorResponse {
  success: false;
  error: string;                      // Main error message
  details?: {
    code: string;                     // Error code (e.g., "SCHEMA_NOT_FOUND")
    field?: string;                   // Field that caused the error
    expected?: string;                // Expected value/type
    actual?: string;                  // Actual value/type received
    suggestions?: string[];           // Possible fixes
  };
  validation_errors?: Array<{
    field: string;
    message: string;
    code: string;
  }>;
}
```

### HTTP Status Codes
- **200 OK**: Successful operation
- **201 Created**: Resource successfully created
- **400 Bad Request**: Invalid input data or malformed request
  - Invalid JSON syntax
  - Missing required fields
  - Invalid field types
  - Invalid query/mutation structure
- **401 Unauthorized**: Authentication required or failed
  - Missing signature for mutation operations
  - Invalid Ed25519 signature
  - Expired timestamp
- **403 Forbidden**: Operation not permitted
  - Insufficient permissions for requested operation
  - Schema in wrong state for operation
  - Public key not authorized
- **404 Not Found**: Requested resource doesn't exist
  - Schema not found
  - Endpoint not available
  - Public key not registered
- **409 Conflict**: Resource conflict
  - Schema already exists
  - Concurrent modification detected
- **422 Unprocessable Entity**: Valid request format but logical errors
  - Schema validation failed
  - Business rule violations
  - Invalid state transition
- **500 Internal Server Error**: Unexpected server error
  - Database connection failures
  - Internal processing errors
  - System configuration issues
- **503 Service Unavailable**: Service temporarily unavailable
  - Network service not initialized
  - Ingestion service disabled
  - Database not accessible

### Error Codes Reference
Common error codes returned in the `details.code` field:

#### Schema Errors
- `SCHEMA_NOT_FOUND`: Requested schema doesn't exist
- `SCHEMA_INVALID_STATE`: Schema state doesn't allow the operation
- `SCHEMA_VALIDATION_FAILED`: Schema structure validation failed
- `SCHEMA_ALREADY_EXISTS`: Attempted to create duplicate schema

#### Authentication/Security Errors
- `PERMISSION_DENIED`: Insufficient permissions for operation
- Note: Authentication is currently disabled for all endpoints

#### Network Errors
- `NETWORK_NOT_INITIALIZED`: Network service must be initialized first
- `PEER_CONNECTION_FAILED`: Failed to connect to specified peer
- `DISCOVERY_TIMEOUT`: Peer discovery operation timed out

#### Ingestion Errors
- `INGESTION_DISABLED`: Ingestion service not enabled
- `VALIDATION_FAILED`: Data validation against schema failed
- `PROCESSING_ERROR`: Error processing ingestion data

---

## Authentication & Security

### Query Operations
- **Authentication**: None required
- **Default Identity**: `web-ui` with `trust_distance: 0`

### Mutation Operations
- **Authentication**: None required (uses "web-ui" as default pub_key)
- **Process**: 
  1. Mutation request sent directly as JSON
  2. Server processes with mock verification
  3. Uses "web-ui" identity for all operations

### All Endpoints
- **Authentication**: None required for any endpoint
- **Default Identity**: All operations use "web-ui" with `trust_distance: 0`
- **Development Mode**: Simplified authentication for development and testing

---

## Development Notes

### CORS Configuration
- Allows all origins, methods, and headers
- Max age: 3600 seconds
- Suitable for development; restrict in production

### JSON Configuration
- Custom error handler for JSON parsing errors
- Detailed error messages for malformed requests

### Logging Integration
- All routes include feature-specific logging
- Log levels configurable at runtime
- Request/response logging available

This documentation should help you align your UI routes with the available backend endpoints. Each route includes the HTTP method, path, description, and example usage.