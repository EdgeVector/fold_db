# DataFold Node

## Overview
DataFold Node is a module that provides a complete interface to FoldDB's functionality, allowing users to:
- Load and manage FoldDB instances
- Access schema management capabilities
- Execute mutations and queries
- Handle permissions and trust distance
- Manage atomic operations

## Core Features

### Database Management
- Initialize and load FoldDB instances
- Configure storage locations
- Handle database connections
- Manage database lifecycle

### Schema Operations
- Load schema definitions
- Validate schema structures
- Access schema information
- Update schemas when needed

### Data Operations
- Execute mutations with atomic guarantees
- Perform queries with field-level permissions
- Handle versioning and history tracking
- Manage Molecules and version chains

### Permission Management
- Configure trust distances
- Set explicit permissions
- Validate access rights
- Handle public key authentication

## API Structure

### Initialization
```rust
pub struct DataFoldNode {
    db: FoldDB,
    config: NodeConfig,
}

pub struct NodeConfig {
    storage_path: PathBuf,
    default_trust_distance: u32,
}
```

### Core Methods
```rust
impl DataFoldNode {
    // Initialize a new node
    pub fn new(config: NodeConfig) -> Result<Self>;
    
    // Load an existing database
    pub fn load(config: NodeConfig) -> Result<Self>;
    
    // Schema operations
    pub fn load_schema(&self, schema: Schema) -> Result<()>;
    pub fn get_schema(&self, schema_id: &str) -> Result<Schema>;
    
    // Data operations
    pub fn query(&self, query: Query) -> Result<QueryResult>;
    pub fn mutate(&self, mutation: Mutation) -> Result<MutationResult>;
    
    // Permission operations
    pub fn set_trust_distance(&self, distance: u32) -> Result<()>;
    pub fn set_permissions(&self, permissions: Permissions) -> Result<()>;
}
```

## Usage Examples

### Initialize Node
```rust
let config = NodeConfig {
    storage_path: PathBuf::from("./data"),
    default_trust_distance: 1,
};
let node = DataFoldNode::new(config)?;
```

### Load Schema
```rust
let schema = Schema::from_json(schema_json)?;
node.load_schema(schema)?;
```

### Execute Query
```rust
let query = Query::new("user_profile")
    .select(&["name", "email"])
    .filter("id", "=", "123");
let result = node.query(query)?;
```

### Execute Mutation
```rust
let mutation = Mutation::new("user_profile")
    .set("name", "John Doe")
    .set("email", "john@example.com");
let result = node.mutate(mutation)?;
```

## Technical Requirements

### Dependencies
- Core FoldDB library
- Serde for serialization
- Standard Rust async runtime

### Constraints
- All operations must maintain atomic guarantees
- Schema must be loaded before any data operations
- Permissions must be validated for all operations
- Trust distance rules must be enforced
- Version history must be maintained

## Error Handling
- Clear error types for different failure scenarios
- Proper propagation of underlying FoldDB errors
- Validation errors for schema and data operations
- Permission-related error cases
- Configuration and initialization errors

## Testing Strategy
- Unit tests for all public methods
- Integration tests for full workflows
- Permission validation tests
- Error handling tests
- Schema validation tests
- Performance benchmarks
