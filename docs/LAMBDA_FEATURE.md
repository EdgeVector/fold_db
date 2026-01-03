# Lambda Feature APIs

The `lambda` feature in the `datafold` crate provides a simplified, high-performance interface for running DataFold nodes within AWS Lambda. It is designed to minimize cold start times and handle the stateless nature of Lambda functions while supporting both single-tenant (standard) and multi-tenant (DynamoDB-backed) deployments.

## Enabling the Feature

To use these APIs, enable the `lambda` feature in your `Cargo.toml`:

```toml
[dependencies]
datafold = { version = "...", features = ["lambda"] }
```

This feature automatically pulls in `aws-backend` dependencies and the `lambda_runtime`.

## Core Components

The API is exposed under `datafold::lambda`.

### 1. `LambdaConfig`

`LambdaConfig` is the primary configuration builder. It allows you to specify storage backends, logging preferences, and optional AI/LLM integration.

#### Usage

```rust
use datafold::lambda::{LambdaConfig, LambdaLogging};
use datafold::StorageConfig;
use std::path::PathBuf;

// Basic configuration with Local storage and Stdout logging
let config = LambdaConfig::new(
    StorageConfig::Local { path: PathBuf::from("/tmp/folddb") },
    LambdaLogging::Stdout
);

// Advanced configuration with DynamoDB storage, AI enabled, and Schema Service
let config = LambdaConfig::new(
    StorageConfig::DynamoDb(dynamo_config),
    LambdaLogging::DynamoDb // Logs to DynamoDB table
)
.with_schema_service_url("https://schema.api.datafold.ai".to_string())
.with_openrouter("api-key".to_string(), "model-name".to_string());
```

**Key Methods:**

- `new(storage: StorageConfig, logging: LambdaLogging)`: Create a basic config.
- `with_db_ops(db_ops: Arc<DbOperations>, logging: LambdaLogging)`: Use a pre-initialized `DbOperations` instance (useful for custom backends).
- `with_schema_service_url(url: String)`: Set the URL for the schema registry service.
- `with_openrouter(api_key, model)`: Enable AI query capabilities via OpenRouter.
- `with_ollama(base_url, model)`: Enable AI query capabilities via Ollama.

### 2. `LambdaContext`

`LambdaContext` is a singleton manager that holds the `DataFoldNode`, `ProgressTracker`, and other shared state. It MUST be initialized once during the Lambda cold start phase.

#### Initialization

```rust
use datafold::lambda::{LambdaContext, LambdaConfig};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // 1. Create Config
    let config = LambdaConfig::new(...);

    // 2. Initialize Global Context (Cold Start)
    LambdaContext::init(config).await.expect("Failed to init context");

    // 3. Start Lambda Runtime
    run(service_fn(function_handler)).await
}
```

#### Accessing the Node

Inside your Lambda handler, you can access the specialized components:

```rust
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Get the default node (single-tenant)
    let node = LambdaContext::node().await?;

    // Get a specific user's node (multi-tenant)
    let user_node = LambdaContext::get_node("user-123").await?;

    // Get the progress tracker for ingestion status
    let tracker = LambdaContext::progress_tracker()?;

    // ... logic ...
}
```

### 3. UI Serving (`ui`)

The `lambda::ui` module provides helpers for serving the DataFold React application directly from the Lambda function, allowing for serverless UI hosting.

#### `get_ui_asset`

This function handles routing for static assets and implements the "SPA fallback" pattern (serving `index.html` for unknown routes) automatically.

```rust
use datafold::lambda::ui::get_ui_asset;

async fn function_handler(req: Request) -> Result<Response<Body>, Error> {
    let path = req.uri().path();

    // Serve UI assets
    if let Some(asset) = get_ui_asset(path) {
        return Ok(Response::builder()
            .header("content-type", asset.mime_type)
            .body(Body::from(asset.content))
            .unwrap());
    }

    // Handle API routes...
}
```

### 4. Logging

The `lambda::logging` module and `LambdaConfig` support specialized logging strategies:

- **Stdout**: Standard logging to CloudWatch Logs (via stdout).
- **DynamoDB**: Structs logs are written to a DynamoDB table (useful for querying logs in the UI).
- **UserLogger**: A scoped logger that attaches `user_id` to log entries, critical for multi-tenant debugging.

```rust
// Access a user-scoped logger in your handler
let user_logger = LambdaContext::get_user_logger("user-123")?;
user_logger.info("Processing request for user");
// Log entry will look like: { "msg": "...", "user_id": "user-123", ... }
```

## Example Structure

A typical `main.rs` for a DataFold Lambda:

```rust
use datafold::lambda::{LambdaConfig, LambdaContext, LambdaLogging};
use lambda_http::{run, service_fn, Body, Error, Request, Response};

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Handle request...
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Setup tracing/logging
    tracing_subscriber::fmt().init();

    // Configure DataFold
    let config = LambdaConfig::new(
        StorageConfig::from_env().expect("Missing storage config"),
        LambdaLogging::DynamoDb
    );

    // Init Context
    LambdaContext::init(config).await.expect("Context init failed");

    // Run Handler
    run(service_fn(function_handler)).await
}
```
