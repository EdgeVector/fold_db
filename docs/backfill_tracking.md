# Backfill Tracking System

## Overview

The backfill tracking system provides comprehensive monitoring and management of transform backfill operations. When a transform schema is approved, the system automatically initiates a backfill process to apply the transform to all existing source data. This document describes the architecture, workflow, and implementation details of the backfill tracking system.

## Architecture

### Components

1. **BackfillTracker** (`src/fold_db_core/infrastructure/backfill_tracker.rs`)
   - Manages backfill lifecycle and state
   - Tracks progress and statistics
   - Handles failure detection and recovery

2. **EventMonitor** (`src/fold_db_core/infrastructure/event_monitor.rs`)
   - Listens for schema approval events
   - Coordinates backfill execution
   - Monitors mutation completion

3. **Message Bus Events**
   - `SchemaApproved`: Triggers backfill initiation
   - `BackfillExpectedMutations`: Sets expected mutation count
   - `MutationExecuted`: Tracks individual mutation completion
   - `BackfillMutationFailed`: Records mutation failures

## Backfill Hash System

### Purpose

Each backfill operation is assigned a unique **backfill hash** to ensure:
- Multiple backfills can run on the same transform schema
- Each approval generates a distinct backfill operation
- Mutations can be traced back to their originating backfill
- Concurrent backfills don't interfere with each other

### Hash Generation

The backfill hash is generated using:
```rust
fn generate_backfill_hash(transform_id: &str, source_schema: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time error")
        .as_nanos();
    
    let input = format!("{}:{}:{}", transform_id, source_schema, timestamp);
    let hash = seahash::hash(input.as_bytes());
    
    format!("backfill_{:016x}", hash)
}
```

**Components:**
- `transform_id`: The transform schema name (e.g., "BlogPostWordIndex")
- `source_schema`: The source data schema (e.g., "BlogPost")
- `timestamp`: Nanosecond precision timestamp
- `seahash`: Stable, high-quality hash function

**Format:** `backfill_XXXXXXXXXXXXXXXX` (16 hex characters)

**Example:** `backfill_9ca89bb0390f7182`

### Why Seahash?

