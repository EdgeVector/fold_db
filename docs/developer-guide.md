# Developer Guide

This guide provides comprehensive information for developers integrating Fold DB into their applications, including embedding patterns, client libraries, and best practices.

## Table of Contents

1. [Integration Patterns](#integration-patterns)
2. [Embedding Fold DB](#embedding-fold-db)
3. [Client Libraries](#client-libraries)
4. [API Integration](#api-integration)
5. [Custom Extensions](#custom-extensions)
6. [Testing Strategies](#testing-strategies)
7. [Performance Optimization](#performance-optimization)
8. [Error Handling](#error-handling)
9. [Examples and Templates](#examples-and-templates)

## Integration Patterns

### Standalone Service

**External Service Integration:**
Use Fold DB as a separate service with API communication.

```rust
// Application code
use reqwest::Client;
use serde_json::json;

pub struct FoldDBClient {
    client: Client,
    base_url: String,
}

impl FoldDBClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }
    
    pub async fn query(&self, schema: &str, fields: &[&str]) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let response = self.client
            .post(&format!("{}/api/execute", self.base_url))
            .json(&json!({
                "operation": format!(
                    "{{\"type\":\"query\",\"schema\":\"{}\",\"fields\":{:?}}}",
                    schema, fields
                )
            }))
            .send()
            .await?;
        
        Ok(response.json().await?)
    }
}
```

### Embedded Library

**Direct Library Integration:**
Embed Fold DB directly into your application.

```rust
use fold_node::{DataFoldNode, NodeConfig, Schema};
use std::path::PathBuf;

pub struct AppWithFoldDB {
    fold_db: DataFoldNode,
    // Your application state
}

impl AppWithFoldDB {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = NodeConfig {
            storage_path: PathBuf::from("./app_data"),
            default_trust_distance: 0,
            network_enabled: false,
        };
        
        let fold_db = DataFoldNode::new(config).await?;
        
        // Load application schemas
        let user_schema = Schema::from_file("schemas/user.json")?;
        fold_db.load_schema(user_schema).await?;
        
        Ok(Self { fold_db })
    }
    
    pub async fn get_user(&self, user_id: &str) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let query = Query::new("User")
            .select(&["id", "username", "email"])
            .filter("id", "eq", user_id);
            
        let result = self.fold_db.query(query).await?;
        
        if let Some(row) = result.rows.first() {
            Ok(Some(User::from_row(row)?))
        } else {
            Ok(None)
        }
    }
}
```

### Microservice Architecture

**Service Mesh Integration:**
```yaml
# docker-compose.yml
version: '3.8'
services:
  app-service:
    image: myapp:latest
    depends_on:
      - folddb-service
    environment:
      - FOLDDB_URL=http://folddb-service:9001
      
  folddb-service:
    image: folddb:latest
    ports:
      - "9001:9001"
    volumes:
      - folddb-data:/data
      - ./schemas:/schemas
    environment:
      - RUST_LOG=info
      
volumes:
  folddb-data:
```

**Service Configuration:**
```rust
// Service discovery integration
pub struct ServiceMesh {
    folddb_client: FoldDBClient,
}

impl ServiceMesh {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let folddb_url = std::env::var("FOLDDB_URL")
            .unwrap_or_else(|_| "http://localhost:9001".to_string());
            
        let client = FoldDBClient::new(folddb_url);
        
        // Health check
        client.health_check().await?;
        
        Ok(Self {
            folddb_client: client,
        })
    }
}
```

## Embedding Fold DB

### Rust Application Integration

**Cargo.toml Configuration:**
```toml
[dependencies]
fold_node = { path = "../fold_node" }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Application Structure:**
```rust
use fold_node::{DataFoldNode, NodeConfig, Schema, Query, Mutation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub profile_data: std::collections::HashMap<String, String>,
}

pub struct UserService {
    db: Arc<DataFoldNode>,
}

impl UserService {
    pub async fn new(config: NodeConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let db = Arc::new(DataFoldNode::new(config).await?);
        
        // Load user schema
        // Note: Schemas are immutable once created. If you need to change the schema structure,
        // create a new schema with a different name.
        let schema_json = include_str!("../schemas/user.json");
        let schema: Schema = serde_json::from_str(schema_json)?;
        db.load_schema(schema).await?;
        
        Ok(Self { db })
    }
    
    pub async fn create_user(&self, user: User) -> Result<String, Box<dyn std::error::Error>> {
        let mutation = Mutation::new("User")
            .operation("create")
            .data(serde_json::to_value(user)?);
            
        let result = self.db.mutate(mutation).await?;
        Ok(result.affected_rows.to_string())
    }
    
    pub async fn get_user(&self, user_id: &str) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let query = Query::new("User")
            .select(&["id", "username", "email", "profile_data"])
            .filter("id", "eq", user_id);
            
        let result = self.db.query(query).await?;
        
        if let Some(row) = result.rows.first() {
            let user: User = serde_json::from_value(row.clone())?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }
    
    pub async fn update_user_profile(&self, user_id: &str, profile_data: std::collections::HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        let mutation = Mutation::new("User")
            .operation("update")
            .filter("id", "eq", user_id)
            .data(json!({"profile_data": profile_data}));
            
        self.db.mutate(mutation).await?;
        Ok(())
    }
}

// Application main
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = NodeConfig {
        storage_path: PathBuf::from("./data"),
        default_trust_distance: 0,
        network_enabled: false,
    };
    
    let user_service = UserService::new(config).await?;
    
    // Create a user
    let user = User {
        id: "user_001".to_string(),
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        profile_data: [
            ("location".to_string(), "San Francisco".to_string()),
            ("bio".to_string(), "Software Engineer".to_string()),
        ].iter().cloned().collect(),
    };
    
    user_service.create_user(user).await?;
    
    // Query the user
    if let Some(retrieved_user) = user_service.get_user("user_001").await? {
        println!("Retrieved user: {:?}", retrieved_user);
    }
    
    Ok(())
}
```

### C/C++ Integration via FFI

**C Header (fold_db.h):**
```c
#ifndef FOLD_DB_H
#define FOLD_DB_H

#include <stdint.h>
#include <stdlib.h>

typedef struct FoldDBNode FoldDBNode;

// Lifecycle management
FoldDBNode* folddb_create(const char* config_path);
void folddb_destroy(FoldDBNode* node);

// Schema operations
int folddb_load_schema(FoldDBNode* node, const char* schema_json);
char* folddb_list_schemas(FoldDBNode* node);

// Data operations
char* folddb_query(FoldDBNode* node, const char* query_json);
char* folddb_mutate(FoldDBNode* node, const char* mutation_json);

// Memory management
void folddb_free_string(char* str);

#endif
```

**Rust FFI Implementation:**
```rust
use fold_node::{DataFoldNode, NodeConfig, Schema, Query, Mutation};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

#[no_mangle]
pub extern "C" fn folddb_create(config_path: *const c_char) -> *mut DataFoldNode {
    let config_path = unsafe {
        if config_path.is_null() {
            return ptr::null_mut();
        }
        CStr::from_ptr(config_path).to_str().unwrap_or("")
    };
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = rt.block_on(async {
        NodeConfig::from_file(config_path).unwrap_or_default()
    });
    
    match rt.block_on(DataFoldNode::new(config)) {
        Ok(node) => Box::into_raw(Box::new(node)),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn folddb_destroy(node: *mut DataFoldNode) {
    if !node.is_null() {
        unsafe {
            let _ = Box::from_raw(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn folddb_query(node: *mut DataFoldNode, query_json: *const c_char) -> *mut c_char {
    let node = unsafe {
        if node.is_null() { return ptr::null_mut(); }
        &*node
    };
    
    let query_str = unsafe {
        if query_json.is_null() { return ptr::null_mut(); }
        CStr::from_ptr(query_json).to_str().unwrap_or("")
    };
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let query: Query = serde_json::from_str(query_str).ok()?;
        let result = node.query(query).await.ok()?;
        serde_json::to_string(&result).ok()
    });
    
    match result {
        Some(json_str) => {
            let c_str = CString::new(json_str).unwrap();
            c_str.into_raw()
        }
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn folddb_free_string(str: *mut c_char) {
    if !str.is_null() {
        unsafe {
            let _ = CString::from_raw(str);
        }
    }
}
```

**C++ Wrapper:**
```cpp
#include "fold_db.h"
#include <string>
#include <memory>
#include <stdexcept>

class FoldDB {
private:
    FoldDBNode* node_;
    
public:
    explicit FoldDB(const std::string& config_path) {
        node_ = folddb_create(config_path.c_str());
        if (!node_) {
            throw std::runtime_error("Failed to create FoldDB node");
        }
    }
    
    ~FoldDB() {
        if (node_) {
            folddb_destroy(node_);
        }
    }
    
    bool loadSchema(const std::string& schema_json) {
        return folddb_load_schema(node_, schema_json.c_str()) == 0;
    }
    
    std::string query(const std::string& query_json) {
        char* result = folddb_query(node_, query_json.c_str());
        if (!result) {
            return "";
        }
        
        std::string result_str(result);
        folddb_free_string(result);
        return result_str;
    }
};

// Usage example
int main() {
    try {
        FoldDB db("config.json");
        
        // Load schema
        std::string schema = R"({
            "name": "User",
            "fields": {
                "id": {"field_type": "Single"},
                "name": {"field_type": "Single"}
            }
        })";
        
        db.loadSchema(schema);
        
        // Query data
        std::string query = R"({
            "type": "query",
            "schema": "User",
            "fields": ["id", "name"]
        })";
        
        std::string result = db.query(query);
        std::cout << "Query result: " << result << std::endl;
        
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
```

### Python Integration

**Python Wrapper (PyO3):**
```rust
use pyo3::prelude::*;
use fold_node::{DataFoldNode, NodeConfig, Schema, Query, Mutation};

#[pyclass]
struct PyFoldDB {
    node: DataFoldNode,
    rt: tokio::runtime::Runtime,
}

#[pymethods]
impl PyFoldDB {
    #[new]
    fn new(config_path: Option<String>) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        let config = if let Some(path) = config_path {
            rt.block_on(NodeConfig::from_file(&path))
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyFileNotFoundError, _>(e.to_string()))?
        } else {
            NodeConfig::default()
        };
        
        let node = rt.block_on(DataFoldNode::new(config))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        Ok(Self { node, rt })
    }
    
    fn load_schema(&self, schema_json: &str) -> PyResult<()> {
        let schema: Schema = serde_json::from_str(schema_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        self.rt.block_on(self.node.load_schema(schema))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        Ok(())
    }
    
    fn query(&self, query_json: &str) -> PyResult<String> {
        let query: Query = serde_json::from_str(query_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        let result = self.rt.block_on(self.node.query(query))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        serde_json::to_string(&result)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
}

#[pymodule]
fn fold_db(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyFoldDB>()?;
    Ok(())
}
```

**Python Usage:**
```python
import fold_db
import json

# Create FoldDB instance
db = fold_db.PyFoldDB("config.json")

# Load schema
schema = {
    "name": "User",
    "fields": {
        "id": {"field_type": "Single"},
        "username": {"field_type": "Single"},
        "email": {"field_type": "Single"}
    }
}

db.load_schema(json.dumps(schema))

# Query data
query = {
    "type": "query",
    "schema": "User",
    "fields": ["id", "username", "email"]
}

result = db.query(json.dumps(query))
data = json.loads(result)

print("Query results:", data)
```

## Client Libraries

### HTTP Client Library

**TypeScript/JavaScript Client:**
```typescript
interface FoldDBConfig {
  baseUrl: string;
  apiKey?: string;
  timeout?: number;
}

interface QueryRequest {
  type: 'query';
  schema: string;
  fields: string[];
  filter?: any;
}

interface MutationRequest {
  type: 'mutation';
  schema: string;
  operation: 'create' | 'update' | 'delete';
  data?: any;
  filter?: any;
}

class FoldDBClient {
  private config: FoldDBConfig;
  
  constructor(config: FoldDBConfig) {
    this.config = {
      timeout: 30000,
      ...config
    };
  }
  
  async query(request: QueryRequest): Promise<any> {
    const response = await fetch(`${this.config.baseUrl}/api/execute`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(this.config.apiKey && { 'Authorization': `Bearer ${this.config.apiKey}` })
      },
      body: JSON.stringify({
        operation: JSON.stringify(request)
      })
    });
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    return await response.json();
  }
  
  async mutate(request: MutationRequest): Promise<any> {
    const response = await fetch(`${this.config.baseUrl}/api/execute`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(this.config.apiKey && { 'Authorization': `Bearer ${this.config.apiKey}` })
      },
      body: JSON.stringify({
        operation: JSON.stringify(request)
      })
    });
    
    return await response.json();
  }
  
  async loadSchema(schema: any): Promise<void> {
    const response = await fetch(`${this.config.baseUrl}/api/schema`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(this.config.apiKey && { 'Authorization': `Bearer ${this.config.apiKey}` })
      },
      body: JSON.stringify(schema)
    });
    
    if (!response.ok) {
      throw new Error(`Failed to load schema: ${response.statusText}`);
    }
  }
  
  async listSchemas(): Promise<any[]> {
    const response = await fetch(`${this.config.baseUrl}/api/schemas`, {
      headers: {
        ...(this.config.apiKey && { 'Authorization': `Bearer ${this.config.apiKey}` })
      }
    });
    
    return await response.json();
  }
}

// Usage
const client = new FoldDBClient({
  baseUrl: 'http://localhost:9001',
  apiKey: 'your-api-key'
});

// Load schema
await client.loadSchema({
  name: 'User',
  fields: {
    id: { field_type: 'Single' },
    username: { field_type: 'Single' }
  }
});

// Query data
const users = await client.query({
  type: 'query',
  schema: 'User',
  fields: ['id', 'username']
});
```

### TCP Client Library

**Go TCP Client:**
```go
package folddb

import (
    "encoding/binary"
    "encoding/json"
    "fmt"
    "net"
    "time"
)

type Client struct {
    conn    net.Conn
    address string
    timeout time.Duration
}

type Request struct {
    AppID     string      `json:"app_id"`
    Operation string      `json:"operation"`
    Params    interface{} `json:"params"`
}

type Response struct {
    Results  []interface{} `json:"results"`
    Errors   []string      `json:"errors"`
    Metadata interface{}   `json:"metadata"`
}

func NewClient(address string) *Client {
    return &Client{
        address: address,
        timeout: 30 * time.Second,
    }
}

func (c *Client) Connect() error {
    conn, err := net.DialTimeout("tcp", c.address, c.timeout)
    if err != nil {
        return err
    }
    c.conn = conn
    return nil
}

func (c *Client) Close() error {
    if c.conn != nil {
        return c.conn.Close()
    }
    return nil
}

func (c *Client) SendRequest(req Request) (*Response, error) {
    // Serialize request
    data, err := json.Marshal(req)
    if err != nil {
        return nil, err
    }
    
    // Send length prefix
    length := uint32(len(data))
    if err := binary.Write(c.conn, binary.LittleEndian, length); err != nil {
        return nil, err
    }
    
    // Send data
    if _, err := c.conn.Write(data); err != nil {
        return nil, err
    }
    
    // Read response length
    var responseLength uint32
    if err := binary.Read(c.conn, binary.LittleEndian, &responseLength); err != nil {
        return nil, err
    }
    
    // Read response data
    responseData := make([]byte, responseLength)
    if _, err := c.conn.Read(responseData); err != nil {
        return nil, err
    }
    
    // Deserialize response
    var resp Response
    if err := json.Unmarshal(responseData, &resp); err != nil {
        return nil, err
    }
    
    return &resp, nil
}

func (c *Client) Query(schema string, fields []string, filter interface{}) (*Response, error) {
    req := Request{
        AppID:     "go-client",
        Operation: "query",
        Params: map[string]interface{}{
            "schema": schema,
            "fields": fields,
            "filter": filter,
        },
    }
    
    return c.SendRequest(req)
}

// Usage example
func main() {
    client := NewClient("localhost:9000")
    
    if err := client.Connect(); err != nil {
        panic(err)
    }
    defer client.Close()
    
    response, err := client.Query("User", []string{"id", "username"}, nil)
    if err != nil {
        panic(err)
    }
    
    fmt.Printf("Results: %+v\n", response.Results)
}
```

## API Integration

### REST API Patterns

**Repository Pattern:**
```rust
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
}

#[async_trait]
pub trait UserRepository {
    async fn create(&self, user: User) -> Result<String, Box<dyn std::error::Error>>;
    async fn get(&self, id: &str) -> Result<Option<User>, Box<dyn std::error::Error>>;
    async fn update(&self, user: User) -> Result<(), Box<dyn std::error::Error>>;
    async fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<User>, Box<dyn std::error::Error>>;
}

pub struct FoldDBUserRepository {
    client: FoldDBClient,
}

impl FoldDBUserRepository {
    pub fn new(client: FoldDBClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl UserRepository for FoldDBUserRepository {
    async fn create(&self, user: User) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client.mutate(MutationRequest {
            type_: "mutation".to_string(),
            schema: "User".to_string(),
            operation: "create".to_string(),
            data: Some(serde_json::to_value(user)?),
            filter: None,
        }).await?;
        
        Ok(response.metadata.id.unwrap_or_default())
    }
    
    async fn get(&self, id: &str) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let response = self.client.query(QueryRequest {
            type_: "query".to_string(),
            schema: "User".to_string(),
            fields: vec!["id".to_string(), "username".to_string(), "email".to_string()],
            filter: Some(json!({"id": id})),
        }).await?;
        
        if let Some(row) = response.results.first() {
            let user: User = serde_json::from_value(row.clone())?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }
    
    async fn update(&self, user: User) -> Result<(), Box<dyn std::error::Error>> {
        self.client.mutate(MutationRequest {
            type_: "mutation".to_string(),
            schema: "User".to_string(),
            operation: "update".to_string(),
            data: Some(serde_json::to_value(&user)?),
            filter: Some(json!({"id": user.id})),
        }).await?;
        
        Ok(())
    }
    
    async fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client.mutate(MutationRequest {
            type_: "mutation".to_string(),
            schema: "User".to_string(),
            operation: "delete".to_string(),
            data: None,
            filter: Some(json!({"id": id})),
        }).await?;
        
        Ok(())
    }
    
    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<User>, Box<dyn std::error::Error>> {
        let response = self.client.query(QueryRequest {
            type_: "query".to_string(),
            schema: "User".to_string(),
            fields: vec!["id".to_string(), "username".to_string(), "email".to_string()],
            filter: Some(json!({
                "limit": limit,
                "offset": offset
            })),
        }).await?;
        
        let users: Vec<User> = response.results
            .iter()
            .map(|row| serde_json::from_value(row.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        
        Ok(users)
    }
}
```

### GraphQL Integration

**GraphQL Schema:**
```graphql
type User {
  id: ID!
  username: String!
  email: String!
  profileData: [ProfileEntry!]!
}

type ProfileEntry {
  key: String!
  value: String!
}

type Query {
  user(id: ID!): User
  users(limit: Int, offset: Int): [User!]!
}

type Mutation {
  createUser(input: CreateUserInput!): User!
  updateUser(id: ID!, input: UpdateUserInput!): User!
  deleteUser(id: ID!): Boolean!
}

input CreateUserInput {
  username: String!
  email: String!
  profileData: [ProfileEntryInput!]
}

input UpdateUserInput {
  username: String
  email: String
  profileData: [ProfileEntryInput!]
}

input ProfileEntryInput {
  key: String!
  value: String!
}
```

**Resolver Implementation:**
```rust
use async_graphql::{Context, Object, Result, ID};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn user(&self, ctx: &Context<'_>, id: ID) -> Result<Option<User>> {
        let repo = ctx.data::<Box<dyn UserRepository>>()?;
        Ok(repo.get(&id).await?)
    }
    
    async fn users(&self, ctx: &Context<'_>, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<User>> {
        let repo = ctx.data::<Box<dyn UserRepository>>()?;
        let limit = limit.unwrap_or(10) as usize;
        let offset = offset.unwrap_or(0) as usize;
        Ok(repo.list(limit, offset).await?)
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_user(&self, ctx: &Context<'_>, input: CreateUserInput) -> Result<User> {
        let repo = ctx.data::<Box<dyn UserRepository>>()?;
        
        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            username: input.username,
            email: input.email,
        };
        
        let id = repo.create(user.clone()).await?;
        Ok(user)
    }
    
    async fn update_user(&self, ctx: &Context<'_>, id: ID, input: UpdateUserInput) -> Result<User> {
        let repo = ctx.data::<Box<dyn UserRepository>>()?;
        
        let mut user = repo.get(&id).await?
            .ok_or("User not found")?;
        
        if let Some(username) = input.username {
            user.username = username;
        }
        if let Some(email) = input.email {
            user.email = email;
        }
        
        repo.update(user.clone()).await?;
        Ok(user)
    }
}
```

## Custom Extensions

### Custom Transform Functions

**Transform Function Trait:**
```rust
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait TransformFunction: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, inputs: &[Value]) -> Result<Value, Box<dyn std::error::Error>>;
}

// Example: Custom encryption transform
pub struct EncryptionTransform {
    key: Vec<u8>,
}

impl EncryptionTransform {
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }
}

#[async_trait]
impl TransformFunction for EncryptionTransform {
    fn name(&self) -> &str {
        "encrypt"
    }
    
    fn description(&self) -> &str {
        "Encrypts the input value using AES-256"
    }
    
    async fn execute(&self, inputs: &[Value]) -> Result<Value, Box<dyn std::error::Error>> {
        if inputs.is_empty() {
            return Err("No input provided".into());
        }
        
        let input_str = inputs[0].as_str()
            .ok_or("Input must be a string")?;
        
        // Implement encryption logic here
        let encrypted = encrypt_aes256(input_str, &self.key)?;
        
        Ok(Value::String(base64::encode(encrypted)))
    }
}

// Register custom transform
pub fn register_custom_transforms(node: &mut DataFoldNode) -> Result<(), Box<dyn std::error::Error>> {
    let encryption_key = load_encryption_key()?;
    let encrypt_transform = Box::new(EncryptionTransform::new(encryption_key));
    
    node.register_transform_function(encrypt_transform)?;
    
    Ok(())
}
```

### Custom Storage Backends

**Storage Backend Trait:**
```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>>;
    async fn put(&self, key: &str, value: Vec<u8>) -> Result<(), Box<dyn std::error::Error>>;
    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>, Box<dyn std::error::Error>>;
}

// Example: S3 storage backend
pub struct S3StorageBackend {
    client: aws_sdk_s3::Client,
    bucket: String,
}

impl S3StorageBackend {
    pub fn new(client: aws_sdk_s3::Client, bucket: String) -> Self {
        Self { client, bucket }
    }
}

#[async_trait]
impl StorageBackend for S3StorageBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        match self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(response) => {
                let data = response.body.collect().await?;
                Ok(Some(data.into_bytes().to_vec()))
            }
            Err(_) => Ok(None),
        }
    }
    
    async fn put(&self, key: &str, value: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(aws_sdk_s3::types::ByteStream::from(value))
            .send()
            .await?;
        
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        
        Ok(())
    }
    
    async fn list(&self, prefix: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let response = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await?;
        
        let keys = response
            .contents()
            .unwrap_or_default()
            .iter()
            .filter_map(|obj| obj.key().map(|k| k.to_string()))
            .collect();
        
        Ok(keys)
    }
}
```

## Testing Strategies

### Unit Testing

**Test Configuration:**
```rust
use fold_node::{DataFoldNode, NodeConfig};
use tempfile::TempDir;

