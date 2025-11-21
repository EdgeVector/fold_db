# DataFold S3 Ingestion API Guide

A comprehensive guide to using DataFold's programmatic S3 ingestion methods for processing files stored in Amazon S3.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Installation & Setup](#installation--setup)
- [API Reference](#api-reference)
- [Usage Examples](#usage-examples)
- [Configuration](#configuration)
- [Common Patterns](#common-patterns)
- [Error Handling](#error-handling)
- [Performance & Best Practices](#performance--best-practices)
- [Troubleshooting](#troubleshooting)

---

## Overview

DataFold's S3 ingestion API allows you to process files directly from Amazon S3 without re-uploading them. This is ideal for:

- **AWS Lambda functions** triggered by S3 events
- **Batch processing** of existing S3 files
- **Data pipelines** that produce S3 outputs
- **Serverless architectures** where files are already in S3

### Benefits

✅ **No Re-upload** - Process files directly from S3  
✅ **Bandwidth Savings** - Avoid unnecessary data transfers  
✅ **Lambda-Ready** - Perfect for serverless deployments  
✅ **Two Processing Modes** - Async (background) or sync (wait for completion)  

---

## Quick Start

### Basic Async Example (Returns Immediately)

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest, IngestionConfig};
use datafold::storage::UploadStorage;
use datafold::datafold_node::DataFoldNode;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup (typically done once at startup)
    let upload_storage = UploadStorage::from_env()?;
    let progress_tracker = datafold::ingestion::create_progress_tracker();
    let node = Arc::new(Mutex::new(DataFoldNode::new("data")?));
    let ingestion_config = IngestionConfig::from_env()?;
    
    // Create request
    let request = S3IngestionRequest::new("s3://my-bucket/data.json".to_string())
        .with_auto_execute(true);
    
    // Start ingestion (returns immediately)
    let response = ingest_from_s3_path_async(
        &request,
        &upload_storage,
        &progress_tracker,
        node.clone(),
        &ingestion_config,
    ).await?;
    
    println!("Started ingestion: {}", response.progress_id.unwrap());
    
    Ok(())
}
```

### Basic Sync Example (Waits for Completion)

```rust
use datafold::ingestion::{ingest_from_s3_path_sync, S3IngestionRequest, IngestionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup (same as async example)
    let upload_storage = UploadStorage::from_env()?;
    let progress_tracker = datafold::ingestion::create_progress_tracker();
    let node = Arc::new(Mutex::new(DataFoldNode::new("data")?));
    let ingestion_config = IngestionConfig::from_env()?;
    
    // Create request
    let request = S3IngestionRequest::new("s3://my-bucket/data.json".to_string())
        .with_auto_execute(true)
        .with_trust_distance(0);
    
    // Wait for ingestion to complete
    let response = ingest_from_s3_path_sync(
        &request,
        &upload_storage,
        &progress_tracker,
        node.clone(),
        &ingestion_config,
    ).await?;
    
    println!("Ingestion complete!");
    println!("Schema used: {:?}", response.schema_used);
    println!("Mutations executed: {}", response.mutations_executed);
    
    Ok(())
}
```

---

## Installation & Setup

### 1. Add Dependency

Add DataFold to your `Cargo.toml`:

```toml
[dependencies]
datafold = "0.1.4"
tokio = { version = "1", features = ["full"] }
```

### 2. Set Environment Variables

```bash
# Required: S3 storage configuration for DataFold
export DATAFOLD_UPLOAD_STORAGE_MODE=s3
export DATAFOLD_UPLOAD_S3_BUCKET=my-datafold-bucket
export DATAFOLD_UPLOAD_S3_REGION=us-west-2

# Required: AI service for schema inference (pick one)
export OPENROUTER_API_KEY=your-openrouter-key
# OR
export OLLAMA_URL=http://localhost:11434

# Optional: AWS credentials (if not using IAM roles)
export AWS_ACCESS_KEY_ID=your-access-key
export AWS_SECRET_ACCESS_KEY=your-secret-key
```

### 3. AWS IAM Permissions

Your AWS credentials need these S3 permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:GetObject",
        "s3:HeadObject"
      ],
      "Resource": "arn:aws:s3:::*/*"
    }
  ]
}
```

**Note:** The permissions need to cover **any S3 bucket** you want to read from, not just your configured DataFold bucket.

---

## API Reference

### S3IngestionRequest

The request object for S3 ingestion operations.

#### Constructor

```rust
pub fn new(s3_path: String) -> Self
```

Creates a new request with default settings:
- `auto_execute`: `true`
- `trust_distance`: `0`
- `pub_key`: `"default"`

#### Builder Methods

```rust
pub fn with_auto_execute(self, auto_execute: bool) -> Self
```
Whether to automatically execute generated mutations. Default: `true`.

```rust
pub fn with_trust_distance(self, trust_distance: u32) -> Self
```
Trust distance for schema matching. Lower values require closer matches. Default: `0`.

```rust
pub fn with_pub_key(self, pub_key: String) -> Self
```
Public key for authentication/authorization. Default: `"default"`.

#### Example

```rust
// Basic request
let request = S3IngestionRequest::new("s3://bucket/file.json".to_string())
    .with_auto_execute(false)
    .with_trust_distance(5)
    .with_pub_key("user-123".to_string());

// With API key passed directly (recommended)
let request = S3IngestionRequest::new("s3://bucket/file.json".to_string())
    .with_auto_execute(true)
    .with_openrouter_api_key("your-api-key".to_string());

// With full configuration
use datafold::ingestion::IngestionConfig;
let mut config = IngestionConfig::default();
config.openrouter.api_key = "your-api-key".to_string();
config.openrouter.model = "anthropic/claude-3.5-sonnet".to_string();
config.enabled = true;

let request = S3IngestionRequest::new("s3://bucket/file.json".to_string())
    .with_ingestion_config(config);
```

### ingest_from_s3_path_async

Asynchronous ingestion that returns immediately with a progress ID.

#### Signature

```rust
pub async fn ingest_from_s3_path_async(
    request: &S3IngestionRequest,
    upload_storage: &UploadStorage,
    progress_tracker: &ProgressTracker,
    node: Arc<Mutex<DataFoldNode>>,
    ingestion_config: Option<&IngestionConfig>,
) -> Result<IngestionResponse, IngestionError>
```

#### Parameters

- `request` - S3 ingestion request configuration (can include API key via `with_openrouter_api_key()`)
- `upload_storage` - Storage handler for file management
- `progress_tracker` - Shared progress tracker for monitoring
- `node` - DataFold database node (wrapped in Arc<Mutex>)
- `ingestion_config` - Optional ingestion configuration. If `None`, uses config from request or environment variables

#### Returns

Returns `IngestionResponse` with:
- `success`: `true` if started successfully
- `progress_id`: UUID string for tracking progress
- Other fields will be `None`/`0` (use progress ID to query status)

#### Use When

- ✅ Lambda functions (return immediately, process in background)
- ✅ Batch processing (don't wait for each file)
- ✅ Long-running ingestions
- ✅ When you need to return a response quickly

### ingest_from_s3_path_sync

Synchronous ingestion that waits for completion.

#### Signature

```rust
pub async fn ingest_from_s3_path_sync(
    request: &S3IngestionRequest,
    upload_storage: &UploadStorage,
    progress_tracker: &ProgressTracker,
    node: Arc<Mutex<DataFoldNode>>,
    ingestion_config: Option<&IngestionConfig>,
) -> Result<IngestionResponse, IngestionError>
```

#### Parameters

Same as `ingest_from_s3_path_async`. The `ingestion_config` parameter is optional.

#### Returns

Returns complete `IngestionResponse` with:
- `success`: Whether ingestion succeeded
- `schema_used`: Schema name that was used
- `mutations_executed`: Number of mutations executed
- `errors`: Any errors encountered
- `progress_id`: None (operation is complete)

#### Use When

- ✅ Need immediate results
- ✅ Simple scripts
- ✅ Sequential processing
- ✅ When errors need immediate handling

---

## Usage Examples

### Example 1: AWS Lambda S3 Event Handler

Process files automatically when uploaded to S3:

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest, IngestionConfig};
use datafold::storage::UploadStorage;
use datafold::datafold_node::DataFoldNode;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

// Global state (initialized once per Lambda container)
lazy_static::lazy_static! {
    static ref UPLOAD_STORAGE: UploadStorage = 
        UploadStorage::from_env().expect("Failed to init storage");
    static ref PROGRESS_TRACKER: datafold::ingestion::ProgressTracker = 
        datafold::ingestion::create_progress_tracker();
    static ref NODE: Arc<Mutex<DataFoldNode>> = 
        Arc::new(Mutex::new(DataFoldNode::new("data").expect("Failed to init node")));
    static ref INGESTION_CONFIG: IngestionConfig = 
        IngestionConfig::from_env().expect("Failed to init config");
}

async fn function_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    // Parse S3 event
    let bucket = event.payload["Records"][0]["s3"]["bucket"]["name"]
        .as_str()
        .ok_or("Missing bucket name")?;
    let key = event.payload["Records"][0]["s3"]["object"]["key"]
        .as_str()
        .ok_or("Missing object key")?;
    
    let s3_path = format!("s3://{}/{}", bucket, key);
    println!("Processing: {}", s3_path);
    
    // Start ingestion
    let request = S3IngestionRequest::new(s3_path)
        .with_auto_execute(true)
        .with_trust_distance(0);
    
    let response = ingest_from_s3_path_async(
        &request,
        &UPLOAD_STORAGE,
        &PROGRESS_TRACKER,
        NODE.clone(),
        &INGESTION_CONFIG,
    ).await?;
    
    Ok(json!({
        "statusCode": 200,
        "body": {
            "message": "Ingestion started",
            "progress_id": response.progress_id,
        }
    }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}
```

### Example 2: Batch Processing Multiple Files

Process all JSON files in an S3 prefix:

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest, IngestionConfig};
use aws_sdk_s3::Client;

async fn batch_process_s3_folder(
    bucket: &str,
    prefix: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Setup DataFold
    let upload_storage = UploadStorage::from_env()?;
    let progress_tracker = datafold::ingestion::create_progress_tracker();
    let node = Arc::new(Mutex::new(DataFoldNode::new("data")?));
    let ingestion_config = IngestionConfig::from_env()?;
    
    // List S3 objects
    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);
    let objects = client
        .list_objects_v2()
        .bucket(bucket)
        .prefix(prefix)
        .send()
        .await?;
    
    let mut progress_ids = Vec::new();
    
    // Process each file
    for object in objects.contents().unwrap_or_default() {
        if let Some(key) = object.key() {
            if key.ends_with(".json") {
                let s3_path = format!("s3://{}/{}", bucket, key);
                println!("Processing: {}", s3_path);
                
                let request = S3IngestionRequest::new(s3_path);
                
                let response = ingest_from_s3_path_async(
                    &request,
                    &upload_storage,
                    &progress_tracker,
                    node.clone(),
                    &ingestion_config,
                ).await?;
                
                if let Some(progress_id) = response.progress_id {
                    progress_ids.push(progress_id);
                }
            }
        }
    }
    
    println!("Started {} ingestion jobs", progress_ids.len());
    Ok(progress_ids)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let progress_ids = batch_process_s3_folder("my-bucket", "data/2024/").await?;
    println!("Progress IDs: {:?}", progress_ids);
    Ok(())
}
```

### Example 3: Sequential Processing with Results

Process files one at a time and get immediate results:

```rust
use datafold::ingestion::{ingest_from_s3_path_sync, S3IngestionRequest, IngestionConfig};

async fn process_files_sequentially(
    files: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let upload_storage = UploadStorage::from_env()?;
    let progress_tracker = datafold::ingestion::create_progress_tracker();
    let node = Arc::new(Mutex::new(DataFoldNode::new("data")?));
    let ingestion_config = IngestionConfig::from_env()?;
    
    for s3_path in files {
        println!("Processing: {}", s3_path);
        
        let request = S3IngestionRequest::new(s3_path.clone())
            .with_auto_execute(true);
        
        // Wait for each file to complete
        let response = ingest_from_s3_path_sync(
            &request,
            &upload_storage,
            &progress_tracker,
            node.clone(),
            &ingestion_config,
        ).await?;
        
        if response.success {
            println!("✓ {}: {} mutations", s3_path, response.mutations_executed);
        } else {
            println!("✗ {}: {:?}", s3_path, response.errors);
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let files = vec![
        "s3://bucket/file1.json".to_string(),
        "s3://bucket/file2.json".to_string(),
        "s3://bucket/file3.json".to_string(),
    ];
    
    process_files_sequentially(files).await?;
    Ok(())
}
```

### Example 4: Custom Error Handling

Handle different error types appropriately:

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest, IngestionError};

async fn process_with_error_handling(
    s3_path: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Setup (same as previous examples)
    let upload_storage = UploadStorage::from_env()?;
    let progress_tracker = datafold::ingestion::create_progress_tracker();
    let node = Arc::new(Mutex::new(DataFoldNode::new("data")?));
    let ingestion_config = IngestionConfig::from_env()?;
    
    let request = S3IngestionRequest::new(s3_path.clone());
    
    match ingest_from_s3_path_async(
        &request,
        &upload_storage,
        &progress_tracker,
        node,
        &ingestion_config,
    ).await {
        Ok(response) => {
            println!("Success: {:?}", response.progress_id);
        }
        Err(IngestionError::InvalidS3Path(path)) => {
            eprintln!("Invalid S3 path format: {}", path);
        }
        Err(IngestionError::S3DownloadFailed(err)) => {
            eprintln!("Failed to download from S3: {}", err);
        }
        Err(IngestionError::FileConversionFailed) => {
            eprintln!("Failed to convert file to JSON");
        }
        Err(err) => {
            eprintln!("Other error: {:?}", err);
        }
    }
    
    Ok(())
}
```

### Example 5: Monitoring Progress (Async Mode)

Track progress of async ingestions:

```rust
use datafold::ingestion::{ingest_from_s3_path_async, ProgressTracker, IngestionStep};
use tokio::time::{sleep, Duration};

async fn process_and_monitor(
    s3_path: String,
    progress_tracker: &ProgressTracker,
) -> Result<(), Box<dyn std::error::Error>> {
    // Setup and start ingestion
    let upload_storage = UploadStorage::from_env()?;
    let node = Arc::new(Mutex::new(DataFoldNode::new("data")?));
    let ingestion_config = IngestionConfig::from_env()?;
    
    let request = S3IngestionRequest::new(s3_path);
    let response = ingest_from_s3_path_async(
        &request,
        &upload_storage,
        progress_tracker,
        node,
        &ingestion_config,
    ).await?;
    
    let progress_id = response.progress_id.unwrap();
    println!("Started: {}", progress_id);
    
    // Poll progress
    loop {
        let tracker = progress_tracker.lock().unwrap();
        if let Some(progress) = tracker.get(&progress_id) {
            match &progress.current_step {
                IngestionStep::Complete => {
                    println!("✓ Complete!");
                    if let Some(results) = &progress.results {
                        println!("Schema: {:?}", results.schema_used);
                        println!("Mutations: {}", results.mutations_executed);
                    }
                    break;
                }
                IngestionStep::Failed(err) => {
                    println!("✗ Failed: {}", err);
                    break;
                }
                step => {
                    println!("Status: {:?}", step);
                }
            }
        }
        drop(tracker);
        sleep(Duration::from_secs(1)).await;
    }
    
    Ok(())
}
```

---

## Configuration

### Environment Variables

DataFold S3 ingestion requires several environment variables:

#### Required: S3 Storage

```bash
# S3 storage mode (required for S3 ingestion)
export DATAFOLD_UPLOAD_STORAGE_MODE=s3

# Your DataFold S3 bucket for uploads/temp files
export DATAFOLD_UPLOAD_S3_BUCKET=my-datafold-bucket

# AWS region
export DATAFOLD_UPLOAD_S3_REGION=us-west-2
```

#### Required: AI Service

DataFold uses AI for schema inference. Choose one:

**Option 1: OpenRouter (Recommended)**
```bash
export OPENROUTER_API_KEY=sk-or-v1-xxx
```

**Option 2: Ollama (Local)**
```bash
export OLLAMA_URL=http://localhost:11434
export OLLAMA_MODEL=llama2  # optional
```

#### Optional: AWS Credentials

If not using IAM roles (e.g., Lambda execution role):

```bash
export AWS_ACCESS_KEY_ID=AKIAXXXXX
export AWS_SECRET_ACCESS_KEY=xxxxx
export AWS_REGION=us-west-2  # optional
```

#### Optional: Database Storage

```bash
# Database storage location (default: "data")
export DATAFOLD_STORAGE_PATH=./my-data

# Or use S3 for database storage (for serverless)
export DATAFOLD_STORAGE_MODE=s3
export DATAFOLD_S3_BUCKET=my-db-bucket
export DATAFOLD_S3_REGION=us-west-2
```

### Programmatic Configuration

Instead of environment variables, you can configure programmatically:

```rust
use datafold::storage::{UploadStorage, S3StorageConfig};
use datafold::ingestion::IngestionConfig;

// S3 Storage
let s3_config = S3StorageConfig {
    bucket: "my-bucket".to_string(),
    region: "us-west-2".to_string(),
    prefix: Some("uploads".to_string()),
};
let upload_storage = UploadStorage::s3(s3_config);

// Ingestion Config
let ingestion_config = IngestionConfig {
    openrouter_api_key: Some("sk-or-v1-xxx".to_string()),
    ollama_url: None,
    ollama_model: None,
};
```

---

## Common Patterns

### Pattern 1: Lambda with Initialization Optimization

Initialize dependencies once per Lambda container:

```rust
use once_cell::sync::Lazy;

static DATAFOLD: Lazy<DataFoldContext> = Lazy::new(|| {
    DataFoldContext::new().expect("Failed to initialize DataFold")
});

struct DataFoldContext {
    upload_storage: UploadStorage,
    progress_tracker: ProgressTracker,
    node: Arc<Mutex<DataFoldNode>>,
    ingestion_config: IngestionConfig,
}

impl DataFoldContext {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            upload_storage: UploadStorage::from_env()?,
            progress_tracker: datafold::ingestion::create_progress_tracker(),
            node: Arc::new(Mutex::new(DataFoldNode::new("data")?)),
            ingestion_config: IngestionConfig::from_env()?,
        })
    }
}

async fn lambda_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    // Use DATAFOLD static instance
    let s3_path = extract_s3_path(&event)?;
    let request = S3IngestionRequest::new(s3_path);
    
    let response = ingest_from_s3_path_async(
        &request,
        &DATAFOLD.upload_storage,
        &DATAFOLD.progress_tracker,
        DATAFOLD.node.clone(),
        &DATAFOLD.ingestion_config,
    ).await?;
    
    Ok(json!({ "progress_id": response.progress_id }))
}
```

### Pattern 2: Batch with Concurrency Control

Process multiple files with controlled parallelism:

```rust
use futures::stream::{self, StreamExt};

async fn batch_with_concurrency(
    s3_paths: Vec<String>,
    max_concurrent: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let upload_storage = Arc::new(UploadStorage::from_env()?);
    let progress_tracker = Arc::new(datafold::ingestion::create_progress_tracker());
    let node = Arc::new(Mutex::new(DataFoldNode::new("data")?));
    let ingestion_config = Arc::new(IngestionConfig::from_env()?);
    
    stream::iter(s3_paths)
        .map(|s3_path| {
            let upload_storage = upload_storage.clone();
            let progress_tracker = progress_tracker.clone();
            let node = node.clone();
            let ingestion_config = ingestion_config.clone();
            
            async move {
                let request = S3IngestionRequest::new(s3_path.clone());
                ingest_from_s3_path_async(
                    &request,
                    &upload_storage,
                    &progress_tracker,
                    node,
                    &ingestion_config,
                ).await
            }
        })
        .buffer_unordered(max_concurrent)
        .for_each(|result| async {
            match result {
                Ok(response) => println!("Started: {:?}", response.progress_id),
                Err(e) => eprintln!("Error: {:?}", e),
            }
        })
        .await;
    
    Ok(())
}
```

### Pattern 3: Conditional Processing

Only process files that meet certain criteria:

```rust
async fn process_if_new(
    s3_path: String,
    last_processed: &HashMap<String, chrono::DateTime<Utc>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if file was modified since last processing
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_s3::Client::new(&config);
    
    let (bucket, key) = parse_s3_path(&s3_path)?;
    
    let head = client
        .head_object()
        .bucket(&bucket)
        .key(&key)
        .send()
        .await?;
    
    let last_modified = head.last_modified().unwrap();
    
    if let Some(last_proc) = last_processed.get(&s3_path) {
        if last_modified <= last_proc {
            println!("Skipping {} (not modified)", s3_path);
            return Ok(());
        }
    }
    
    // Process file
    println!("Processing {} (modified)", s3_path);
    let request = S3IngestionRequest::new(s3_path);
    // ... (ingestion code)
    
    Ok(())
}
```

---

## Error Handling

### Error Types

The `IngestionError` enum covers common failure scenarios:

```rust
pub enum IngestionError {
    InvalidS3Path(String),           // Invalid s3:// path format
    S3DownloadFailed(String),        // Failed to download from S3
    FileConversionFailed,            // Failed to convert to JSON
    SchemaInferenceFailed(String),   // AI schema inference failed
    MutationGenerationFailed(String),// Failed to generate mutations
    DatabaseError(String),           // Database operation failed
    ConfigError(String),             // Configuration error
}
```

### Handling Specific Errors

```rust
match ingest_from_s3_path_async(&request, ...).await {
    Ok(response) => {
        // Success
    }
    Err(IngestionError::InvalidS3Path(path)) => {
        // Path format is wrong - check the s3:// format
        eprintln!("Invalid path: {}", path);
    }
    Err(IngestionError::S3DownloadFailed(msg)) => {
        // S3 download failed - check credentials, permissions, path exists
        eprintln!("Download failed: {}", msg);
    }
    Err(IngestionError::FileConversionFailed) => {
        // File format not supported or corrupted
        eprintln!("Cannot convert file to JSON");
    }
    Err(IngestionError::SchemaInferenceFailed(msg)) => {
        // AI service failed - check API keys, service availability
        eprintln!("Schema inference failed: {}", msg);
    }
    Err(err) => {
        eprintln!("Other error: {:?}", err);
    }
}
```

### Retry Logic

Implement retry for transient failures:

```rust
use tokio::time::{sleep, Duration};

async fn ingest_with_retry(
    request: &S3IngestionRequest,
    max_retries: u32,
) -> Result<IngestionResponse, IngestionError> {
    let upload_storage = UploadStorage::from_env().unwrap();
    let progress_tracker = datafold::ingestion::create_progress_tracker();
    let node = Arc::new(Mutex::new(DataFoldNode::new("data").unwrap()));
    let ingestion_config = IngestionConfig::from_env().unwrap();
    
    let mut attempts = 0;
    
    loop {
        attempts += 1;
        
        match ingest_from_s3_path_async(
            request,
            &upload_storage,
            &progress_tracker,
            node.clone(),
            &ingestion_config,
        ).await {
            Ok(response) => return Ok(response),
            Err(e) if attempts < max_retries => {
                eprintln!("Attempt {} failed: {:?}, retrying...", attempts, e);
                sleep(Duration::from_secs(2u64.pow(attempts))).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

---

## Performance & Best Practices

### Performance Tips

1. **Reuse Components**
   ```rust
   // Initialize once, reuse many times
   let upload_storage = UploadStorage::from_env()?;
   let node = Arc::new(Mutex::new(DataFoldNode::new("data")?));
   
   // Use in multiple ingestions
   for s3_path in paths {
       ingest_from_s3_path_async(&request, &upload_storage, ..., node.clone(), ...).await?;
   }
   ```

2. **Use Async for Throughput**
   - Async mode for high throughput
   - Sync mode for sequential processing
   - Combine with concurrency control

3. **Right-Size Lambda**
   - Memory: 512MB-1GB typical
   - Timeout: 60-300s depending on file sizes
   - Use provisioned concurrency for consistent performance

4. **Monitor Progress**
   - Use progress_id to track long-running ingestions
   - Implement health checks
   - Log errors for debugging

### Best Practices

#### ✅ DO

- **Use async mode in Lambda** - Return quickly to avoid timeouts
- **Initialize once** - Reuse UploadStorage, Node, etc.
- **Handle errors gracefully** - Log and alert on failures
- **Validate S3 paths** - Check format before calling API
- **Use IAM roles** - Avoid hardcoded credentials
- **Monitor costs** - Track S3 API calls and data transfer

#### ❌ DON'T

- **Don't recreate node for each file** - It's expensive
- **Don't use sync in Lambda** - Can timeout on large files
- **Don't ignore errors** - Silent failures hide problems
- **Don't process huge files** - Consider chunking or streaming
- **Don't hardcode buckets** - Use environment variables
- **Don't skip testing** - Test with representative data

---

## Troubleshooting

### Problem: "Invalid S3 path format"

**Symptoms:**
```
Error: InvalidS3Path("my-file.json")
```

**Solution:**
Ensure path starts with `s3://`:
```rust
// ✅ Correct
"s3://bucket/file.json"
"s3://my-bucket/path/to/file.json"

// ❌ Wrong
"bucket/file.json"
"https://bucket.s3.amazonaws.com/file.json"
```

### Problem: "Failed to download from S3: Access Denied"

**Symptoms:**
```
Error: S3DownloadFailed("Access Denied")
```

**Solutions:**

1. **Check AWS credentials:**
   ```bash
   aws s3 ls s3://your-bucket/your-file.json
   ```

2. **Verify IAM permissions:**
   ```json
   {
     "Effect": "Allow",
     "Action": ["s3:GetObject", "s3:HeadObject"],
     "Resource": "arn:aws:s3:::your-bucket/*"
   }
   ```

3. **Check bucket policy** - Ensure bucket allows reads

### Problem: "Cannot download from S3 path when using local storage"

**Symptoms:**
```
Error: ConfigError("S3 storage not configured")
```

**Solution:**
Configure S3 storage mode:
```bash
export DATAFOLD_UPLOAD_STORAGE_MODE=s3
export DATAFOLD_UPLOAD_S3_BUCKET=my-bucket
export DATAFOLD_UPLOAD_S3_REGION=us-west-2
```

### Problem: "Schema inference failed"

**Symptoms:**
```
Error: SchemaInferenceFailed("API key invalid")
```

**Solutions:**

1. **Check API key:**
   ```bash
   echo $OPENROUTER_API_KEY  # Should show your key
   ```

2. **Verify AI service:**
   ```bash
   # For Ollama
   curl http://localhost:11434/api/tags
   ```

3. **Check environment:**
   ```rust
   let config = IngestionConfig::from_env()?;
   println!("Config: {:?}", config);
   ```

### Problem: Lambda timeouts

**Symptoms:**
- Lambda times out before ingestion completes

**Solutions:**

1. **Use async mode:**
   ```rust
   // ✅ Returns immediately
   ingest_from_s3_path_async(&request, ...).await?;
   
   // ❌ Waits for completion
   ingest_from_s3_path_sync(&request, ...).await?;
   ```

2. **Increase timeout:**
   - Set Lambda timeout to 300s
   - For large files, consider Step Functions

3. **Reduce trust_distance:**
   ```rust
   let request = S3IngestionRequest::new(path)
       .with_trust_distance(0);  // Faster matching
   ```

### Problem: High S3 costs

**Symptoms:**
- Unexpected S3 API call charges

**Solutions:**

1. **Enable S3 request metrics** - Monitor GET requests
2. **Batch similar files** - Process in groups
3. **Use same-region buckets** - Avoid cross-region transfer
4. **Cache results** - Don't reprocess unchanged files

---

## Additional Resources

### Documentation

- [S3 Ingestion Quick Start](S3_INGESTION_QUICK_START.md) - Quick reference guide
- [S3 File Path Ingestion](S3_FILE_PATH_INGESTION.md) - Detailed HTTP API docs
- [S3 Configuration](S3_CONFIGURATION.md) - S3 setup guide
- [Ingestion Workflow](INGESTION_WORKFLOW.md) - How ingestion works

### Examples

- [simple_s3_ingestion.rs](../examples/simple_s3_ingestion.rs) - Basic examples
- [lambda_s3_ingestion.rs](../examples/lambda_s3_ingestion.rs) - Lambda handler

### HTTP API Alternative

If you prefer HTTP/REST over Rust API:

```bash
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "s3FilePath=s3://my-bucket/file.json" \
  -F "autoExecute=true"
```

See [S3_INGESTION_QUICK_START.md](S3_INGESTION_QUICK_START.md) for HTTP API details.

---

## Summary

DataFold's S3 ingestion API provides a powerful way to process S3 files programmatically:

- **Two modes**: Async (background) or Sync (wait for completion)
- **Lambda-ready**: Perfect for serverless architectures
- **Flexible**: Works with any S3 bucket you have access to
- **Efficient**: No re-upload required, direct S3 access

### Quick Reference

```rust
// Async (returns immediately)
let response = ingest_from_s3_path_async(&request, ...).await?;
println!("Progress ID: {}", response.progress_id.unwrap());

// Sync (waits for completion)  
let response = ingest_from_s3_path_sync(&request, ...).await?;
println!("Mutations: {}", response.mutations_executed);
```

For questions or issues, please open an issue on GitHub.

---

**Version:** 0.1.4+  
**Last Updated:** November 2024