We use [seahash](https://crates.io/crates/seahash) instead of Rust's `DefaultHasher` because:
- **Stability**: Hash values are consistent across Rust versions
- **Quality**: Excellent avalanche and distribution properties
- **Speed**: Optimized for modern CPUs
- **Determinism**: Same input always produces same output

## Backfill Workflow

### 1. Schema Approval

When a transform schema is approved:

```rust
// User approves schema via API
POST /api/schema/BlogPostWordIndex/approve

// Server checks if it's a transform schema
let is_transform = transform_manager.transform_exists("BlogPostWordIndex")?;

// Generate unique backfill hash
let backfill_hash = BackfillTracker::generate_hash(
    "BlogPostWordIndex",
    "BlogPost"
);

// Approve schema with backfill hash
schema_manager.set_schema_state_with_backfill(
    "BlogPostWordIndex",
    SchemaState::Approved,
    Some(backfill_hash)
)?;

// Return hash to client
{ "success": true, "backfill_hash": "backfill_9ca89bb0390f7182" }
```

### 2. Backfill Initiation

The EventMonitor receives the `SchemaApproved` event:

```rust
// Start tracking the backfill
backfill_tracker.start_backfill_with_hash(
    backfill_hash,
    transform_id,
    source_schema
);

// Scan source schema for all records
let source_records = scan_source_schema(source_schema);
let expected_mutations = source_records.len();

// Publish expected count
message_bus.publish(BackfillExpectedMutations {
    transform_id,
    backfill_hash,
    count: expected_mutations
});

// Create mutations with backfill context
for record in source_records {
    let mutation = create_mutation_with_context(
        record,
        backfill_hash.clone()
    );
    execute_mutation(mutation);
}
```

### 3. Progress Tracking

As mutations are executed:

```rust
// On successful mutation
message_bus.publish(MutationExecuted {
    schema: transform_id,
    mutation_context: Some(MutationContext {
        backfill_hash: Some(backfill_hash),
        ...
    }),
    ...
});

// EventMonitor updates progress
backfill_tracker.increment_mutation_completed(&backfill_hash);

// Check if complete
if mutations_completed >= mutations_expected {
    backfill.status = BackfillStatus::Completed;
}
```

### 4. Failure Detection

The system monitors for failures:

```rust
// If mutation fails
message_bus.publish(BackfillMutationFailed {
    backfill_hash,
    error: "Mutation failed: ..."
});

// Tracker increments failure count
backfill_tracker.increment_mutation_failed(&backfill_hash, error);

// Check failure rate
let total = mutations_completed + mutations_failed;
let failure_rate = mutations_failed as f64 / total as f64;

// If failure rate > 10% and total > 10, mark as failed
if failure_rate > 0.1 && total > 10 {
    backfill.status = BackfillStatus::Failed;
    backfill.error = Some(format!("Backfill failed: {} mutations failed", mutations_failed));
}
```

### 5. Completion

Backfill completes when:
- All expected mutations are processed: `mutations_completed >= mutations_expected`
- Zero mutations expected (no source data): Immediately marked as `Completed`
- Failure threshold exceeded: Marked as `Failed`

## Backfill States

```rust
pub enum BackfillStatus {
    /// Backfill is currently in progress
    InProgress,
    /// Backfill completed successfully
    Completed,
    /// Backfill failed with error
    Failed,
}
```

### State Transitions

```
                ┌─────────────┐
                │  (Start)    │
                └──────┬──────┘
                       │
                       ▼
              ┌────────────────┐
              │   InProgress   │◄─────┐
              └────────┬───────┘      │
                       │              │
        ┌──────────────┼──────────────┴────────────┐
        │              │                            │
        ▼              ▼                            ▼
  ┌──────────┐   ┌──────────┐            ┌────────────────┐
  │ Expected │   │ All Done │            │ Failure Rate   │
  │  = 0     │   │ Tracking │            │    > 10%       │
  └────┬─────┘   └────┬─────┘            └────────┬───────┘
       │              │                            │
       ▼              ▼                            ▼
  ┌─────────────┐  ┌─────────────┐        ┌──────────────┐
  │  Completed  │  │  Completed  │        │    Failed    │
  └─────────────┘  └─────────────┘        └──────────────┘
```

## API Endpoints

### Get Backfill Status by Hash

```http
GET /api/backfill/{hash}

Response:
{
  "data": {
    "backfill_hash": "backfill_9ca89bb0390f7182",
    "transform_id": "BlogPostWordIndex",
    "source_schema": "BlogPost",
    "status": "Completed",
    "records_produced": 450,
    "mutations_expected": 450,
    "mutations_completed": 450,
    "mutations_failed": 0,
    "start_time": 1696723200,
    "end_time": 1696723205,
    "duration_seconds": 5,
    "error": null
  }
}
```

### Get All Backfills

```http
GET /api/transforms/backfills

Response: [
  {
    "backfill_hash": "backfill_9ca89bb0390f7182",
    "transform_id": "BlogPostWordIndex",
    "source_schema": "BlogPost",
    "status": "Completed",
    ...
  },
  ...
]
```

### Get Active Backfills

```http
GET /api/transforms/backfills/active

Response: [ ... ] // Only InProgress backfills
```

### Get Backfill Statistics

```http
GET /api/transforms/backfills/statistics

Response:
{
  "total_backfills": 15,
  "active_backfills": 2,
  "completed_backfills": 12,
  "failed_backfills": 1,
  "total_mutations_expected": 50000,
  "total_mutations_completed": 48500,
  "total_mutations_failed": 125,
  "total_records_produced": 48500
}
```

## Data Structures

### BackfillInfo

```rust
pub struct BackfillInfo {
    /// Unique hash identifying this specific backfill operation
    pub backfill_hash: String,
    
    /// Transform ID being backfilled
    pub transform_id: String,
    
    /// Source schema name
    pub source_schema: String,
    
    /// Current status
    pub status: BackfillStatus,
    
    /// Items processed so far (source records scanned)
    pub items_processed: u64,
    
    /// Total items to process (if known)
    pub items_total: Option<u64>,
    
    /// When the backfill started (Unix timestamp)
    pub start_time: u64,
    
    /// When the backfill completed (if finished)
    pub end_time: Option<u64>,
    
    /// Error message if failed
    pub error: Option<String>,
    
    /// Records produced by the backfill
    pub records_produced: u64,
    
    /// Expected number of mutations to be created
    pub mutations_expected: u64,
    
    /// Number of mutations completed so far
    pub mutations_completed: u64,
    
    /// Number of mutations that failed
    pub mutations_failed: u64,
}
```

## Performance Considerations

### Memory

- Backfills are stored in-memory in a `HashMap<String, BackfillInfo>`
- Cleanup runs hourly, keeping the last 100 completed backfills
- Active backfills are never cleaned up

### Scalability

- Each backfill tracks approximately 200 bytes of data
- 100 backfills = ~20KB memory
- Suitable for thousands of backfills before memory pressure

### Concurrency

- Multiple backfills can run simultaneously
- Each backfill has a unique hash preventing collisions
- Message bus ensures thread-safe event delivery

## Frontend Integration

### Schema Approval with Backfill Hash

```typescript
// Approve schema
const result = await schemaClient.approveSchema("BlogPostWordIndex");

if (result.success && result.data.backfill_hash) {
  console.log("Backfill started:", result.data.backfill_hash);
  
  // Track progress
  pollBackfillStatus(result.data.backfill_hash);
}
```

### BackfillMonitor Component

The `BackfillMonitor` component provides real-time visibility:

```jsx
<BackfillMonitor />
```

Features:
- Live backfill status updates
- Progress bars for active backfills
- Mutation-level statistics
- Error reporting
- Historical backfill data

## Testing

### Unit Tests

```bash
# Test backfill tracker
cargo test --test backfill_failure_test

# Test zero-data backfills
cargo test --test backfill_on_approval_test
```

### Integration Tests

```bash
# Full backfill workflow
cargo test --test blogpost_backfill_integration_test

# HTTP API integration
cargo test --test http_transform_registration_backfill_test
```

## Error Handling

### Common Issues

1. **No Source Data**
   - Backfill completes immediately with `mutations_expected = 0`
   - Status: `Completed` with 0 records produced

2. **Transform Execution Failures**
   - Individual mutation failures are tracked
   - Failure rate > 10% triggers backfill failure
   - Error message captured in `BackfillInfo.error`

3. **System Failures**
   - Backfill state persists in-memory only
   - Server restart loses active backfill state
   - Re-approval triggers new backfill with new hash

### Recovery

If a backfill fails:
1. Check the error message in `BackfillInfo.error`
2. Fix the underlying issue (transform logic, data format, etc.)
3. Block the schema to prevent queries
4. Fix the transform definition
5. Re-approve the schema to trigger a new backfill

## Best Practices

1. **Monitor Backfill Progress**
   - Use the BackfillMonitor UI component
   - Check `/api/transforms/backfills/active` for long-running backfills

2. **Test Transforms Before Approval**
   - Create a subset of test data
   - Verify transform logic works correctly
   - Approve only when confident

3. **Handle Large Datasets**
   - Backfills process all source data synchronously
   - For very large datasets (>1M records), consider:
     - Breaking into smaller schemas
     - Running during off-peak hours
     - Monitoring system resources

4. **Track Backfill Hashes**
   - Save returned backfill hash from approval
   - Use hash for specific backfill queries
   - Log hash in operational dashboards

## Future Enhancements

Potential improvements:
- Persistent backfill state (survive restarts)
- Pausable/resumable backfills
- Backfill priority queues
- Rate limiting for large backfills
- Backfill metrics and dashboards
- Automatic retry on transient failures

## Related Documentation

- [Transform Functions](transform_functions.md)
- [LLM Query Workflow](llm_query_workflow.md)
- [Project Logic](project_logic.md)

