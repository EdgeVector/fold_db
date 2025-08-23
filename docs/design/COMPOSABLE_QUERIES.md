# Composable Queries Design Document

## Overview

This document outlines the design for composable queries in DataFold, allowing queries to be combined where the output of one query serves as an index or input for another query. This enables powerful query composition patterns while maintaining the security and trust model of the existing system.

## Current System Analysis

### Existing Query Architecture

The current query system consists of:

- **Query Structure**: [`Query`](src/schema/types/operations.rs:7) with `schema_name`, `fields`, `pub_key`, `trust_distance`, and `filter`
- **Operation Enum**: [`Operation`](src/schema/types/operation.rs) supporting Query and Mutation variants
- **Execution Engine**: [`query()`](src/fold_db_core/mod.rs:340) method in [`FoldDB`](src/fold_db_core/mod.rs:73)
- **Event-Driven Architecture**: [`MessageBus`](src/fold_db_core/mod.rs:83) for component communication
- **HTTP API**: [`execute_query`](src/datafold_node/query_routes.rs:46) endpoint for web queries
- **Schema Management**: [`SchemaCore`](src/fold_db_core/mod.rs:76) managing schema states (Available, Approved, etc.)
- **Range Schema Support**: Built-in filtering for range schemas with range_key filtering
- **Permission System**: Field-level permissions with trust distance validation via [`PermissionWrapper`](src/fold_db_core/mod.rs:81)
- **Transform System**: [`TransformManager`](src/fold_db_core/mod.rs:77) and [`TransformOrchestrator`](src/fold_db_core/mod.rs:78) for data transformations

### Current Limitations

1. **Single Query Execution**: Each query executes independently
2. **No Data Flow**: Results cannot be passed between queries
3. **Static Filtering**: Filters must be known at query definition time
4. **No Composition**: Cannot build complex queries from simpler components

## Composable Query Design

### Core Concepts

