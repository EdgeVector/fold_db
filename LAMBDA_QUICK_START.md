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

## Complete Example

See `examples/lambda_s3_ingestion.rs` for a complete working example.

## More Info

- Full guide: [docs/LAMBDA_INTEGRATION.md](docs/LAMBDA_INTEGRATION.md)
- Example with Dockerfile: [examples/Dockerfile.lambda](examples/Dockerfile.lambda)
- AI Query Examples: [docs/AI_QUERY_EXAMPLES.md](docs/AI_QUERY_EXAMPLES.md)


