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

## AI Query Methods (Stateless)

DataFold Lambda API includes powerful AI query capabilities using natural language.
All AI query methods are **fully stateless** - no server-side session management required.

### Configuration

Enable AI query functionality by providing AI configuration during initialization:

#### With OpenRouter

```rust
use datafold::lambda::LambdaConfig;

let config = LambdaConfig::new()
    .with_openrouter(
        "sk-or-v1-your-api-key".to_string(),
        "anthropic/claude-3.5-sonnet".to_string()
    );

LambdaContext::init(config).await?;
```

#### With Ollama

```rust
let config = LambdaConfig::new()
    .with_ollama(
        "http://localhost:11434".to_string(),
        "llama2".to_string()
    );

LambdaContext::init(config).await?;
```

### Simple AI Query

The simplest way to query your data using natural language:

```rust
use datafold::lambda::LambdaContext;

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let query = event.payload["query"]
        .as_str()
        .unwrap_or("Show me all products");
    
    // Execute AI query - returns interpreted results
    let response = LambdaContext::ai_query(query).await?;
    
    Ok(json!({
        "statusCode": 200,
        "body": {
            "interpretation": response.ai_interpretation,
            "results_count": response.raw_results.len(),
            // Optionally include context for follow-ups
            "context": response.context
        }
    }))
}
```

### Complete Query Workflow

For more detailed results with query planning and summaries:

```rust
async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let query = event.payload["query"].as_str().unwrap_or("");
    
    // Run complete workflow: analyze + execute + summarize
    let response = LambdaContext::run_ai_query(query).await?;
    
    Ok(json!({
        "statusCode": 200,
        "body": {
            "query_plan": {
                "schema": response.query_plan.schema_name,
                "reasoning": response.query_plan.reasoning,
            },
            "summary": response.summary,
            "results": response.results,
            "context": response.context  // For follow-ups
        }
    }))
}
```

### Follow-up Questions (Stateless)

Handle multi-turn conversations by passing context back from the client:

```rust
use datafold::lambda::{LambdaContext, FollowupRequest, QueryContext};

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    // Check if this is a follow-up or initial query
    if let Some(context_value) = event.payload.get("context") {
        // This is a follow-up
        let question = event.payload["question"]
            .as_str()
            .ok_or("Missing question")?;
        
        let context: QueryContext = serde_json::from_value(context_value.clone())?;
        
        let response = LambdaContext::ask_followup(FollowupRequest {
            context,
            question: question.to_string(),
        }).await?;
        
        Ok(json!({
            "statusCode": 200,
            "body": {
                "answer": response.answer,
                "executed_new_query": response.executed_new_query,
                "context": response.context  // Updated context
            }
        }))
    } else {
        // Initial query
        let query = event.payload["query"]
            .as_str()
            .ok_or("Missing query")?;
        
        let response = LambdaContext::run_ai_query(query).await?;
        
        Ok(json!({
            "statusCode": 200,
            "body": {
                "summary": response.summary,
                "results": response.results,
                "context": response.context
            }
        }))
    }
}
```

### Multi-turn Conversation Example

Client-side example showing how to maintain conversation context:

```rust
// First question
let payload1 = json!({
    "query": "Show me all electronics products"
});
let response1 = invoke_lambda(payload1).await?;
let context = response1["body"]["context"].clone();

// Second question - follow-up
let payload2 = json!({
    "context": context,
    "question": "Which ones are under $100?"
});
let response2 = invoke_lambda(payload2).await?;
let updated_context = response2["body"]["context"].clone();

// Third question - another follow-up
let payload3 = json!({
    "context": updated_context,
    "question": "Sort by price"
});
let response3 = invoke_lambda(payload3).await?;
```

### Advanced Configuration

```rust
use datafold::lambda::{LambdaConfig, AIConfig, AIProvider, OpenRouterConfig};

let ai_config = AIConfig {
    provider: AIProvider::OpenRouter,
    openrouter: Some(OpenRouterConfig {
        api_key: "sk-or-v1-...".to_string(),
        model: "anthropic/claude-3.5-sonnet".to_string(),
        base_url: None,  // Use default
    }),
    ollama: None,
    timeout_seconds: 180,  // 3 minutes
    max_retries: 5,
};

let config = LambdaConfig::new()
    .with_schema_service_url("https://schema.example.com".to_string())
    .with_ai_config(ai_config);

LambdaContext::init(config).await?;
```

### Reading from AWS Secrets Manager