#### 1. Query Composition Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComposableQuery {
    /// Simple query (current behavior)
    Simple(Query),
    
    /// Sequential composition: steps execute in order; later steps may reference earlier results
    Sequential {
        queries: Vec<QueryStep>,
    },
    
    /// Parallel composition: steps execute concurrently; no cross-step references within the same group
    Parallel {
        queries: Vec<QueryStep>,
        aggregation: Option<ParallelAggregation>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStep {
    /// Unique identifier for this query step
    pub id: String,
    /// The actual query to execute (can reference previous results using ${query_id.field_name})
    pub query: Query,
    /// Optional fan-out behavior when a referenced value is an array
    pub fanout: Option<FanoutConfig>,
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanoutConfig {
    /// How to handle arrays when substituting into filters
    pub mode: FanoutMode,          // Each | In
    /// Hard limit on number of values to fan out
    pub max_values: usize,         // default: 1000
    /// Number of values per batch in Each mode
    pub batch_size: usize,         // default: 100
    /// Max concurrent batches
    pub concurrency: usize,        // default: 10
    /// Dedupe values before fanout
    pub dedupe: bool,              // default: true
    /// Optional key for merge/dedupe after fanout
    pub join_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FanoutMode {
    Each,   // Execute second query once per value; concatenate (with optional dedupe)
    In,     // Inject array directly into filter (e.g., user_id: [..]) if schema supports it
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParallelAggregation {
    None,       // Return a map keyed by step id
    Union,      // Concatenate arrays; schemas must match
    Merge {     // Merge objects by join_key
        join_key: String,
        prefer: Prefer, // Left | Right | Error
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Prefer { Left, Right, Error }
```

#### 2. Variable System - Simple Query Composition

**Variables are the simple mechanism that makes queries composable.** Query results get stored in variables that can be used in subsequent queries.

**How Variables Work:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResultStore {
    /// Query results storage: query_id -> query_result
    pub results: HashMap<String, QueryResult>,
}

impl QueryResultStore {
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
        }
    }

    pub fn store(&mut self, id: String, data: Value) {
        self.results.insert(id.clone(), QueryResult {
            query_id: id,
            data,
            metadata: QueryMetadata {
                execution_time_ms: 0, // Will be set by executor
                rows_returned: 0,
                schemas_accessed: vec![],
                fields_accessed: vec![],
            },
        });
    }

    pub fn get(&self, id: &str) -> Option<&QueryResult> {
        self.results.get(id)
    }
}
```

**Direct Field Reference Example:**

Query 1:
```json
{
  "id": "get_department_users",
  "query": {
    "schema_name": "Department",
    "fields": ["user_ids"],
    "filter": {"department_name": "Engineering"}
  }
}
```

Query 2 references Query 1's result directly:
```json
{
  "id": "get_user_preferences",
  "query": {
    "schema_name": "UserPreferences",
    "fields": ["theme", "notifications", "language"],
    "filter": {"user_id": "${get_department_users.user_ids}"}
  }
}
```

**What This Does:**
1. Query 1 executes and returns `{"user_ids": ["alice", "bob", "charlie"]}`
2. Query 2 uses `${get_department_users.user_ids}` which directly references the field from Query 1's result
3. System substitutes with the actual value: `["alice", "bob", "charlie"]`
4. When a filter value is an array, the system automatically executes the query once for each array element
5. Returns combined results from all executions

**Even simpler - no variable declarations needed, just direct field references.**

#### 3. Execution Context

```rust
#[derive(Debug, Clone)]
pub struct QueryExecutionContext {
    /// Original request metadata
    pub pub_key: String,
    pub trust_distance: u32,
    /// Execution state
    pub execution_id: String,
    pub depth: u32,
    pub max_depth: u32,
    /// Results cache for optimization
    pub result_cache: HashMap<String, QueryResult>,
    /// Permission context
    pub permission_context: PermissionContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Query that produced this result
    pub query_id: String,
    /// Actual result data
    pub data: Value,
    /// Execution metadata
    pub metadata: QueryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    pub execution_time_ms: u64,
    pub rows_returned: usize,
    pub schemas_accessed: Vec<String>,
    pub fields_accessed: Vec<String>,
}
```

### Use Case Examples

#### Example 1: Index-Driven Query

**Scenario**: Get user preferences for all users in a specific department

```json
{
  "type": "sequential",
  "queries": [
    {
      "id": "get_department_users",
      "query": {
        "schema_name": "Department",
        "fields": ["user_ids"],
        "filter": {"department_name": "Engineering"}
      }
    },
    {
      "id": "get_user_preferences",
      "query": {
        "schema_name": "UserPreferences",
        "fields": ["theme", "notifications", "language"],
        "filter": {"user_id": "${get_department_users.user_ids}"}
      }
    }
  ]
}
```

**Step-by-Step Execution with Direct Field References**:

1. **Execute First Query**:
   ```sql
   -- Conceptual: Query Department schema
   SELECT user_ids FROM Department WHERE department_name = "Engineering"
   ```
   **Result stored as**: `get_department_users = {"user_ids": ["alice", "bob", "charlie"]}`

2. **Execute Second Query with Field Reference**:
   - Query has filter `{"user_id": "${get_department_users.user_ids}"}`
   - System looks up `get_department_users.user_ids` and substitutes: `{"user_id": ["alice", "bob", "charlie"]}`
   - Since filter value is an array, system creates separate queries:
     - Query A: `UserPreferences` with filter `{"user_id": "alice"}`
     - Query B: `UserPreferences` with filter `{"user_id": "bob"}`
     - Query C: `UserPreferences` with filter `{"user_id": "charlie"}`

3. **Execute Individual Queries**:
   ```sql
   -- Query A
   SELECT theme, notifications, language FROM UserPreferences WHERE user_id = "alice"
   -- Query B
   SELECT theme, notifications, language FROM UserPreferences WHERE user_id = "bob"
   -- Query C
   SELECT theme, notifications, language FROM UserPreferences WHERE user_id = "charlie"
   ```

4. **Return Combined Results**:
   ```json
   [
     {"user_id": "alice", "theme": "dark", "notifications": true, "language": "en"},
     {"user_id": "bob", "theme": "light", "notifications": false, "language": "es"},
     {"user_id": "charlie", "theme": "dark", "notifications": true, "language": "fr"}
   ]
   ```

**Key Point**: Direct field references eliminate the need for variable declarations. Just reference any previous query's result using `${query_id.field_name}`.

#### Example 2: Simple Field Reference

**Scenario**: Get user scores for a specific test

```json
{
  "type": "sequential",
  "queries": [
    {
      "id": "get_test_config",
      "query": {
        "schema_name": "TestConfig",
        "fields": ["max_score"],
        "filter": {"test_name": "math_quiz"}
      }
    },
    {
      "id": "get_user_scores",
      "query": {
        "schema_name": "UserScores",
        "fields": ["user_id", "score", "percentage"],
        "filter": {"max_score": "${get_test_config.max_score}"}
      }
    }
  ]
}
```

**Execution Flow**:
1. Query `TestConfig` schema → returns `{"max_score": 100}`
2. Reference field directly: `${get_test_config.max_score}` → `100`
3. Query `UserScores` with filter `{"max_score": 100}`

#### Example 3: Parallel Query Execution

**Scenario**: Get data from multiple sources simultaneously

```json
{
  "type": "parallel",
  "queries": [
    {
      "id": "get_user_profile",
      "query": {
        "schema_name": "UserProfile",
        "fields": ["user_id", "name", "email"],
        "filter": {"user_id": "user123"}
      }
    },
    {
      "id": "get_user_scores",
      "query": {
        "schema_name": "UserScores",
        "fields": ["total_score", "rank"],
        "filter": {"user_id": "user123"}
      }
    }
  ],
  "aggregation": {
    "Merge": {
      "join_key": "user_id",
      "prefer": "Left"
    }
  }
}
```

**Note:** This returns a merged object with `user_id` as the join key, preferring left side values in case of conflicts.

#### Example 3.5: Parallel Query with No Aggregation (Map Result)

**Scenario**: Get multiple independent datasets as a map

```json
{
  "type": "parallel",
  "queries": [
    {
      "id": "get_active_users",
      "query": {
        "schema_name": "User",
        "fields": ["user_id", "status"],
        "filter": {"status": "active"}
      }
    },
    {
      "id": "get_system_stats",
      "query": {
        "schema_name": "SystemMetrics",
        "fields": ["metric_name", "value"],
        "filter": {"category": "performance"}
      }
    }
  ],
  "aggregation": null
}
```

**Returns**: A map where each step ID maps to its result array:

```json
{
  "get_active_users": [
    {"user_id": "alice", "status": "active"},
    {"user_id": "bob", "status": "active"}
  ],
  "get_system_stats": [
    {"metric_name": "cpu_usage", "value": 45.2},
    {"metric_name": "memory_usage", "value": 67.8}
  ]
}
```

#### Example 4: Fanout Configuration

**Scenario**: Control how array values are handled during substitution

```json
{
  "type": "sequential",
  "queries": [
    {
      "id": "get_department_users",
      "query": {
        "schema_name": "Department",
        "fields": ["user_ids"],
        "filter": {"department_name": "Engineering"}
      }
    },
    {
      "id": "get_user_preferences",
      "fanout": {
        "mode": "Each",
        "dedupe": true,
        "max_values": 1000,
        "batch_size": 100,
        "concurrency": 10
      },
      "query": {
        "schema_name": "UserPreferences",
        "fields": ["theme", "notifications", "language"],
        "filter": {"user_id": "${get_department_users.user_ids}"}
      }
    }
  ]
}
```

**Note:** Within a single Parallel group, queries cannot reference each other's results; they execute simultaneously. Variable sharing across parallel *stages* may be introduced in Phase 2 via barriered stages (P1 runs in parallel, then P2 can reference P1 results), but not within the same group.

### Execution Engine Design

#### 1. Integration with Existing FoldDB Architecture

The composable query executor integrates with DataFold's existing architecture:

```rust
use crate::fold_db_core::{FoldDB, MessageBus};
use crate::schema::types::{Query, SchemaError};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComposableQueryResult {
    /// Single result from Simple or Sequential queries
    Single(Value),
    /// Map of results from Parallel queries with None aggregation
    Map(HashMap<String, Value>),
    /// Aggregated result from Parallel queries with Union/Merge
    Aggregated(Value),
}

pub struct ComposableQueryExecutor {
    /// Reference to existing FoldDB instance
    db: Arc<FoldDB>,
    /// Query result storage for variable substitution
    query_result_store: QueryResultStore,
    /// Message bus for event-driven operations
    message_bus: Arc<MessageBus>,
}

impl ComposableQueryExecutor {
    pub fn new(db: Arc<FoldDB>) -> Self {
        Self {
            message_bus: db.message_bus(),
            query_result_store: QueryResultStore::new(),
            db,
        }
    }
    
    pub fn execute(&mut self, composable_query: ComposableQuery, context: QueryExecutionContext) -> Result<ComposableQueryResult, SchemaError> {
        match composable_query {
            ComposableQuery::Simple(query) => {
                let result = self.execute_single_query(query, &context)?;
                Ok(ComposableQueryResult::Single(result))
            },
            ComposableQuery::Sequential { queries } => {
                let result = self.execute_sequential(queries, context)?;
                Ok(ComposableQueryResult::Single(result))
            },
            ComposableQuery::Parallel { queries, aggregation } => {
                self.execute_parallel(queries, aggregation, context)
            }
        }
    }
    
    fn execute_sequential(&mut self, queries: Vec<QueryStep>, context: QueryExecutionContext) -> Result<Value, SchemaError> {
        let mut final_result = None;
        
        for query_step in queries {
            // 1. Substitute field references in the query filter
            let resolved_query = self.resolve_field_references(query_step.query, &context)?;
            
            // 2. Execute query using existing FoldDB infrastructure
            let step_result = if let Some(fanout) = &query_step.fanout {
                self.execute_with_fanout(resolved_query, fanout)?
            } else {
                self.db.query(resolved_query)?
            };
            
            // 3. Store result for future references
            self.query_result_store.store(query_step.id.clone(), step_result.clone());
            final_result = Some(step_result);
        }
        
        final_result.ok_or_else(|| SchemaError::InvalidData("No queries provided".to_string()))
    }
    
    /// Leverage existing FoldDB query method
    fn execute_single_query(&self, query: Query, _context: &QueryExecutionContext) -> Result<Value, SchemaError> {
        self.db.query(query)
    }

    fn execute_with_fanout(&self, query: Query, fanout: &FanoutConfig) -> Result<Value, SchemaError> {
        // Implementation for handling array substitution with fanout behavior
        // - Each mode: split array into batches and execute query for each value
        // - In mode: inject array directly if schema supports it
        // Apply limits: max_values, batch_size, concurrency, dedupe
        // Return concatenated/merged results based on configuration
        todo!("Implement fanout execution logic")
    }

    fn execute_parallel(&mut self, queries: Vec<QueryStep>, aggregation: Option<ParallelAggregation>, context: QueryExecutionContext) -> Result<ComposableQueryResult, SchemaError> {
        // Execute all queries concurrently using existing infrastructure
        // Apply aggregation strategy to combine results
        todo!("Implement parallel execution logic")
    }
    
    fn resolve_field_references(&self, mut query: Query, _context: &QueryExecutionContext) -> Result<Query, SchemaError> {
        if let Some(filter) = &mut query.filter {
            self.substitute_field_refs_in_value(filter)?;
        }
        Ok(query)
    }

    fn substitute_field_refs_in_value(&self, value: &mut Value) -> Result<(), SchemaError> {
        match value {
            Value::String(s) if s.starts_with("${") && s.ends_with("}") => {
                let reference = &s[2..s.len()-1];
                let (step, field) = reference.split_once('.')
                    .ok_or_else(|| SchemaError::InvalidData(format!("Invalid reference: {reference}")))?;
                let obj = self.query_result_store.get(step)
                    .ok_or_else(|| SchemaError::InvalidData(format!("Unknown step: {step}")))?;
                let Value::Object(map) = &obj.data else {
                    return Err(SchemaError::InvalidData(format!("Referenced result for {step} is not an object")));
                };
                let Some(val) = map.get(field) else {
                    return Err(SchemaError::InvalidData(format!("Field not found: {step}.{field}")));
                };
                *value = val.clone();
            }
            Value::Object(m) => for v in m.values_mut() { self.substitute_field_refs_in_value(v)?; },
            Value::Array(a)  => for v in a.iter_mut()   { self.substitute_field_refs_in_value(v)?; },
            _ => {}
        }
        Ok(())
    }
}
```

### Execution Semantics

#### JSON encoding of enums
- `ParallelAggregation` is externally-tagged:
  - `{ "None": null }`, `{ "Union": null }`,
  - `{ "Merge": { "join_key": "user_id", "prefer": "Left" } }`
- `FanoutMode`: `"Each"` or `"In"`.

#### References
- Only Sequential steps may reference earlier steps using `${step_id.field}`.
- Forward/self references are invalid and rejected at validation.
- The referenced step must return a top-level object containing the referenced field. Referencing fields inside arrays of rows is unsupported in Phase 1.

#### Substitution Scope
- Substitution applies to `filter` only in Phase 1. (Extending to other fields may come later.)
- Types are preserved: numbers/booleans/arrays remain typed, not coerced to strings.

#### Fan-out
- Default `fanout.mode = Each`: when a substituted value is an array, execute the step once per value and concatenate results.
- Defaults: `max_values=1000`, `batch_size=100`, `concurrency=10`, `dedupe=true`.
- If `fanout.mode = In`, inject the whole array directly into the filter (requires schema support for IN-like semantics).

#### Short-circuiting
- If a referenced earlier step yields zero values for a required substitution, the downstream step short-circuits and returns an empty result.

#### Output Shape
- Sequential returns the **last step's** `rows` and `metadata`.
- Parallel returns:
  - `ParallelAggregation::None` → object keyed by step id
  - `Union` → concatenated array (schemas must match)
  - `Merge` → merged objects by `join_key` with `prefer` policy

### Validation & Limits

#### Validation (pre-execution)
- Validate caller permissions for all step schemas before any step executes.
- All step ids must be unique.
- `${step.field}` references must point to earlier steps.
- Referenced fields must be included in the earlier step's `fields`.
- Depth must be ≤ `max_depth`.
- Validate caller permissions for all step schemas up front before any execution.

#### Limits (defaults)
- `MAX_DEPTH = 5`
- `MAX_FANOUT_VALUES = 1000`
- `MAX_SECOND_QUERY_BYTES = 65536`
- `MAX_PARALLEL_CONCURRENCY = 10`

### Error Model

Standardized error codes:
- `CQ_REF_FORWARD` – reference to a future/self step
- `CQ_REF_UNKNOWN` – unknown step id
- `CQ_REF_FIELD_MISSING` – field not found in referenced result
- `CQ_TYPE_MISMATCH` – substituted type incompatible with target
- `CQ_MAX_DEPTH` – composition exceeds depth limit
- `CQ_MAX_FANOUT` – array size exceeds fanout limit
- `CQ_MAX_QUERY_SIZE` – serialized filter exceeds size limit
- `CQ_TIMEOUT` – step exceeded execution time
- `CQ_EMPTY_FIRST` – no steps provided (sequential)

#### 3. Security and Permission Model

```rust
pub struct ComposableQueryPermissions {
    base_permissions: PermissionWrapper,
}

impl ComposableQueryPermissions {
    /// Validate that user has permissions for all queries in composition
    pub fn validate_composition_permissions(&self, query: &ComposableQuery, context: &QueryExecutionContext) -> Result<(), QueryError> {
        // 1. Extract all schemas that will be accessed
        let accessed_schemas = self.extract_all_schemas(query);
        
        // 2. Validate permissions for each schema
        for schema in accessed_schemas {
            self.validate_schema_access(&schema, context)?;
        }
        
        // 3. Check for privilege escalation through composition
        self.validate_no_privilege_escalation(query, context)?;
        
        Ok(())
    }
    
    /// Ensure composition doesn't allow access to data that individual queries wouldn't allow
    fn validate_no_privilege_escalation(&self, query: &ComposableQuery, context: &QueryExecutionContext) -> Result<(), QueryError> {
        // Check that composition doesn't create unintended data access patterns
        // E.g., using public data to generate keys for private data access
    }
}
```

## API Changes

### 1. Integration with Existing Query Routes

```rust
// In src/datafold_node/query_routes.rs - extends existing functionality

use super::http_server::AppState;
use crate::schema::types::operations::Query;
use actix_web::{web, HttpResponse, Responder};
use serde_json::{json, Value};

/// Execute a composable query (extends existing execute_query endpoint)
pub async fn execute_composable_query(
    request: web::Json<ComposableQueryRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut node_guard = state.node.lock().await;
    
    // Create executor with existing FoldDB instance
    let mut executor = ComposableQueryExecutor::new(node_guard.get_fold_db());
    
    // Create execution context from request
    let context = QueryExecutionContext {
        pub_key: request.pub_key.clone().unwrap_or_else(|| "web-ui".to_string()),
        trust_distance: request.trust_distance.unwrap_or(0),
        max_execution_time_ms: request.options.max_execution_time_ms,
    };
    
    match executor.execute(request.query.clone(), context) {
        Ok(result) => HttpResponse::Ok().json(json!({"data": result})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to execute composable query: {}", e)})),
    }
}

/// Validate composable query structure (new endpoint)
pub async fn validate_composable_query(
    request: web::Json<ComposableQuery>,
    state: web::Data<AppState>,
) -> impl Responder {
    let node_guard = state.node.lock().await;
    
    // Validate query structure and permissions
    match ComposableQueryValidator::validate(&request, &node_guard) {
        Ok(validation_result) => HttpResponse::Ok().json(validation_result),
        Err(e) => HttpResponse::BadRequest()
            .json(json!({"error": format!("Query validation failed: {}", e)})),
    }
}
```

### 2. Request/Response Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposableQueryRequest {
    pub query: ComposableQuery,
    pub options: QueryOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOptions {
    pub max_execution_time_ms: Option<u64>,
    pub max_depth: Option<u32>,
    pub enable_caching: bool,
    pub return_execution_plan: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposableQueryResponse {
    pub result: QueryResult,
    pub execution_plan: Option<ExecutionPlan>,
    pub performance_metrics: PerformanceMetrics,
}
```

### 3. Frontend Integration

```typescript
// New API client methods
export interface ComposableQueryAPI {
  executeComposableQuery(query: ComposableQuery, options?: QueryOptions): Promise<ComposableQueryResponse>;
  validateComposableQuery(query: ComposableQuery): Promise<ValidationResult>;
}

// React component for building composable queries
export const ComposableQueryBuilder: React.FC<{
  onQueryExecute: (result: ComposableQueryResponse) => void;
}>;

// Example usage with direct field references
const composableQuery: ComposableQuery = {
  type: "sequential",
  queries: [
    {
      id: "get_users",
      query: {
        schema_name: "Department",
        fields: ["user_ids"],
        filter: { department: "Engineering" }
      }
    },
    {
      id: "get_preferences",
      query: {
        schema_name: "UserPreferences",
        fields: ["theme", "language"],
        filter: { user_id: "${get_users.user_ids}" }
      }
    }
  ]
};
```

## Implementation Roadmap

### Phase 1: Foundation (4-6 weeks)

1. **Core Types and Module Structure** (1 week)
   - Create new module `src/fold_db_core/composable/` for composable query components
   - Implement [`ComposableQuery`](docs/design/COMPOSABLE_QUERIES.md:35) enum in `src/fold_db_core/composable/types.rs`
   - Add [`QueryResultStore`](docs/design/COMPOSABLE_QUERIES.md:384) in `src/fold_db_core/composable/result_store.rs`
   - Create [`ComposableQueryExecutor`](docs/design/COMPOSABLE_QUERIES.md:381) in `src/fold_db_core/composable/executor.rs`

2. **Integration with Existing Architecture** (2 weeks)
   - Extend [`FoldDB`](src/fold_db_core/mod.rs:73) with composable query support
   - Integrate with existing [`MessageBus`](src/fold_db_core/mod.rs:83) for event-driven operations
   - Use existing [`PermissionWrapper`](src/fold_db_core/mod.rs:81) for security validation
   - Leverage existing [`query()`](src/fold_db_core/mod.rs:340) method for individual query execution

3. **Field Reference System** (1 week)
   - Implement `${query_id.field_name}` syntax parsing and substitution
   - Add support for array value handling and fanout
   - Integrate with existing [`SchemaError`](src/schema/types/errors.rs) error handling

4. **Testing and Validation** (1 week)
   - Unit tests using existing test infrastructure
   - Integration tests with real [`FoldDB`](src/fold_db_core/mod.rs:73) instances
   - Performance benchmarks against existing query system

### Phase 2: Advanced Features (4-6 weeks)

1. **Parallel Composition with Event System** (2 weeks)
   - Implement concurrent query execution using existing [`MessageBus`](src/fold_db_core/mod.rs:83)
   - Add result aggregation strategies (Union, Merge)
   - Integrate with [`EventMonitor`](src/fold_db_core/mod.rs:85) for parallel execution tracking

2. **Advanced Fanout and Array Handling** (2 weeks)
   - Implement [`FanoutConfig`](docs/design/COMPOSABLE_QUERIES.md:62) with batching and concurrency limits
   - Add support for complex array substitution patterns
   - Leverage existing range schema filtering for optimized array queries

3. **Observability and Performance** (2 weeks)
   - Integrate with existing [`EventMonitor`](src/fold_db_core/mod.rs:85) for execution metrics
   - Add composable query events to [`MessageBus`](src/fold_db_core/mod.rs:83)
   - Implement query result caching within [`QueryResultStore`](docs/design/COMPOSABLE_QUERIES.md:384)

### Phase 3: Production Features (3-4 weeks)

1. **HTTP API Integration** (1 week)
   - Extend existing [`query_routes.rs`](src/datafold_node/query_routes.rs) with composable endpoints
   - Integrate with existing [`AppState`](src/datafold_node/query_routes.rs:24) and [`DataFoldNode`](src/datafold_node/query_routes.rs:8)
   - Use existing error handling patterns and response formats

2. **Frontend Integration** (2 weeks)
   - Extend existing React UI in [`static-react/`](src/datafold_node/static-react) directory
   - Build composable query interface using existing UI patterns
   - Integrate with existing query execution infrastructure

3. **Production Readiness** (1 week)
   - Add comprehensive logging using existing [`LogFeature`](src/datafold_node/query_routes.rs:14) system
   - Performance testing with existing benchmark infrastructure
   - Documentation and deployment guides

## Performance Considerations

### 1. Query Optimization

- **Dependency Analysis**: Identify independent queries for parallel execution
- **Result Caching**: Cache intermediate results to avoid redundant queries
- **Query Pushdown**: Push filters as far down as possible in the execution tree
- **Lazy Evaluation**: Only execute queries whose results are actually needed

### 2. Resource Management

- **Memory Usage**: Limit intermediate result sizes and implement streaming where possible
- **Execution Limits**: Set maximum depth and execution time for compositions
- **Connection Pooling**: Reuse database connections across composed queries
- **Rate Limiting**: Prevent abuse through complex query compositions

### 3. Integration with Existing Observability

Composable queries integrate with DataFold's existing observability infrastructure:

```rust
use crate::fold_db_core::infrastructure::event_monitor::EventStatistics;
use crate::logging::features::{log_feature, LogFeature};

#[derive(Debug, Clone, Serialize)]
pub struct ComposableQueryMetrics {
    pub total_execution_time_ms: u64,
    pub individual_query_times: Vec<u64>,
    pub schemas_accessed: usize,
    pub total_rows_processed: usize,
    pub fanout_operations: usize,
    pub cache_hit_ratio: f64,
    pub event_statistics: EventStatistics,
}

impl ComposableQueryExecutor {
    fn log_execution_metrics(&self, metrics: &ComposableQueryMetrics) {
        log_feature!(LogFeature::Query, info,
            "Composable query execution completed in {}ms, {} schemas accessed, {} rows processed",
            metrics.total_execution_time_ms, metrics.schemas_accessed, metrics.total_rows_processed
        );
    }
}
```

## Integration with Current Codebase

### 1. Module Structure

The composable query system will be added as a new module within the existing architecture:

```
src/fold_db_core/
├── composable/               # New module for composable queries
│   ├── mod.rs               # Module exports
│   ├── types.rs             # ComposableQuery, QueryStep, etc.
│   ├── executor.rs          # ComposableQueryExecutor
│   ├── result_store.rs      # QueryResultStore
│   ├── field_resolver.rs    # Field reference resolution
│   └── validator.rs         # Query validation and security
├── infrastructure/          # Existing infrastructure
├── managers/               # Existing managers
└── services/               # Existing services
```

### 2. Dependencies

Add to [`Cargo.toml`](Cargo.toml):

```toml
# Already available - no new dependencies needed
# - serde/serde_json for serialization
# - uuid for correlation IDs
# - tokio for async operations
# - log for logging integration
```

### 3. Integration Points

```rust
// In src/fold_db_core/mod.rs - extend existing FoldDB
impl FoldDB {
    /// Execute a composable query
    pub fn execute_composable_query(&self, query: ComposableQuery, context: QueryExecutionContext) -> Result<ComposableQueryResult, SchemaError> {
        let mut executor = ComposableQueryExecutor::new(Arc::new(self.clone()));
        executor.execute(query, context)
    }
}

// In src/datafold_node/mod.rs - extend DataFoldNode
impl DataFoldNode {
    pub fn execute_composable_query(&mut self, query: ComposableQuery) -> Result<Value, SchemaError> {
        let context = QueryExecutionContext {
            pub_key: "node".to_string(),
            trust_distance: 0,
            max_execution_time_ms: Some(30000),
        };
        
        match self.fold_db.execute_composable_query(query, context)? {
            ComposableQueryResult::Single(result) => Ok(result),
            ComposableQueryResult::Aggregated(result) => Ok(result),
            ComposableQueryResult::Map(results) => Ok(serde_json::to_value(results)?),
        }
    }
}
```

When `options.return_execution_plan` or debug is enabled, the response includes an `execution_trace`:
```json
{
  "trace": [
    {"id": "get_department_users", "rows": 3, "ms": 14},
    {"id": "get_user_preferences", "fanout": {"mode":"Each","values":3,"batches":1}, "rows": 3, "ms": 28}
  ]
}
```
**Logging**: Log redacted hash of materialized filter values for reproducibility without leaking sensitive data. Include step execution order, fanout decisions, and error locations for debugging.

## Security Considerations

### 1. Permission Inheritance

- Effective permission = intersection of caller permissions and each step’s schema policy.
- No privilege amplification: second (or later) steps cannot broaden scope beyond caller.
- trust_distance for the composition = max(trust_distance across all steps).
- Prevent existence probing: obtaining keys from public data does not grant access to private schemas without explicit permission.

### 2. Information Leakage Prevention

- Prevent using public query results to infer private data existence
- Limit composition depth to prevent complex attack vectors  
- Audit trail for all composed query executions

### 3. Resource Protection

- Query composition cannot be used for denial-of-service attacks
- Execution time and resource limits enforced
- Complex compositions require higher privilege levels

## Migration Strategy

### 1. Backward Compatibility

- All existing [`Query`](src/schema/types/operations.rs:7) structures continue to work unchanged
- [`ComposableQuery::Simple`](docs/design/COMPOSABLE_QUERIES.md:37) wraps existing queries
- Gradual migration path for applications

### 2. Feature Rollout

1. **Beta Release**: Advanced users and internal testing
2. **Limited Release**: Selected schemas and use cases
3. **General Availability**: Full feature set available

### 3. Performance Validation

- Benchmark against existing query performance
- Ensure no regression in simple query execution
- Validate scalability with complex compositions

## Conclusion

The composable query system extends DataFold's querying capabilities while maintaining security, performance, and the existing trust model. The design provides a solid foundation for complex data access patterns while ensuring the system remains secure and performant.

The phased implementation approach allows for iterative development and validation, ensuring each component is thoroughly tested before building additional complexity on top.