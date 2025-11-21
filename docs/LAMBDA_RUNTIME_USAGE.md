# How to Use the Lambda Runtime

This guide shows you step-by-step how to use DataFold in AWS Lambda.

## Quick Start Example

Here's a complete, working Lambda function:

```rust
use datafold::lambda::{LambdaConfig, LambdaContext};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde_json::{json, Value};

// Your Lambda handler function
async fn function_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    tracing::info!("Received event: {:?}", event.payload);
    
    // Get the DataFold node from the context
    let node = LambdaContext::node()?;
    
    // Do something with the node
    let node_guard = node.lock().await;
    let node_id = node_guard.get_node_id();
    drop(node_guard);
    
    // Return response
    Ok(json!({
        "statusCode": 200,
        "body": {
            "message": "Success!",
            "node_id": node_id
        }
    }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Setup logging (for CloudWatch)
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time() // CloudWatch adds timestamps
        .init();
    
    // Initialize the Lambda context (once, reused across invocations)
    let config = LambdaConfig::new();
    LambdaContext::init(config).await?;
    
    tracing::info!("Lambda ready!");
    
    // Run the Lambda runtime
    run(service_fn(function_handler)).await
}
```

## Step-by-Step Setup

### 1. Create a New Project

```bash
cargo new my-lambda --bin
cd my-lambda
```

### 2. Update `Cargo.toml`

```toml
[package]
name = "my-lambda"
version = "0.1.0"
edition = "2021"

[dependencies]
datafold = { version = "0.1", features = ["lambda"] }
lambda_runtime = "0.13"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Optional: for AWS SDK operations
aws-sdk-s3 = "1.0"
aws-config = "1.0"
```

### 3. Write Your Handler (`src/main.rs`)

See the complete example above, or customize for your use case below.

### 4. Build for Lambda

#### For x86_64 (Intel/AMD):
```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

#### For ARM64 (Graviton - cheaper and faster!):
```bash
# Install target first
rustup target add aarch64-unknown-linux-gnu

# Build
cargo build --release --target aarch64-unknown-linux-gnu
```

### 5. Package for Deployment

```bash
# Copy the binary and rename to 'bootstrap'
cp target/x86_64-unknown-linux-gnu/release/my-lambda bootstrap

# Strip debug symbols (optional, saves ~1 MB)
strip bootstrap

# Create zip file
zip lambda.zip bootstrap

# Clean up
rm bootstrap
```

### 6. Deploy to Lambda

Using AWS CLI:

```bash
aws lambda create-function \
  --function-name my-datafold-lambda \
  --runtime provided.al2 \
  --role arn:aws:iam::YOUR_ACCOUNT:role/lambda-execution-role \
  --handler bootstrap \
  --zip-file fileb://lambda.zip \
  --timeout 30 \
  --memory-size 512 \
  --environment Variables="{SCHEMA_SERVICE_URL=https://your-schema-service.com}"
```

Or use AWS Console, SAM, or CDK (see examples below).

## Common Use Cases

### Use Case 1: Process Data from Event

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct MyEvent {
    data_id: String,
    operation: String,
}

async fn function_handler(event: LambdaEvent<MyEvent>) -> Result<Value, Error> {
    let data_id = event.payload.data_id;
    
    // Get node
    let node = LambdaContext::node()?;
    let mut node_guard = node.lock().await;
    
    // Query or mutate data
    // Example: Query by ID
    // let result = node_guard.query(...);
    
    drop(node_guard);
    
    Ok(json!({
        "statusCode": 200,
        "body": { "processed": data_id }
    }))
}
```

### Use Case 2: S3 Triggered Ingestion

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct S3Event {
    #[serde(rename = "Records")]
    records: Vec<S3Record>,
}

#[derive(Deserialize)]
struct S3Record {
    s3: S3Info,
}

#[derive(Deserialize)]
struct S3Info {
    bucket: BucketInfo,
    object: ObjectInfo,
}

#[derive(Deserialize)]
struct BucketInfo {
    name: String,
}

#[derive(Deserialize)]
struct ObjectInfo {
    key: String,
}

