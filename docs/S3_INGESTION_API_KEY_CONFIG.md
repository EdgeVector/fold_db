# S3 Ingestion API Key Configuration

## Overview

The S3 ingestion API now supports multiple ways to configure the OpenRouter API key, providing flexibility for different use cases:

1. **Direct API Key** - Pass the API key directly in the request (recommended)
2. **Full Configuration** - Pass a complete `IngestionConfig` object
3. **Environment Variables** - Legacy approach using environment variables

## Method 1: Direct API Key (Recommended)

This is the simplest and most flexible approach, especially for Lambda functions and programmatic usage.

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};

let request = S3IngestionRequest::new("s3://bucket/file.json".to_string())
    .with_auto_execute(true)
    .with_openrouter_api_key("your-api-key-here".to_string());

let response = ingest_from_s3_path_async(
    &request, 
    &upload_storage, 
    &progress_tracker, 
    node, 
    None  // No config needed - using key from request
).await?;
```

### Benefits
- ✅ No environment variables required
- ✅ Perfect for Lambda functions
- ✅ Easy to manage multiple keys
- ✅ Clear and explicit

## Method 2: Custom OpenRouter Configuration

For more control over model selection and base URL:

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};

let request = S3IngestionRequest::new("s3://bucket/file.json".to_string())
    .with_openrouter_config(
        "your-api-key".to_string(),
        "anthropic/claude-3.5-sonnet".to_string(),
        "https://openrouter.ai/api/v1".to_string()
    );

let response = ingest_from_s3_path_async(&request, &upload_storage, &progress_tracker, node, None).await?;
```

## Method 3: Full Configuration Object

For maximum control over all ingestion settings:

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest, IngestionConfig};

// Build custom configuration
let mut config = IngestionConfig::default();
config.openrouter.api_key = "your-api-key".to_string();
config.openrouter.model = "anthropic/claude-3.5-sonnet".to_string();
config.enabled = true;
config.max_retries = 5;
config.timeout_seconds = 120;

// Pass in request
let request = S3IngestionRequest::new("s3://bucket/file.json".to_string())
    .with_ingestion_config(config);

let response = ingest_from_s3_path_async(&request, &upload_storage, &progress_tracker, node, None).await?;
```

## Method 4: Environment Variables (Legacy)

The original approach still works for backwards compatibility:

```bash
export FOLD_OPENROUTER_API_KEY=your-api-key
export OPENROUTER_MODEL=anthropic/claude-3.5-sonnet
```

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest, IngestionConfig};

// Config loaded from environment variables
let ingestion_config = IngestionConfig::from_env()?;

let request = S3IngestionRequest::new("s3://bucket/file.json".to_string());

let response = ingest_from_s3_path_async(
    &request, 
    &upload_storage, 
    &progress_tracker, 
    node, 
    Some(&ingestion_config)  // Pass environment-based config
).await?;
```

## Priority Order

When multiple configuration sources are provided, the priority is:

1. **`ingestion_config` parameter** - Explicitly passed to function
2. **`request.ingestion_config`** - Set via `with_openrouter_api_key()` or `with_ingestion_config()`
3. **Environment variables** - Falls back to `IngestionConfig::from_env()`

## Lambda Function Example

Perfect for AWS Lambda triggered by S3 events:

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    // Extract S3 path from event
    let bucket = event.payload["Records"][0]["s3"]["bucket"]["name"].as_str().unwrap();
    let key = event.payload["Records"][0]["s3"]["object"]["key"].as_str().unwrap();
    let s3_path = format!("s3://{}/{}", bucket, key);

    // Get API key from Lambda environment or secrets manager
    let api_key = std::env::var("FOLD_OPENROUTER_API_KEY")?;

    // Create request with API key
    let request = S3IngestionRequest::new(s3_path)
        .with_auto_execute(true)
        .with_openrouter_api_key(api_key);

    // Ingest without needing any other config
    let response = ingest_from_s3_path_async(
        &request, 
        &upload_storage, 
        &progress_tracker, 
        node, 
        None
    ).await?;

    Ok(json!({
        "statusCode": 200,
        "body": format!("Ingestion started: {}", response.progress_id.unwrap())
    }))
}
```

## Batch Processing Example

Processing multiple files with different configurations:

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};

let files = vec![
    ("s3://bucket/file1.json", "api-key-1"),
    ("s3://bucket/file2.json", "api-key-2"),
    ("s3://bucket/file3.json", "api-key-3"),
];

for (s3_path, api_key) in files {
    let request = S3IngestionRequest::new(s3_path.to_string())
        .with_openrouter_api_key(api_key.to_string());
    
    let response = ingest_from_s3_path_async(
        &request, 
        &upload_storage, 
        &progress_tracker, 
        node.clone(), 
        None
    ).await?;
    
    println!("Started {}: {}", s3_path, response.progress_id.unwrap());
}
```

## Migration Guide

### From Old API

**Before:**
```rust
let ingestion_config = IngestionConfig::from_env()?;
let request = S3IngestionRequest::new("s3://bucket/file.json".to_string());
let response = ingest_from_s3_path_async(&request, &storage, &tracker, node, &ingestion_config).await?;
```

**After (Recommended):**
```rust
let request = S3IngestionRequest::new("s3://bucket/file.json".to_string())
    .with_openrouter_api_key("your-api-key".to_string());
let response = ingest_from_s3_path_async(&request, &storage, &tracker, node, None).await?;
```

**After (Backwards Compatible):**
```rust
let ingestion_config = IngestionConfig::from_env()?;
let request = S3IngestionRequest::new("s3://bucket/file.json".to_string());
let response = ingest_from_s3_path_async(&request, &storage, &tracker, node, Some(&ingestion_config)).await?;
```

## Best Practices

1. **For Lambda Functions**: Use `with_openrouter_api_key()` with secrets from AWS Secrets Manager or environment variables
2. **For Local Development**: Use environment variables or direct API key
3. **For Multi-tenant Systems**: Pass different API keys per request using `with_openrouter_api_key()`
4. **For Complex Configurations**: Use `with_ingestion_config()` to control retries, timeouts, etc.

## Security Notes

⚠️ **Never hardcode API keys in source code**

✅ **Good:**
```rust
let api_key = std::env::var("OPENROUTER_API_KEY")?;
let request = S3IngestionRequest::new(s3_path)
    .with_openrouter_api_key(api_key);
```

❌ **Bad:**
```rust
let request = S3IngestionRequest::new(s3_path)
    .with_openrouter_api_key("sk-1234567890abcdef".to_string());  // DON'T DO THIS
```

## See Also

- [S3 File Path Ingestion](S3_FILE_PATH_INGESTION.md)
- [S3 Ingestion Quick Start](S3_INGESTION_QUICK_START.md)
- [S3 Ingestion API Guide](S3_INGESTION_API_GUIDE.md)
- [Lambda Example](../examples/lambda_s3_ingestion.rs)