```rust
use aws_sdk_secretsmanager::Client as SecretsClient;

async fn get_openrouter_key() -> Result<String, Error> {
    let config = aws_config::load_from_env().await;
    let client = SecretsClient::new(&config);
    
    let response = client
        .get_secret_value()
        .secret_id("datafold/openrouter-key")
        .send()
        .await?;
    
    Ok(response.secret_string().unwrap_or_default().to_string())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let api_key = get_openrouter_key().await?;
    
    let config = LambdaConfig::new()
        .with_openrouter(api_key, "anthropic/claude-3.5-sonnet".to_string());
    
    LambdaContext::init(config).await?;
    run(service_fn(handler)).await
}
```

### Important Notes

- **Stateless**: Each Lambda invocation is independent
- **Client Manages Context**: Client sends full context with each follow-up
- **No Session Storage**: No DynamoDB/Redis needed
- **Context Size**: Be mindful of payload size (6MB Lambda limit)
- **Error Handling**: AI methods return errors if not configured

### Example Queries

```rust
// Simple search
LambdaContext::ai_query("Find all electronics products").await?

// Complex query
LambdaContext::run_ai_query("Show blog posts about AI from last month").await?

// Follow-up
LambdaContext::ask_followup(FollowupRequest {
    context: previous_context,
    question: "Which have more than 100 views?".to_string(),
}).await?
```

## Multi-Tenant Logging

DataFold Lambda supports pluggable logging backends for multi-tenant deployments.

**All internal datafold logging is automatically captured** - when you configure a logger, all `log::info!()`, `log::error!()`, etc. calls throughout datafold are forwarded to your custom logger implementation.

### How It Works

1. You implement the `Logger` trait with your backend (DynamoDB, S3, etc.)
2. Pass your logger to `LambdaConfig::with_logger()`
3. DataFold automatically bridges all internal logging to your logger
4. Your logger implementation determines how to handle `user_id` (e.g., via task-local storage)

### Basic Logging (Stdout)

```rust
use datafold::lambda::{LambdaContext, LambdaConfig, StdoutLogger};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Use stdout logger for development/debugging
    let config = LambdaConfig::new()
        .with_logger(Arc::new(StdoutLogger));
    
    LambdaContext::init(config).await?;
    run(service_fn(handler)).await
}

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let user_id = event.payload["user_id"].as_str().unwrap_or("anonymous");
    
    // Create user-scoped logger
    let logger = LambdaContext::create_logger(user_id)?;
    
    logger.info("request_started", "Processing your request").await?;
    
    // Your business logic...
    let result = LambdaContext::ingest_json(
        event.payload["data"].clone(),
        true,
        0,
        user_id.to_string()
    ).await?;
    
    logger.info("ingestion_completed", &format!("Started: {}", result)).await?;
    
    Ok(json!({ "statusCode": 200, "progress_id": result }))
}
```

Output to CloudWatch:
```
[user_123] [INFO] request_started - Processing your request
[user_123] [INFO] ingestion_completed - Started: abc-123-def
```

### Custom Logger Implementation

Implement the `Logger` trait with your backend of choice (DynamoDB, S3, custom database, etc.):

```rust
use datafold::lambda::{Logger, LogEntry};
use async_trait::async_trait;

pub struct MyCustomLogger {
    // Your backend (DynamoDB, S3, PostgreSQL, etc.)
}

#[async_trait]
impl Logger for MyCustomLogger {
    async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Write to your backend
        println!("Logging for user {}: {}", entry.user_id, entry.message);
        Ok(())
    }
    
    // Optional: implement querying
    async fn query(
        &self,
        user_id: &str,
        limit: Option<usize>,
        from_timestamp: Option<i64>,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        // Query from your backend
        Ok(vec![])
    }
}
```

### DynamoDB Logger Example

See `examples/lambda_dynamodb_logger.rs` for a complete DynamoDB implementation.

**In your Lambda project:**

```rust
// src/dynamodb_logger.rs
use datafold::lambda::{Logger, LogEntry, LogLevel};
use async_trait::async_trait;
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use std::collections::HashMap;
use tokio::task_local;

// Task-local storage for current user
task_local! {
    pub static CURRENT_USER: String;
}

pub struct DynamoDbLogger {
    client: Client,
    table_name: String,
}

impl DynamoDbLogger {
    pub async fn new(table_name: String) -> Self {
        let config = aws_config::load_from_env().await;
        let client = Client::new(&config);
        Self { client, table_name }
    }
}

#[async_trait]
impl Logger for DynamoDbLogger {
    async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ttl = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() + (30 * 24 * 60 * 60)) as i64; // 30 days
        
        // Get user_id from entry or task-local storage
        let user_id = entry.user_id
            .or_else(|| CURRENT_USER.try_with(|id| id.clone()).ok())
            .unwrap_or_else(|| "system".to_string());
        
        let mut item = HashMap::new();
        item.insert("user_id".to_string(), AttributeValue::S(user_id));
        item.insert("timestamp".to_string(), AttributeValue::N(entry.timestamp.to_string()));
        item.insert("level".to_string(), AttributeValue::S(entry.level.as_str().to_string()));
        item.insert("event_type".to_string(), AttributeValue::S(entry.event_type));
        item.insert("message".to_string(), AttributeValue::S(entry.message));
        item.insert("ttl".to_string(), AttributeValue::N(ttl.to_string()));
        
        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await?;
        
        Ok(())
    }
    
    async fn query(
        &self,
        user_id: &str,
        limit: Option<usize>,
        from_timestamp: Option<i64>,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        // Query implementation...
        Ok(vec![])
    }
}
```