pub struct TestEnvironment {
    pub node: DataFoldNode,
    pub temp_dir: TempDir,
}

impl TestEnvironment {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        
        let config = NodeConfig {
            storage_path: temp_dir.path().to_path_buf(),
            default_trust_distance: 0,
            network_enabled: false,
        };
        
        let node = DataFoldNode::new(config).await?;
        
        Ok(Self { node, temp_dir })
    }
    
    pub async fn load_test_schema(&self) -> Result<(), Box<dyn std::error::Error>> {
        let schema_json = r#"{
            "name": "TestUser",
            "fields": {
                "id": {
                    "field_type": "Single",
                    "permission_policy": {
                        "read_policy": {"NoRequirement": null},
                        "write_policy": {"NoRequirement": null}
                    }
                },
                "username": {
                    "field_type": "Single",
                    "permission_policy": {
                        "read_policy": {"NoRequirement": null},
                        "write_policy": {"NoRequirement": null}
                    }
                }
            }
        }"#;
        
        let schema: Schema = serde_json::from_str(schema_json)?;
        self.node.load_schema(schema).await?;
        
        Ok(())
    }
}

#[tokio::test]
async fn test_user_creation() {
    let env = TestEnvironment::new().await.unwrap();
    env.load_test_schema().await.unwrap();
    
    // Create user
    let mutation = Mutation::new("TestUser")
        .operation("create")
        .data(json!({
            "id": "test_user_1",
            "username": "testuser"
        }));
    
    let result = env.node.mutate(mutation).await.unwrap();
    assert_eq!(result.affected_rows, 1);
    
    // Query user
    let query = Query::new("TestUser")
        .select(&["id", "username"])
        .filter("id", "eq", "test_user_1");
    
    let result = env.node.query(query).await.unwrap();
    assert_eq!(result.rows.len(), 1);
    
    let user = &result.rows[0];
    assert_eq!(user["id"], "test_user_1");
    assert_eq!(user["username"], "testuser");
}
```

### Integration Testing

**HTTP API Testing:**
```rust
use actix_web::test;
use actix_web::web::Data;

