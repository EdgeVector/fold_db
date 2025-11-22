# DataFold Lambda Quick Start

Use DataFold in AWS Lambda functions with minimal setup.

## Installation

Add to your Lambda project's `Cargo.toml`:

```toml
[dependencies]
datafold = { version = "0.1.16", features = ["lambda"] }
lambda_runtime = "0.13"
tokio = { version = "1", features = ["macros"] }
serde_json = "1"
```

## Basic Usage

```rust
use datafold::lambda::{LambdaConfig, LambdaContext};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde_json::{json, Value};

// Lambda handler - called for each invocation
async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    // Access the DataFold node
    let node = LambdaContext::node()?;
    
    // Your logic here...
    let node_guard = node.lock().await;
    let node_id = node_guard.get_node_id().to_string();
    drop(node_guard);
    
    Ok(json!({
        "statusCode": 200,
        "body": { "message": "Success", "node_id": node_id }
    }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize once during cold start
    let config = LambdaConfig::new();
    LambdaContext::init(config).await?;
    
    // Run Lambda runtime
    run(service_fn(handler)).await
}
```

## Configuration

### Default (No Configuration)
```rust
let config = LambdaConfig::new();
LambdaContext::init(config).await?;
```

### With Schema Service
```rust
let config = LambdaConfig::new()
    .with_schema_service_url("https://your-schema-service.com".to_string());
LambdaContext::init(config).await?;
```

### From Environment Variable
```rust
let mut config = LambdaConfig::new();
if let Ok(url) = std::env::var("SCHEMA_SERVICE_URL") {
    config = config.with_schema_service_url(url);
}
LambdaContext::init(config).await?;
```

## Build for Lambda

### x86_64 (Intel/AMD)
```bash
cargo build --release --target x86_64-unknown-linux-gnu --features lambda
```

### ARM64 (Graviton)
```bash
cargo build --release --target aarch64-unknown-linux-gnu --features lambda
```

## Deploy

### Create Deployment Package
```bash
# Copy binary and rename to 'bootstrap'
cp target/x86_64-unknown-linux-gnu/release/YOUR_BINARY bootstrap

# Create zip
zip lambda.zip bootstrap
```

### Deploy with AWS CLI
```bash
aws lambda create-function \
  --function-name my-datafold-function \
  --runtime provided.al2 \
  --role arn:aws:iam::YOUR_ACCOUNT:role/lambda-role \
  --handler bootstrap \
  --zip-file fileb://lambda.zip \
  --timeout 300 \
  --memory-size 512
```

## S3 Event Example

```rust
use datafold::lambda::LambdaContext;
use datafold::ingestion::json_processor::{convert_file_to_json, flatten_root_layers};
use serde_json::json;

async fn s3_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    // Parse S3 event
    let bucket = event.payload["Records"][0]["s3"]["bucket"]["name"]
        .as_str().ok_or("Missing bucket")?;
    let key = event.payload["Records"][0]["s3"]["object"]["key"]
        .as_str().ok_or("Missing key")?;
    
    // Download file from S3 (implement your S3 download logic)
    let file_path = download_from_s3(bucket, key).await?;
    
    // Convert file to JSON
    let json_value = convert_file_to_json(&file_path).await?;
    
    // Flatten unnecessary wrapper layers
    let flattened_json = flatten_root_layers(json_value);
    
    // Ingest using Lambda context
    let progress_id = LambdaContext::ingest_json(
        flattened_json,
        true,  // auto_execute
        0,     // trust_distance
        "default".to_string()
    ).await?;
    
    Ok(json!({ 
        "statusCode": 200, 
        "body": { "progress_id": progress_id }
    }))
}
```

## Direct Ingestion API (Recommended)

### Async Ingestion (Returns Immediately)
```rust
use datafold::lambda::LambdaContext;
use serde_json::json;

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let data = json!([
        {"id": 1, "name": "Alice", "email": "alice@example.com"},
        {"id": 2, "name": "Bob", "email": "bob@example.com"}
    ]);
    
    // Start ingestion in background
    let progress_id = LambdaContext::ingest_json(
        data,
        true,              // auto_execute
        0,                 // trust_distance
        "default".to_string()  // pub_key
    ).await?;
    
    Ok(json!({
        "statusCode": 200,
        "body": { "progress_id": progress_id }
    }))
}
```

### Sync Ingestion (Waits for Completion)
```rust
use datafold::lambda::LambdaContext;
use serde_json::json;

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let data = json!([
        {"id": 1, "name": "Alice"},
        {"id": 2, "name": "Bob"}
    ]);
    
    // Wait for ingestion to complete
    let response = LambdaContext::ingest_json_sync(
        data,
        true,              // auto_execute
        0,                 // trust_distance
        "default".to_string()  // pub_key
    ).await?;
    
    Ok(json!({
        "statusCode": 200,
        "body": {
            "success": response.success,
            "mutations_executed": response.mutations_executed,
            "schema_used": response.schema_used
        }
    }))
}
```

## Accessing DataFold Components

### Get Node
```rust
let node = LambdaContext::node()?;
let node_guard = node.lock().await;
// Use node...
drop(node_guard);
```

### Get Progress Tracker
```rust
let tracker = LambdaContext::progress_tracker()?;
```

### Check Progress
```rust
if let Some(progress) = LambdaContext::get_progress(&progress_id)? {
    println!("Status: {:?}", progress.current_step);
}
```

## Lambda Configuration

### Memory
- **Minimum**: 512 MB
- **Recommended**: 1024 MB for heavy processing

### Timeout
- **Quick operations**: 30 seconds
- **Data processing**: 2-5 minutes
- **Batch processing**: 5-15 minutes

### Environment Variables
```bash
# Optional
SCHEMA_SERVICE_URL=https://your-schema-service.com
RUST_LOG=info  # For logging
```

## Common Patterns

### Simple Processing
```rust
async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let node = LambdaContext::node()?;
    // Process event...
    Ok(json!({"statusCode": 200}))
}
```

### With Error Handling
```rust
async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    match process_event(&event).await {
        Ok(result) => Ok(json!({"statusCode": 200, "body": result})),
        Err(e) => Ok(json!({"statusCode": 500, "body": e.to_string()}))
    }
}
```

## Troubleshooting

### "Context not initialized"
Make sure `LambdaContext::init()` is called in `main()` before the handler runs.

### Out of Memory
Increase Lambda memory allocation or optimize data processing.

### Timeout
Increase Lambda timeout or process data in smaller chunks.

## Complete Example

See `examples/lambda_s3_ingestion.rs` for a complete working example.

## More Info

- Full guide: [docs/LAMBDA_INTEGRATION.md](docs/LAMBDA_INTEGRATION.md)
- Example with Dockerfile: [examples/Dockerfile.lambda](examples/Dockerfile.lambda)