**Usage:**

```rust
// src/main.rs
use datafold::lambda::{LambdaContext, LambdaConfig};
use std::sync::Arc;

mod dynamodb_logger;
use dynamodb_logger::{DynamoDbLogger, CURRENT_USER};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Create DynamoDB logger
    let logger = DynamoDbLogger::new("datafold-logs".to_string()).await;
    
    // Initialize datafold with custom logger
    let config = LambdaConfig::new()
        .with_logger(Arc::new(logger));
    
    LambdaContext::init(config).await?;
    run(service_fn(handler)).await
}

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let user_id = event.payload["user_id"].as_str().unwrap_or("anonymous");
    
    // Set user context for this request
    CURRENT_USER.scope(user_id.to_string(), async {
        // All logging (including internal datafold logs) within this scope
        // will automatically have user_id set to "user_123"
        
        let result = LambdaContext::ingest_json(
            event.payload["data"].clone(),
            true,
            0,
            user_id.to_string()
        ).await?;
        
        // Internal datafold logs during ingestion will also have user_id
        
        Ok(json!({ "statusCode": 200, "progress_id": result }))
    }).await
}
```

### DynamoDB Table Setup

```bash
# Create table
aws dynamodb create-table \
  --table-name datafold-logs \
  --attribute-definitions \
    AttributeName=user_id,AttributeType=S \
    AttributeName=timestamp,AttributeType=N \
  --key-schema \
    AttributeName=user_id,KeyType=HASH \
    AttributeName=timestamp,KeyType=RANGE \
  --billing-mode PAY_PER_REQUEST

# Enable TTL for automatic cleanup
aws dynamodb update-time-to-live \
  --table-name datafold-logs \
  --time-to-live-specification "Enabled=true, AttributeName=ttl"
```

### IAM Permissions

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "dynamodb:PutItem",
        "dynamodb:Query"
      ],
      "Resource": "arn:aws:dynamodb:*:*:table/datafold-logs"
    }
  ]
}
```

### Querying Logs

```rust
// Query user's logs
let logs = LambdaContext::query_logs(
    "user_123",
    Some(100),  // limit
    None        // from_timestamp
).await?;

for log in logs {
    println!("{}: {} - {}", log.timestamp, log.event_type, log.message);
}
```

### Logger Methods

```rust
let logger = LambdaContext::create_logger("user_123")?;

// Simple logging
logger.info("event_type", "message").await?;
logger.error("event_type", "message").await?;
logger.warn("event_type", "message").await?;
logger.debug("event_type", "message").await?;
logger.trace("event_type", "message").await?;

// Logging with metadata
use std::collections::HashMap;
use datafold::lambda::LogLevel;

logger.log(
    LogLevel::Info,
    "ingestion_completed",
    "Successfully ingested data",
    Some(HashMap::from([
        ("record_count".to_string(), "1000".to_string()),
        ("schema".to_string(), "users".to_string()),
    ]))
).await?;
```

### Cost Considerations

**DynamoDB (recommended for multi-tenant):**
- Writes: $1.25 per million requests
- Reads: $0.25 per million requests
- Storage: $0.25/GB/month
- TTL deletions: FREE

**CloudWatch Logs:**
- Storage: $0.50/GB/month
- Ingestion: $0.50/GB
- GetLogEvents: FREE
- Insights queries: $0.005/GB scanned (expensive at scale)

**S3 + Athena:**
- Storage: $0.023/GB/month (cheapest)
- Athena queries: $5/TB scanned (with partitioning)
- 5-15 minute query delay

## Complete Example

See `examples/lambda_s3_ingestion.rs` for a complete working example.

## More Info

- Full guide: [docs/LAMBDA_INTEGRATION.md](docs/LAMBDA_INTEGRATION.md)
- Example with Dockerfile: [examples/Dockerfile.lambda](examples/Dockerfile.lambda)
- AI Query Examples: [docs/AI_QUERY_EXAMPLES.md](docs/AI_QUERY_EXAMPLES.md)


