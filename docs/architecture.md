# Fold DB Architecture

## Overview

Fold DB is designed as a distributed, event-driven data platform built around atomic data storage units called "atoms". The architecture supports schema-driven data management, programmable transforms, peer-to-peer networking, and fine-grained access control.

## Core Architectural Principles

### 1. Event-Driven Design
- Field value changes trigger automatic transform execution
- Direct event processing without complex correlation patterns
- Real-time data processing and computation

### 2. Atom-Based Storage
- Immutable storage units with version history
- UUID-based identification for global uniqueness
- Atomic operations with consistency guarantees

### 3. Schema-First Approach
- Strict schema validation for all data operations
- Schema immutability ensures data consistency and integrity
- Field-level permission and payment configuration

### 4. Distributed by Design
- Peer-to-peer networking for node discovery
- Trust-based access control between nodes
- Schema synchronization across the network

## System Components

### Core Database (FoldDB)

The central database component that manages all data operations:

```rust
pub struct FoldDB {
    db_ops: Arc<DbOperations>,
    managers: Managers,
    orchestration: OrchestrationLayer,
}
```

**Key Responsibilities:**
- Atom lifecycle management (create, read, update, delete)
- Schema validation and enforcement
- Transform orchestration and execution
- Permission checking and fee calculation

**Storage Engine:**
- Uses Sled embedded database for persistence
- Atomic operations with ACID guarantees
- Efficient key-value storage with range queries

### Schema Management System

Schemas define the structure and behavior of data:

```json
{
  "name": "SchemaName",
  "fields": {
    "field_name": {
      "field_type": "Single|Collection|Range",
      "permission_policy": {
        "read_policy": {"Distance": 0},
        "write_policy": {"Distance": 1}
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "min_payment": 100
      },
      "transform": {
        "inputs": ["input_field"],
        "logic": "computation logic",
        "output": "output_field"
      }
    }
  }
}
```

**Field Types:**
- **Single**: Simple scalar values (string, number, boolean)
- **Collection**: Arrays of values
- **Range**: Key-value pairs for hierarchical data

### Transform Engine

Programmable computation system with domain-specific language:

```
// Transform DSL Example
when UserProfile.age changes {
    if age >= 18 {
        UserProfile.status = "adult"
    } else {
        UserProfile.status = "minor"
    }
}
```

**Features:**
- Event-driven execution on field changes
- Built-in functions and operators
- Type safety and validation
- Dependency resolution between transforms

### Permission System

Multi-layered access control:

```rust
pub enum PermissionPolicy {
    NoRequirement,
    Distance(u32),        // Trust distance requirement
    PublicKey(String),    // Specific public key
    Explicit(String),     // Explicit permission
}
```

**Trust Distance Model:**
- Distance 0: Local node only
- Distance 1: Direct trusted peers
- Distance 2+: Extended trust network

### Payment System

Configurable fee structure for data access:

```rust
pub struct PaymentConfig {
    base_multiplier: f64,
    trust_distance_scaling: Option<TrustDistanceScaling>,
    min_payment: Option<u64>,
}
```

**Scaling Models:**
- **Linear**: `fee = base * (slope * distance + intercept)`
- **Exponential**: `fee = base * (base^distance * scale)`
- **None**: Flat fee regardless of distance

### Network Layer

Peer-to-peer networking built on libp2p:

```rust
pub struct NetworkCore {
    swarm: Swarm<NetworkBehaviour>,
    schema_service: SchemaService,
    peer_discovery: PeerDiscovery,
}
```

**Network Protocols:**
- **mDNS**: Local network discovery
- **Noise**: Encrypted communication
- **Yamux**: Stream multiplexing
- **Request-Response**: Schema synchronization

## Data Flow Architecture

### 1. Ingestion Pipeline

```
External Data → Schema Validation → Atom Creation → Event Processing → Storage
```

**Steps:**
1. Data arrives via API (HTTP/TCP/CLI)
2. Schema validation against loaded schema
3. Atom creation with UUID and metadata
4. Field processing and transform triggering
5. Persistence to storage engine

### 2. Query Processing

```
Query Request → Permission Check → Schema Resolution → Data Retrieval → Response
```

**Query Types:**
- **Field Selection**: Specific fields from atoms
- **Filtering**: Field-based predicates
- **Range Queries**: Key-based filtering for Range fields
- **Cross-Schema**: Queries spanning multiple schemas

### 3. Transform Execution

```
Field Change → Event Detection → Transform Resolution → Execution → Result Storage
```

**Execution Flow:**
1. Field value change detected
2. Related transforms identified
3. Transform dependencies resolved
4. Transform logic executed
5. Results stored as new field values

## API Architecture

### Multi-Protocol Support

**CLI Interface:**
```bash
datafold_cli load-schema schema.json
datafold_cli query --schema UserProfile --fields username,email
```

**HTTP REST API:**
```http
POST /api/schema
POST /api/execute
GET /api/schemas
```

**TCP Protocol:**
```
[4-byte length][JSON payload]
```

### Request/Response Format

**Query Request:**
```json
{
  "type": "query",
  "schema": "UserProfile",
  "fields": ["username", "email"],
  "filter": {
    "field": "age",
    "operator": "gt",
    "value": 18
  }
}
```

**Mutation Request:**
```json
{
  "type": "mutation",
  "schema": "UserProfile",
  "operation": "create",
  "data": {
    "username": "alice",
    "email": "alice@example.com"
  }
}
```

## Storage Architecture

### Atom Storage Model

Each atom is stored with:
- **Primary Key**: Schema name + Atom UUID
- **Version Chain**: Linked list of atom versions
- **Field Data**: JSON-serialized field values
- **Metadata**: Timestamps, permissions, fees