#[actix_rt::test]
async fn test_http_api() {
    let env = TestEnvironment::new().await.unwrap();
    env.load_test_schema().await.unwrap();
    
    let app = test::init_service(
        App::new()
            .app_data(Data::new(env.node.clone()))
            .service(
                web::scope("/api")
                    .route("/execute", web::post().to(execute_operation))
                    .route("/schema", web::post().to(load_schema))
            )
    ).await;
    
    // Test schema loading
    let req = test::TestRequest::post()
        .uri("/api/schema")
        .set_json(&json!({
            "name": "TestSchema",
            "fields": {
                "test_field": {
                    "field_type": "Single"
                }
            }
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    // Test query execution
    let req = test::TestRequest::post()
        .uri("/api/execute")
        .set_json(&json!({
            "operation": "{\"type\":\"query\",\"schema\":\"TestUser\",\"fields\":[\"id\",\"username\"]}"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}
```

### Load Testing

**Performance Testing:**
```rust
use std::time::Instant;
use tokio::task::JoinSet;

#[tokio::test]
async fn load_test_concurrent_queries() {
    let env = TestEnvironment::new().await.unwrap();
    env.load_test_schema().await.unwrap();
    
    // Insert test data
    for i in 0..1000 {
        let mutation = Mutation::new("TestUser")
            .operation("create")
            .data(json!({
                "id": format!("user_{}", i),
                "username": format!("user{}", i)
            }));
        
        env.node.mutate(mutation).await.unwrap();
    }
    
    let mut tasks = JoinSet::new();
    let start = Instant::now();
    
    // Spawn 100 concurrent queries
    for _ in 0..100 {
        let node = env.node.clone();
        tasks.spawn(async move {
            let query = Query::new("TestUser")
                .select(&["id", "username"])
                .limit(10);
            
            node.query(query).await
        });
    }
    
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        results.push(result.unwrap().unwrap());
    }
    
    let duration = start.elapsed();
    println!("Completed 100 queries in {:?}", duration);
    println!("Average: {:?} per query", duration / 100);
    
    assert_eq!(results.len(), 100);
    assert!(duration.as_millis() < 5000); // Should complete within 5 seconds
}
```

## Performance Optimization

### Connection Pooling

**HTTP Client Pool:**
```rust
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct PooledFoldDBClient {
    client: Client,
    base_url: String,
    semaphore: Arc<Semaphore>,
}

impl PooledFoldDBClient {
    pub fn new(base_url: String, max_connections: usize) -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(max_connections)
            .pool_idle_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        
        Self {
            client,
            base_url,
            semaphore: Arc::new(Semaphore::new(max_connections)),
        }
    }
    
    pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse, Box<dyn std::error::Error>> {
        let _permit = self.semaphore.acquire().await?;
        
        let response = self.client
            .post(&format!("{}/api/execute", self.base_url))
            .json(&json!({
                "operation": serde_json::to_string(&request)?
            }))
            .send()
            .await?;
        
        Ok(response.json().await?)
    }
}
```

### Caching Strategies

**Query Result Caching:**
```rust
use moka::future::Cache;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct QueryKey {
    schema: String,
    fields: Vec<String>,
    filter: Option<serde_json::Value>,
}

impl Hash for QueryKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.schema.hash(state);
        self.fields.hash(state);
        if let Some(filter) = &self.filter {
            filter.to_string().hash(state);
        }
    }
}

impl PartialEq for QueryKey {
    fn eq(&self, other: &Self) -> bool {
        self.schema == other.schema 
            && self.fields == other.fields 
            && self.filter == other.filter
    }
}

impl Eq for QueryKey {}

pub struct CachingFoldDBClient {
    client: FoldDBClient,
    cache: Cache<QueryKey, QueryResponse>,
}

impl CachingFoldDBClient {
    pub fn new(client: FoldDBClient, cache_size: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(cache_size)
            .time_to_live(ttl)
            .build();
        
        Self { client, cache }
    }
    
    pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse, Box<dyn std::error::Error>> {
        let key = QueryKey {
            schema: request.schema.clone(),
            fields: request.fields.clone(),
            filter: request.filter.clone(),
        };
        
        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }
        
        let response = self.client.query(request).await?;
        self.cache.insert(key, response.clone()).await;
        
        Ok(response)
    }
}
```

### Batch Operations

**Batch Query Client:**
```rust
pub struct BatchFoldDBClient {
    client: FoldDBClient,
    batch_size: usize,
    batch_timeout: Duration,
}

impl BatchFoldDBClient {
    pub fn new(client: FoldDBClient, batch_size: usize, batch_timeout: Duration) -> Self {
        Self {
            client,
            batch_size,
            batch_timeout,
        }
    }
    
    pub async fn batch_query(&self, requests: Vec<QueryRequest>) -> Result<Vec<QueryResponse>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        
        for chunk in requests.chunks(self.batch_size) {
            let batch_request = json!({
                "operations": chunk.iter().map(|req| {
                    json!({
                        "type": "query",
                        "schema": req.schema,
                        "fields": req.fields,
                        "filter": req.filter
                    })
                }).collect::<Vec<_>>()
            });
            
            let response = self.client.execute_batch(batch_request).await?;
            results.extend(response.results);
        }
        
        Ok(results)
    }
}
```

## Error Handling

### Error Types

**Comprehensive Error Handling:**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FoldDBClientError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },
    
    #[error("Payment required: {amount} sats")]
    PaymentRequired { amount: u64 },
    
    #[error("Schema not found: {schema}")]
    SchemaNotFound { schema: String },
    
    #[error("Field not found: {field} in schema {schema}")]
    FieldNotFound { schema: String, field: String },
    
    #[error("Validation error: {message}")]
    Validation { message: String },
    
    #[error("Server error: {status} - {message}")]
    Server { status: u16, message: String },
    
    #[error("Timeout after {duration:?}")]
    Timeout { duration: Duration },
    
    #[error("Rate limit exceeded")]
    RateLimit,
}

impl FoldDBClientError {
    pub fn is_retryable(&self) -> bool {
        match self {
            FoldDBClientError::Network(_) => true,
            FoldDBClientError::Server { status, .. } => *status >= 500,
            FoldDBClientError::Timeout { .. } => true,
            FoldDBClientError::RateLimit => true,
            _ => false,
        }
    }
}
```

