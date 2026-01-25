# Lambda Multi-Tenancy Architecture

## Overview

This document describes the multi-tenancy architecture for Lambda/stateless deployments.

## Single Source of Truth: Request Context

The **user_id** is obtained dynamically from the HTTP request context, NOT from configuration.

### Flow

```
HTTP Request
     │
     ▼
┌─────────────────────────────────────┐
│ UserContextMiddleware               │
│ - Extracts `x-user-id` header       │
│ - Calls run_with_user(user_id, ...) │
│ - Sets task-local CURRENT_USER_ID   │
└─────────────────────────────────────┘
     │
     ▼
┌─────────────────────────────────────┐
│ Storage Layer Operations            │
│ - get_current_user_id() called      │
│ - Returns user from request context │
│ - Falls back to default if needed   │
└─────────────────────────────────────┘
     │
     ▼
┌─────────────────────────────────────┐
│ DynamoDB                            │
│ - PK = user_id:key or user_id       │
│ - Multi-tenant data isolation       │
└─────────────────────────────────────┘
```

## Key Components

### 1. Request Context (logging/core.rs)

```rust
// Set user context for a request
run_with_user(&user_id, async { ... }).await

// Get current user from any code
get_current_user_id() -> Option<String>
```

### 2. Storage Stores (dynamodb_backend.rs, dynamodb_store.rs)

All DynamoDB stores now get `user_id` dynamically:

```rust
fn get_current_user_id(&self) -> String {
    crate::logging::core::get_current_user_id()
        .unwrap_or_else(|| self.default_user_id.clone())
}
```

- `DynamoDbKvStore` - Uses `get_current_user_id()` for PK
- `DynamoDbNativeIndexStore` - Uses `get_current_user_id()` for PK
- `DynamoDbNamespacedStore` - Uses `get_current_user_id()` for PK
- `DynamoDbSchemaStore` - Uses `get_current_user_id()` for PK

### 3. Background Tasks (ingestion/routes.rs, ingestion_spawner.rs)

When spawning background tasks with `tokio::spawn`, user context must be propagated:

```rust
let user_id_for_task = get_current_user_id().unwrap_or("unknown".to_string());

tokio::spawn(async move {
    run_with_user(&user_id_for_task, async move {
        // Background work here has correct user context
    }).await
});
```

### 4. Event Orchestration (EventEnvelope)

Events published across service boundaries carry user context via `EventEnvelope`:

```rust
use crate::fold_db_core::infrastructure::{Event, EventEnvelope};

// Create envelope with current user context (automatic)
let envelope = EventEnvelope::new(event);

// Create envelope with explicit user_id
let envelope = EventEnvelope::with_user(event, "user123".to_string());

// Serialize for transport (SNS, SQS, HTTP)
let bytes = envelope.to_bytes()?;

// Deserialize received message
let envelope = EventEnvelope::from_bytes(&bytes)?;

// Process with restored user context
envelope.process_with_context(|event| async move {
    // Storage operations here will use the envelope's user_id
    handle_event(event).await
}).await;
```

The `EventEnvelope` struct:

```rust
pub struct EventEnvelope {
    pub event: Event,              // The wrapped event
    pub user_id: Option<String>,   // User ID for multi-tenant isolation
    pub correlation_id: Option<String>, // Optional tracing ID
    pub timestamp_ms: u64,         // Creation timestamp
}

impl EventEnvelope {
    // Create with current user context
    pub fn new(event: Event) -> Self;

    // Create with explicit user_id
    pub fn with_user(event: Event, user_id: String) -> Self;

    // Add correlation ID for tracing
    pub fn with_correlation_id(self, id: String) -> Self;

    // Serialize to JSON bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error>;

    // Deserialize from JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error>;

    // Process event with restored user context
    pub async fn process_with_context<F, Fut, T>(self, f: F) -> T;
}
```

## Configuration

### Config user_id

The `cloud_config.user_id` in config is now used as:

- **Default fallback** for operations without request context (startup, background tasks without context)
- **NOT the source of truth** for data isolation

### For Development

The frontend generates a hash of the login identifier and sends it as `x-user-id` header.

### For Production (Lambda)

AWS API Gateway + Cognito sets the `x-user-id` header from the authenticated user.

## Benefits

1. **True stateless operation** - Node can be created/destroyed per request
2. **Multi-user support** - Different users can share same Lambda
3. **Data isolation** - Each user's data is partitioned by user_id in DynamoDB
4. **Simple migration** - Existing code works with new architecture
5. **Event tracing** - EventEnvelope preserves user context across distributed systems

## Files Modified

### Storage Layer

- `src/storage/dynamodb_backend.rs` - Dynamic user_id in DynamoDbKvStore, DynamoDbNativeIndexStore, DynamoDbNamespacedStore
- `src/storage/dynamodb_store.rs` - Dynamic user_id in DynamoDbSchemaStore

### Event System

- `src/fold_db_core/infrastructure/message_bus/events.rs` - Added EventEnvelope with serialization + process_with_context
- `src/fold_db_core/infrastructure/message_bus/async_bus.rs` - Added create_envelope helpers
- `src/fold_db_core/infrastructure/mod.rs` - Exported EventEnvelope

### Background Tasks

- `src/ingestion/routes.rs` - User context propagation to spawned tasks
- `src/ingestion/ingestion_spawner.rs` - User context propagation to spawned tasks

## Usage Pattern for Distributed Events

When implementing SNS/SQS-based event processing:

### Publisher (sending events)

```rust
// When publishing to external systems
let event = Event::MutationExecuted(mutation_event);
let envelope = EventEnvelope::new(event);  // Captures current user_id
let bytes = envelope.to_bytes()?;

// Send to SNS/SQS
sns_client.publish().message(base64::encode(&bytes)).send().await?;
```

### Consumer (receiving events)

```rust
// When receiving from SQS
let bytes = base64::decode(&sqs_message.body)?;
let envelope = EventEnvelope::from_bytes(&bytes)?;

// Process with correct user context
envelope.process_with_context(|event| async move {
    match event {
        Event::MutationExecuted(mutation) => {
            // Storage operations use envelope's user_id
            db.save_mutation(mutation).await
        }
        _ => Ok(())
    }
}).await?;
```
