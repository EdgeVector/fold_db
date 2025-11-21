# AWS Lambda Integration Guide

This guide shows how to use DataFold in AWS Lambda functions for serverless operations.

## Quick Start

### 1. Add Dependencies

Add to your Lambda project's `Cargo.toml`:

```toml
[dependencies]
datafold = { version = "0.1", features = ["lambda"] }
lambda_runtime = "0.13"
tokio = { version = "1", features = ["macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### 2. Create Lambda Handler

```rust
use datafold::lambda::{LambdaConfig, LambdaContext};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Create configuration
    let config = LambdaConfig::new();
    
    // Initialize once during cold start
    LambdaContext::init(config).await?;
    
    // Run Lambda handler
    run(service_fn(handler)).await
}

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    // Access the DataFold node
    let node = LambdaContext::node()?;
    
    // Your logic here...
    
    Ok(json!({
        "statusCode": 200,
        "body": { "message": "Success" }
    }))
}
```

### 3. Optional Environment Variables

```bash
# Optional
SCHEMA_SERVICE_URL=https://your-schema-service.com
```

### 4. Build for Lambda

For x86_64 (Intel/AMD):
```bash
cargo build --release --target x86_64-unknown-linux-gnu --features lambda
```

For ARM64 (Graviton2/3):
```bash
cargo build --release --target aarch64-unknown-linux-gnu --features lambda
```

### 5. Package and Deploy

```bash
# Copy binary
cp target/x86_64-unknown-linux-gnu/release/your_lambda bootstrap

# Create deployment package
zip lambda.zip bootstrap

# Deploy with AWS CLI
aws lambda create-function \
  --function-name datafold-lambda \
  --runtime provided.al2 \
  --role arn:aws:iam::ACCOUNT:role/lambda-role \
  --handler bootstrap \
  --zip-file fileb://lambda.zip \
  --timeout 300 \
  --memory-size 512
```

## Lambda Context API

### Configuration

Create a `LambdaConfig` with optional settings:

```rust
use datafold::lambda::LambdaConfig;
use std::path::PathBuf;

// Basic configuration
let config = LambdaConfig::new();

// With schema service
let config = LambdaConfig::new()
    .with_schema_service_url("https://schema.example.com".to_string());

// With custom storage path
let config = LambdaConfig::new()
    .with_storage_path(PathBuf::from("/tmp/custom"));
```

### Initialization

The `LambdaContext` should be initialized once during the cold start:

```rust
use datafold::lambda::{LambdaConfig, LambdaContext};

// Create configuration
let config = LambdaConfig::new();

// Initialize context
LambdaContext::init(config).await?;
```

### Accessing the Node

```rust
use datafold::lambda::LambdaContext;

async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    let node = LambdaContext::node()?;
    let node_guard = node.lock().await;
    
    // Use node for operations
    let node_id = node_guard.get_id();
    
    drop(node_guard);
    Ok(())
}
```

### Progress Tracking

```rust
use datafold::lambda::LambdaContext;

// Get progress tracker
let tracker = LambdaContext::progress_tracker()?;

// Check progress
if let Some(progress) = LambdaContext::get_progress(&progress_id)? {
    println!("Current step: {:?}", progress.current_step);
    println!("Completed: {}", progress.completed);
}
```

## Complete Example

```rust
use datafold::lambda::{LambdaConfig, LambdaContext};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde_json::{json, Value};

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    tracing::info!("Processing event");
    
    // Access node
    let node = LambdaContext::node()?;
    let node_guard = node.lock().await;
    let node_id = node_guard.get_id();
    drop(node_guard);
    
    // Your Lambda logic here
    // For ingestion, schema operations, queries, etc.
    
    Ok(json!({
        "statusCode": 200,
        "body": {
            "message": "Processed successfully",
            "node_id": node_id
        }
    }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();
    
    // Create config (optionally from environment)
    let mut config = LambdaConfig::new();
    if let Ok(schema_url) = std::env::var("SCHEMA_SERVICE_URL") {
        config = config.with_schema_service_url(schema_url);
    }
    
    LambdaContext::init(config).await?;
    run(service_fn(handler)).await
}
```

## IAM Permissions

Your Lambda execution role needs these permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "logs:CreateLogGroup",
        "logs:CreateLogStream",
        "logs:PutLogEvents"
      ],
      "Resource": "arn:aws:logs:*:*:*"
    }
  ]
}
```

Add additional permissions based on your use case (S3, DynamoDB, etc.).

## Performance Optimization

### Memory Configuration

- **Basic operations**: 256 MB minimum
- **Data processing**: 512 MB recommended
- **Heavy workloads**: 1024 MB+

### Timeout Configuration

- **Quick operations**: 30 seconds
- **Data processing**: 1-5 minutes
- **Batch operations**: 5-15 minutes (Lambda max)

### Cold Start Optimization

The `LambdaContext` is designed to minimize cold start time:

1. Initialization happens once per container
2. Resources are reused across invocations
3. Database files use /tmp (Lambda's writable directory)
4. Minimal dependencies for fast compilation

### Provisioned Concurrency

For consistent performance, consider using provisioned concurrency:

```bash
aws lambda put-provisioned-concurrency-config \
  --function-name datafold-lambda \
  --provisioned-concurrent-executions 5
```

## Troubleshooting

### "Context not initialized" Error

Ensure `LambdaContext::init()` is called with a valid configuration before any operations:

```rust
#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = LambdaConfig::new();
    LambdaContext::init(config).await?;  // Must be called first
    run(service_fn(handler)).await
}
```

### Storage Path Issues

Lambda only allows writes to `/tmp`:

```rust
// This is the default, but you can customize if needed
let config = LambdaConfig::new()
    .with_storage_path(PathBuf::from("/tmp/folddb"));
```

### Memory Issues

If you see out-of-memory errors, increase Lambda memory:

```bash
aws lambda update-function-configuration \
  --function-name datafold-lambda \
  --memory-size 1024
```

## Example Project Structure

```
my-lambda/
├── Cargo.toml
├── src/
│   └── main.rs          # Lambda handler code
├── Makefile             # Build commands
└── deploy.sh            # Deployment script
```

### Makefile Example

```makefile
.PHONY: build package deploy

build:
	cargo build --release --target x86_64-unknown-linux-gnu --features lambda

package: build
	cp target/x86_64-unknown-linux-gnu/release/my_lambda bootstrap
	zip lambda.zip bootstrap
	rm bootstrap

deploy: package
	aws lambda update-function-code \
	  --function-name datafold-lambda \
	  --zip-file fileb://lambda.zip
```

## Container-Based Deployment

For larger Lambda functions, use container images:

```dockerfile
FROM public.ecr.aws/lambda/provided:al2

# Copy your binary
COPY bootstrap ${LAMBDA_RUNTIME_DIR}/bootstrap

CMD ["bootstrap"]
```

Build and deploy:

```bash
docker build -t datafold-lambda .
docker tag datafold-lambda:latest $ECR_REPO:latest
docker push $ECR_REPO:latest

aws lambda update-function-code \
  --function-name datafold-lambda \
  --image-uri $ECR_REPO:latest
```

## See Also

- [Example: lambda_s3_ingestion.rs](../examples/lambda_s3_ingestion.rs)
- [DataFold Node Documentation](../README.md)