### Retry Logic

**Exponential Backoff:**
```rust
use tokio::time::{sleep, Duration};

pub struct RetryConfig {
    pub max_attempts: usize,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

pub async fn retry_with_backoff<F, T, E>(
    operation: F,
    config: RetryConfig,
) -> Result<T, E>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>>>>,
    E: std::fmt::Debug,
{
    let mut attempt = 0;
    let mut delay = config.base_delay;
    
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                attempt += 1;
                
                if attempt >= config.max_attempts {
                    return Err(error);
                }
                
                println!("Attempt {} failed: {:?}, retrying in {:?}", attempt, error, delay);
                sleep(delay).await;
                
                delay = std::cmp::min(
                    Duration::from_millis((delay.as_millis() as f64 * config.backoff_multiplier) as u64),
                    config.max_delay,
                );
            }
        }
    }
}

// Usage
let result = retry_with_backoff(
    || Box::pin(client.query(request.clone())),
    RetryConfig::default(),
).await?;
```

## Examples and Templates

### Web Application Template

**Actix Web Application:**
```rust
use actix_web::{web, App, HttpServer, Result, HttpResponse};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CreateUserRequest {
    username: String,
    email: String,
}

#[derive(Serialize)]
struct UserResponse {
    id: String,
    username: String,
    email: String,
}

async fn create_user(
    user_repo: web::Data<Box<dyn UserRepository>>,
    req: web::Json<CreateUserRequest>,
) -> Result<HttpResponse> {
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: req.username.clone(),
        email: req.email.clone(),
    };
    
    match user_repo.create(user.clone()).await {
        Ok(_) => Ok(HttpResponse::Created().json(UserResponse {
            id: user.id,
            username: user.username,
            email: user.email,
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(format!("Error: {}", e))),
    }
}

async fn get_user(
    user_repo: web::Data<Box<dyn UserRepository>>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    
    match user_repo.get(&user_id).await {
        Ok(Some(user)) => Ok(HttpResponse::Ok().json(UserResponse {
            id: user.id,
            username: user.username,
            email: user.email,
        })),
        Ok(None) => Ok(HttpResponse::NotFound().json("User not found")),
        Err(e) => Ok(HttpResponse::InternalServerError().json(format!("Error: {}", e))),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize FoldDB client
    let folddb_client = FoldDBClient::new("http://localhost:9001".to_string());
    let user_repo: Box<dyn UserRepository> = Box::new(FoldDBUserRepository::new(folddb_client));
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(user_repo.clone()))
            .service(
                web::scope("/api/users")
                    .route("", web::post().to(create_user))
                    .route("/{id}", web::get().to(get_user))
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### CLI Application Template

**CLI Tool with Clap:**
```rust
use clap::{Parser, Subcommand};
use serde_json::json;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, default_value = "http://localhost:9001")]
    folddb_url: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new user
    CreateUser {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        email: String,
    },
    /// Get user by ID
    GetUser {
        #[arg(short, long)]
        id: String,
    },
    /// List all users
    ListUsers {
        #[arg(short, long, default_value = "10")]
        limit: usize,
        #[arg(short, long, default_value = "0")]
        offset: usize,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = FoldDBClient::new(cli.folddb_url);
    
    match cli.command {
        Commands::CreateUser { username, email } => {
            let response = client.mutate(MutationRequest {
                type_: "mutation".to_string(),
                schema: "User".to_string(),
                operation: "create".to_string(),
                data: Some(json!({
                    "username": username,
                    "email": email
                })),
                filter: None,
            }).await?;
            
            println!("User created: {}", serde_json::to_string_pretty(&response)?);
        }
        
        Commands::GetUser { id } => {
            let response = client.query(QueryRequest {
                type_: "query".to_string(),
                schema: "User".to_string(),
                fields: vec!["id".to_string(), "username".to_string(), "email".to_string()],
                filter: Some(json!({"id": id})),
            }).await?;
            
            println!("User: {}", serde_json::to_string_pretty(&response)?);
        }
        
        Commands::ListUsers { limit, offset } => {
            let response = client.query(QueryRequest {
                type_: "query".to_string(),
                schema: "User".to_string(),
                fields: vec!["id".to_string(), "username".to_string(), "email".to_string()],
                filter: Some(json!({
                    "limit": limit,
                    "offset": offset
                })),
            }).await?;
            
            println!("Users: {}", serde_json::to_string_pretty(&response)?);
        }
    }
    
    Ok(())
}
```

### Docker Configuration

**Multi-stage Dockerfile:**
```dockerfile
# Build stage
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin myapp

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/myapp /app/
COPY --from=builder /app/schemas /app/schemas/

