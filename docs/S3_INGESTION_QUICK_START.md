# S3 Ingestion Quick Start

Quick reference for using DataFold's S3 file path ingestion feature.

## 🚀 Three Ways to Ingest S3 Files

### 1. HTTP API (curl/REST)

```bash
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "s3FilePath=s3://my-bucket/data.json" \
  -F "autoExecute=true"
```

### 2. Web UI

1. Navigate to http://localhost:9001
2. Go to File Upload tab
3. Select "S3 File Path" radio button
4. Enter: `s3://my-bucket/path/to/file.json`
5. Click "Process S3 File"

### 3. Programmatic API (Rust/Lambda)

**Async (Returns Immediately):**
```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};

let request = S3IngestionRequest::new("s3://bucket/file.json".to_string());
let response = ingest_from_s3_path_async(&request, &state).await?;
println!("Started: {}", response.progress_id.unwrap());
```

**Sync (Waits for Completion):**
```rust
use datafold::ingestion::{ingest_from_s3_path_sync, S3IngestionRequest};

let request = S3IngestionRequest::new("s3://bucket/file.json".to_string())
    .with_auto_execute(true)
    .with_trust_distance(0);
    
let response = ingest_from_s3_path_sync(&request, &state).await?;
println!("Complete: {} mutations", response.mutations_executed);
```

## ⚙️ Configuration

### Environment Variables

```bash
# Required for S3 ingestion
export DATAFOLD_UPLOAD_STORAGE_MODE=s3
export DATAFOLD_UPLOAD_S3_BUCKET=my-uploads-bucket
export DATAFOLD_UPLOAD_S3_REGION=us-west-2

# AWS credentials (automatically picked up)
export AWS_ACCESS_KEY_ID=your-key
export AWS_SECRET_ACCESS_KEY=your-secret
```

### AWS Permissions

```json
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Action": ["s3:GetObject", "s3:HeadObject"],
    "Resource": "arn:aws:s3:::*/*"
  }]
}
```

## 📝 S3 Path Format

```
s3://bucket-name/path/to/file.ext
```

**Examples:**
- `s3://my-data-bucket/tweets.json`
- `s3://analytics/2024/data.csv`
- `s3://uploads/users/john/document.pdf`

## 🎯 Common Patterns

### Lambda S3 Event Handler

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};
use lambda_runtime::{service_fn, LambdaEvent, Error};

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let bucket = event.payload["Records"][0]["s3"]["bucket"]["name"].as_str()?;
    let key = event.payload["Records"][0]["s3"]["object"]["key"].as_str()?;
    let s3_path = format!("s3://{}/{}", bucket, key);
    
    let request = S3IngestionRequest::new(s3_path);
    let response = ingest_from_s3_path_async(&request, &state).await?;
    
    Ok(json!({ "progress_id": response.progress_id }))
}
```

### Batch Processing

```rust
let files = vec![
    "s3://bucket/file1.json",
    "s3://bucket/file2.json",
    "s3://bucket/file3.json",
];

for file in files {
    let request = S3IngestionRequest::new(file.to_string());
    let response = ingest_from_s3_path_async(&request, &state).await?;
    println!("Processing: {}", response.progress_id.unwrap());
}
```

### Custom Settings

```rust
let request = S3IngestionRequest::new("s3://bucket/file.json".to_string())
    .with_auto_execute(false)       // Don't auto-execute mutations
    .with_trust_distance(5)          // Custom trust distance
    .with_pub_key("my-key".to_string());  // Custom auth key
```

## ❓ Troubleshooting

### Error: "Cannot download from S3 path when using local storage"

**Solution:** Enable S3 storage mode:
```bash
export DATAFOLD_UPLOAD_STORAGE_MODE=s3
export DATAFOLD_UPLOAD_S3_BUCKET=my-bucket
export DATAFOLD_UPLOAD_S3_REGION=us-west-2
```

### Error: "Invalid S3 path format"

**Solution:** Ensure path starts with `s3://`:
```
✅ s3://bucket/file.json
❌ bucket/file.json
❌ https://bucket.s3.amazonaws.com/file.json
```

### Error: "Failed to download S3 file: Access Denied"

**Solution:** Check AWS credentials and IAM permissions:
```bash
# Test access
aws s3 ls s3://your-bucket/your-file.json
```

## 📚 More Information

- **Full Documentation:** [S3_FILE_PATH_INGESTION.md](S3_FILE_PATH_INGESTION.md)
- **Lambda Example:** [../examples/lambda_s3_ingestion.rs](../examples/lambda_s3_ingestion.rs)
- **Simple Example:** [../examples/simple_s3_ingestion.rs](../examples/simple_s3_ingestion.rs)
- **Main README:** [../README.md](../README.md)

## 🎉 Benefits

- ✅ No file re-upload (save bandwidth)
- ✅ Lambda-ready (perfect for serverless)
- ✅ Same processing pipeline as regular uploads
- ✅ Works with any S3 bucket you have access to

---

**Quick Start Time:** < 5 minutes  
**Difficulty:** Easy  
**Version:** 0.1.4+