### Index Structure

**Primary Indexes:**
- Schema-based atom lookup
- UUID-based global lookup
- Field-based query indexes

**Range Field Indexes:**
- Key-based B-tree indexes
- Prefix and pattern matching
- Efficient range scans

### Storage Layout

```
/schemas/{schema_name}/
  ├── atoms/
  │   ├── {uuid1} → atom_data
  │   ├── {uuid2} → atom_data
  │   └── ...
  ├── indexes/
  │   ├── fields/
  │   └── ranges/
  └── metadata/
      ├── schema_definition
      └── statistics
```

## Distributed Architecture

### Node Types

**Standalone Node:**
- Single-node deployment
- Full functionality without networking
- Local schema and data management

**Clustered Node:**
- Multi-node deployment
- Schema synchronization
- Distributed query processing

**Embedded Node:**
- Library integration
- In-process data management
- Application-specific schemas

### Network Topology

```
Node A ←→ Node B ←→ Node C
  ↑         ↑         ↑
  └─────────┼─────────┘
            ↓
          Node D
```

**Connection Patterns:**
- **Mesh**: Full connectivity between nodes
- **Hub**: Central node with spoke connections
- **Chain**: Linear node connections

### Schema Synchronization

**Pull-Based Model:**
1. Node requests schema from peer
2. Schema validation and compatibility check
3. Local schema loading and indexing
4. Notification to dependent systems

**Push-Based Model:**
1. New schema creation detected locally
2. Broadcast to connected peers
3. Peer validation and acceptance
4. Network-wide schema propagation

**Note**: Schemas are immutable once created. To change schema structure, a new schema with a different name must be created.

## Concurrency Model

### Async/Await Architecture

Built on Tokio runtime for high concurrency:

```rust
#[tokio::main]
async fn main() {
    let node = DataFoldNode::new(config).await?;
    node.start_services().await?;
}
```

### Locking Strategy

**Database-Level Locking:**
- Read-write locks for schema operations
- Fine-grained locks for atom modifications
- Deadlock prevention with ordered locking

**Network-Level Coordination:**
- Distributed consensus for schema changes
- Eventually consistent data replication
- Conflict resolution with timestamps

## Error Handling

### Error Hierarchy

```rust
pub enum FoldDbError {
    SchemaError(SchemaError),
    NetworkError(NetworkError),
    PermissionError(PermissionError),
    PaymentError(PaymentError),
    StorageError(StorageError),
}
```

### Error Recovery

**Transactional Safety:**
- Atomic operations with rollback
- Consistent state recovery
- Error propagation to clients

**Network Resilience:**
- Connection retry mechanisms
- Peer failure detection
- Graceful degradation

## Performance Characteristics

### Query Performance

**Field Queries:**
- O(1) lookup for primary key access
- O(log n) for indexed field access
- O(n) for unindexed field scans

**Range Queries:**
- O(log n) for key-based access
- O(k) for prefix/pattern matching (k = matches)
- Efficient range scans with B-tree indexes

### Transform Performance

**Execution Time:**
- Sub-millisecond for simple operations
- Parallel execution for independent transforms
- Optimized dependency resolution

### Network Performance

**Throughput:**
- Hundreds of operations per second per node
- Linear scaling with cluster size
- Efficient binary protocol

## Security Model

### Authentication

**Public Key Infrastructure:**
- Ed25519 key pairs for node identity
- Digital signatures for operation verification
- Trust chains for multi-hop authentication

### Authorization

**Permission Validation:**
- Field-level access control
- Distance-based trust decisions
- Explicit permission overrides

### Network Security

**Transport Encryption:**
- Noise protocol for peer communication
- TLS for HTTP endpoints
- Secure key exchange

## Monitoring and Observability

### Logging System

**Feature-Based Logging:**
```rust
log_feature!(LogFeature::Transform, info, "Transform executed successfully");
log_feature!(LogFeature::Network, debug, "Peer connection established");
log_feature!(LogFeature::Schema, error, "Schema validation failed");
```

**Log Levels:**
- ERROR: Critical failures
- WARN: Non-critical issues
- INFO: Important events
- DEBUG: Detailed diagnostics
- TRACE: Verbose debugging

### Metrics Collection

**Performance Metrics:**
- Operation latency and throughput
- Resource utilization (CPU, memory, disk)
- Network statistics (connections, bandwidth)

**Business Metrics:**
- Schema usage patterns
- Transform execution frequency
- Payment transaction volume

### Health Monitoring

**Node Health:**
- Service availability checks
- Resource threshold monitoring
- Peer connectivity status

**System Health:**
- Schema consistency validation
- Data integrity checks
- Network partition detection

## Extension Points

### Custom Transform Functions

```rust
pub trait TransformFunction {
    fn name(&self) -> &str;
    fn execute(&self, inputs: &[Value]) -> Result<Value>;
}
```

### Storage Backends

```rust
pub trait StorageBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
}
```

### Network Protocols

```rust
pub trait NetworkProtocol {
    async fn send_message(&self, peer: PeerId, message: Message) -> Result<()>;
    async fn receive_message(&self) -> Result<(PeerId, Message)>;
}
```

## Future Architecture Considerations

### Horizontal Scaling
- Consistent hashing for data distribution
- Replica placement strategies
- Load balancing algorithms

### Advanced Querying
- SQL-like query language
- Join operations across schemas
- Aggregation and analytics functions

### Machine Learning Integration
- Transform-based ML pipelines
- Real-time model inference
- Feature store capabilities

---

**Next**: See [Use Cases](./use-cases.md) for practical applications of this architecture.