async fn function_handler(event: LambdaEvent<S3Event>) -> Result<Value, Error> {
    let record = &event.payload.records[0];
    let bucket = &record.s3.bucket.name;
    let key = &record.s3.object.key;
    
    tracing::info!("Processing s3://{}/{}", bucket, key);
    
    // Download from S3
    let aws_config = aws_config::load_from_env().await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);
    
    let object = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;
    
    let bytes = object.body.collect().await?.into_bytes();
    let json_data: Value = serde_json::from_slice(&bytes)?;
    
    // Get node and progress tracker
    let node = LambdaContext::node()?;
    let tracker = LambdaContext::progress_tracker()?;
    
    // Use datafold's ingestion APIs
    // (implement your ingestion logic here)
    
    Ok(json!({
        "statusCode": 200,
        "body": { "message": "Ingested successfully" }
    }))
}
```

### Use Case 3: API Gateway Endpoint

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ApiGatewayEvent {
    body: Option<String>,
    #[serde(rename = "httpMethod")]
    http_method: String,
    path: String,
}

#[derive(Serialize)]
struct ApiResponse {
    #[serde(rename = "statusCode")]
    status_code: u16,
    body: String,
}

async fn function_handler(event: LambdaEvent<ApiGatewayEvent>) -> Result<ApiResponse, Error> {
    match event.payload.http_method.as_str() {
        "GET" => handle_get(event.payload.path).await,
        "POST" => handle_post(event.payload.body).await,
        _ => Ok(ApiResponse {
            status_code: 405,
            body: "Method Not Allowed".to_string(),
        }),
    }
}

async fn handle_get(path: String) -> Result<ApiResponse, Error> {
    let node = LambdaContext::node()?;
    let node_guard = node.lock().await;
    
    // Query data based on path
    // let results = node_guard.query(...);
    
    Ok(ApiResponse {
        status_code: 200,
        body: json!({"path": path}).to_string(),
    })
}

async fn handle_post(body: Option<String>) -> Result<ApiResponse, Error> {
    let data: Value = serde_json::from_str(&body.unwrap_or_default())?;
    
    let node = LambdaContext::node()?;
    let node_guard = node.lock().await;
    
    // Insert data
    // node_guard.mutate(...);
    
    Ok(ApiResponse {
        status_code: 201,
        body: json!({"message": "Created"}).to_string(),
    })
}
```

### Use Case 4: Scheduled Processing (EventBridge)

```rust
async fn function_handler(_event: LambdaEvent<Value>) -> Result<Value, Error> {
    tracing::info!("Running scheduled task");
    
    let node = LambdaContext::node()?;
    let node_guard = node.lock().await;
    
    // Perform periodic tasks
    // - Cleanup old data
    // - Generate reports
    // - Sync data
    // - etc.
    
    Ok(json!({
        "statusCode": 200,
        "body": { "message": "Scheduled task complete" }
    }))
}
```

## Configuration Options

### Basic Configuration

```rust
let config = LambdaConfig::new();
LambdaContext::init(config).await?;
```

### With Schema Service

```rust
let config = LambdaConfig::new()
    .with_schema_service_url("https://schema.example.com".to_string());
LambdaContext::init(config).await?;
```

### With Custom Storage Path

```rust
use std::path::PathBuf;

let config = LambdaConfig::new()
    .with_storage_path(PathBuf::from("/tmp/custom-db"));
LambdaContext::init(config).await?;
```

### From Environment Variables

```rust
let mut config = LambdaConfig::new();

if let Ok(schema_url) = std::env::var("SCHEMA_SERVICE_URL") {
    config = config.with_schema_service_url(schema_url);
}

LambdaContext::init(config).await?;
```

## Deployment Options

### Option 1: AWS CLI (Simple)

```bash
# Create function
aws lambda create-function \
  --function-name my-function \
  --runtime provided.al2 \
  --role arn:aws:iam::ACCOUNT:role/lambda-role \
  --handler bootstrap \
  --zip-file fileb://lambda.zip

# Update function code
aws lambda update-function-code \
  --function-name my-function \
  --zip-file fileb://lambda.zip
```

### Option 2: AWS SAM (Recommended)

Create `template.yaml`:

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Transform: AWS::Serverless-2016-10-31