EXPOSE 8080

CMD ["./myapp"]
```

**Docker Compose with FoldDB:**
```yaml
version: '3.8'

services:
  myapp:
    build: .
    ports:
      - "8080:8080"
    environment:
      - FOLDDB_URL=http://folddb:9001
      - RUST_LOG=info
    depends_on:
      - folddb
    
  folddb:
    image: folddb:latest
    ports:
      - "9001:9001"
    volumes:
      - folddb_data:/data
      - ./schemas:/schemas
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9001/api/health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  folddb_data:
```

## Troubleshooting

### Mutation Processor Issues

#### Common Mutation Errors

**1. Missing Key Configuration**
```
Error: "HashRange schema 'BlogPostWordIndex' requires key configuration"
```
**Solution:** Ensure HashRange schemas have a `key` field with both `hash_field` and `range_field`:
```json
{
  "name": "BlogPostWordIndex",
  "schema_type": "HashRange",
  "key": {
    "hash_field": "word",
    "range_field": "publish_date"
  }
}
```

**2. Missing Required Fields**
```
Error: "HashRange schema mutation missing hash field 'word'"
```
**Solution:** Include values for all required key fields in your mutation:
```json
{
  "schema_name": "BlogPostWordIndex",
  "fields_and_values": {
    "word": "technology",           // Required hash field
    "publish_date": "2025-01-15",  // Required range field
    "content": "AI advances..."
  }
}
```

**3. Empty Key Fields**
```
Error: "HashRange schema 'BlogPostWordIndex' requires non-empty hash_field in key configuration"
```
**Solution:** Ensure key configuration has non-empty field names:
```json
{
  "key": {
    "hash_field": "word",        // Must not be empty
    "range_field": "publish_date" // Must not be empty
  }
}
```

**4. Field Name Mismatch**
```
Error: "Range schema mutation missing range field 'timestamp'"
```
**Solution:** Verify mutation field names match the schema's key configuration:
```json
// Schema key configuration
{
  "key": {
    "range_field": "timestamp"
  }
}