Resources:
  MyDataFoldFunction:
    Type: AWS::Serverless::Function
    Properties:
      CodeUri: ./
      Handler: bootstrap
      Runtime: provided.al2
      Timeout: 30
      MemorySize: 512
      Environment:
        Variables:
          SCHEMA_SERVICE_URL: https://schema.example.com
      Events:
        ApiEvent:
          Type: Api
          Properties:
            Path: /api
            Method: post
```

Deploy:

```bash
sam build
sam deploy --guided
```

### Option 3: Container Image (For Large Functions)

Create `Dockerfile`:

```dockerfile
FROM public.ecr.aws/lambda/provided:al2

# Copy your binary
COPY target/x86_64-unknown-linux-gnu/release/my-lambda ${LAMBDA_RUNTIME_DIR}/bootstrap

# Make it executable
RUN chmod +x ${LAMBDA_RUNTIME_DIR}/bootstrap

CMD ["bootstrap"]
```

Build and deploy:

```bash
docker build -t my-lambda .
docker tag my-lambda:latest $ECR_REPO:latest
docker push $ECR_REPO:latest

aws lambda create-function \
  --function-name my-function \
  --package-type Image \
  --code ImageUri=$ECR_REPO:latest \
  --role arn:aws:iam::ACCOUNT:role/lambda-role
```

## Makefile for Easy Building

Create `Makefile`:

```makefile
.PHONY: build package deploy clean

# Build for x86_64
build:
	cargo build --release --target x86_64-unknown-linux-gnu

# Build for ARM (Graviton)
build-arm:
	cargo build --release --target aarch64-unknown-linux-gnu

# Package for deployment
package: build
	cp target/x86_64-unknown-linux-gnu/release/my-lambda bootstrap
	strip bootstrap
	zip lambda.zip bootstrap
	rm bootstrap

# Deploy to Lambda
deploy: package
	aws lambda update-function-code \
		--function-name my-datafold-lambda \
		--zip-file fileb://lambda.zip

# Clean build artifacts
clean:
	cargo clean
	rm -f lambda.zip bootstrap
```

Usage:

```bash
make build     # Build the binary
make package   # Create lambda.zip
make deploy    # Deploy to AWS
```

## Testing Locally

You can test your Lambda locally using:

### AWS SAM Local

```bash
# Start local API
sam local start-api

# Invoke function
sam local invoke MyDataFoldFunction -e test-event.json
```

### Cargo Lambda

```bash
# Install cargo-lambda
cargo install cargo-lambda

# Run locally
cargo lambda watch

# Invoke
cargo lambda invoke --data-ascii '{"test": "data"}'
```

## Troubleshooting

### Issue: "Context not initialized"

Make sure you call `LambdaContext::init()` before running the handler:

```rust
#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = LambdaConfig::new();
    LambdaContext::init(config).await?;  // ← Must be here!
    run(service_fn(function_handler)).await
}
```

### Issue: Binary too large for direct upload

If your binary is >50 MB (unlikely):
1. Use S3 upload instead of direct zip
2. Use container images
3. Enable feature flags to reduce size

### Issue: Out of memory

Increase Lambda memory:

```bash
aws lambda update-function-configuration \
  --function-name my-function \
  --memory-size 1024
```

### Issue: Timeout

Increase Lambda timeout:

```bash
aws lambda update-function-configuration \
  --function-name my-function \
  --timeout 300
```

## Next Steps

1. Check out the [full example](../examples/lambda_s3_ingestion.rs)
2. Read the [Lambda Integration Guide](LAMBDA_INTEGRATION.md)
3. Review [Binary Size Analysis](LAMBDA_BINARY_SIZE_ANALYSIS.md)

## Summary

The lambda runtime usage is straightforward:

1. **Add dependencies** - `datafold` with `lambda` feature + `lambda_runtime`
2. **Initialize context** - `LambdaContext::init(config).await?` in main
3. **Access node** - `LambdaContext::node()?` in handler
4. **Build** - `cargo build --release --target x86_64-unknown-linux-gnu`
5. **Package** - Rename to `bootstrap`, create zip
6. **Deploy** - Use AWS CLI, SAM, or CDK

That's it! The lambda module handles all the complexity of initialization and context management.