// Mutation must use 'timestamp', not 'date' or 'time'
{
  "fields_and_values": {
    "timestamp": "2025-01-15T10:30:00Z",  // Correct field name
    "action": "login"
  }
}
```

#### Debugging Mutation Issues

**1. Verify Schema Key Configuration**
```bash
# Check schema key configuration
curl -X GET http://localhost:9001/api/schema/BlogPostWordIndex
```

**2. Validate Mutation Field Names**
```bash
# Test mutation with correct field names
curl -X POST http://localhost:9001/api/mutation \
  -H "Content-Type: application/json" \
  -d '{
    "schema_name": "BlogPostWordIndex",
    "mutation_type": "create",
    "fields_and_values": {
      "word": "technology",
      "publish_date": "2025-01-15",
      "content": "AI advances..."
    }
  }'
```

**3. Check Error Logs**
```bash
# View mutation processor logs
tail -f logs/mutation_processor.log
```

#### Migration Troubleshooting

**Legacy Mutations Failing**
- Verify schema still supports legacy field names
- Check if schema was updated to require universal key configuration
- Ensure backward compatibility is maintained

**Performance Issues**
- Universal key extraction is optimized for performance
- Early validation prevents unnecessary processing
- Clear error messages reduce debugging time

This comprehensive developer guide provides the foundation for integrating Fold DB into various types of applications, from embedded libraries to distributed microservices. The examples and patterns shown here can be adapted to specific use cases and requirements.

---

**Complete Documentation Set**: You now have comprehensive documentation covering all aspects of Fold DB, from basic concepts to advanced integration patterns.