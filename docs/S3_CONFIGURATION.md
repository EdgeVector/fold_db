# S3 Storage Configuration

This document describes how to configure FoldDB to use S3-backed storage for serverless deployments (AWS Lambda, Cloudflare Workers, etc).

## Quick Start

### Option 1: Environment Variables (Recommended)

Set these environment variables:

```bash
# Enable S3 storage mode
export DATAFOLD_STORAGE_MODE=s3

# S3 configuration (required)
export DATAFOLD_S3_BUCKET=my-folddb-bucket
export DATAFOLD_S3_REGION=us-west-2

# Optional: customize S3 prefix and local cache path
export DATAFOLD_S3_PREFIX=production/folddb  # defaults to "folddb"
export DATAFOLD_S3_LOCAL_PATH=/tmp/folddb-data  # defaults to "/tmp/folddb-data"
```

Then use FoldDB normally with automatic S3 configuration:

```rust
use datafold::{FoldDB, StorageConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads configuration from environment variables
    let config = StorageConfig::from_env()?;
    
    match config {
        StorageConfig::S3 { config } => {
            let db = FoldDB::new_with_s3(config).await?;
            
            // Use database normally
            // ... perform operations ...
            
            // Sync to S3 when done
            db.flush_to_s3().await?;
        }
        StorageConfig::Local { path } => {
            let db = FoldDB::new(path.to_str().unwrap())?;
            // ... use local storage ...
        }
    }
    
    Ok(())
}
```

### Option 2: Programmatic Configuration

```rust
use datafold::{FoldDB, S3Config};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let s3_config = S3Config::new(
        "my-folddb-bucket".to_string(),
        "us-west-2".to_string(),
        "production/folddb".to_string(),
    ).with_local_path(PathBuf::from("/tmp/folddb-data"));
    
    let db = FoldDB::new_with_s3(s3_config).await?;
    
    // Use database normally
    // ...
    
    // Sync to S3
    db.flush_to_s3().await?;
    
    Ok(())
}
```

## How It Works

FoldDB with S3 storage works using a simple file sync approach:

1. **On Startup** (`new_with_s3()`):
   - Downloads entire Sled database directory from S3 to local disk (e.g., `/tmp/folddb-data`)
   - If no data exists in S3, starts with empty database
   - Opens Sled database from local path

2. **During Operation**:
   - All operations use Sled normally (local disk)
   - No network calls to S3 during queries/mutations
   - Full Sled performance

3. **On Flush** (`flush_to_s3()`):
   - Flushes Sled to local disk
   - Uploads entire directory to S3
   - Call this periodically or before Lambda shutdown

## Environment Variables Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATAFOLD_STORAGE_MODE` | No | `local` | Storage mode: `local` or `s3` |
| `DATAFOLD_STORAGE_PATH` | No | `data` | Path for local storage (when mode=local) |
| `DATAFOLD_S3_BUCKET` | Yes (for S3) | - | S3 bucket name |
| `DATAFOLD_S3_REGION` | Yes (for S3) | - | AWS region (e.g., `us-west-2`) |
| `DATAFOLD_S3_PREFIX` | No | `folddb` | Prefix/path within bucket |
| `DATAFOLD_S3_LOCAL_PATH` | No | `/tmp/folddb-data` | Local cache directory |

## AWS Lambda Configuration

### Lambda Function Setup

```python
# lambda_function.py
import subprocess
import os

def handler(event, context):
    # Set S3 configuration
    os.environ['DATAFOLD_STORAGE_MODE'] = 's3'
    os.environ['DATAFOLD_S3_BUCKET'] = 'my-folddb-bucket'
    os.environ['DATAFOLD_S3_REGION'] = 'us-west-2'
    
    # Your Lambda handler code that uses FoldDB
    # ...
```

### IAM Permissions

Your Lambda function needs these S3 permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:GetObject",
        "s3:PutObject",
        "s3:ListBucket"
      ],
      "Resource": [
        "arn:aws:s3:::my-folddb-bucket/*",
        "arn:aws:s3:::my-folddb-bucket"
      ]
    }
  ]
}
```

### Memory and Timeout

- **Memory**: Allocate enough for your database size + overhead (e.g., 512MB for 200MB database)
- **Timeout**: Account for download/upload time (e.g., 60 seconds)
- **/tmp size**: Lambda provides up to 10GB of /tmp storage

## Flush Strategies

### Manual Flush

Call `flush_to_s3()` explicitly when needed:

```rust
// After important operations
db.flush_to_s3().await?;
```

### Periodic Flush

Flush on a schedule:

```rust
use tokio::time::{interval, Duration};

let mut flush_interval = interval(Duration::from_secs(300)); // 5 minutes

loop {
    tokio::select! {
        _ = flush_interval.tick() => {
            if let Err(e) = db.flush_to_s3().await {
                eprintln!("Failed to flush to S3: {}", e);
            }
        }
        // ... other operations
    }
}
```

### Lambda Shutdown Hook

Flush before Lambda terminates:

```rust
// In Lambda handler
async fn lambda_handler(event: Request) -> Result<Response, Error> {
    let db = FoldDB::new_with_s3(config).await?;
    
    // Process request
    let result = process_request(&db, event).await?;
    
    // Flush before returning
    db.flush_to_s3().await?;
    
    Ok(result)
}
```

## Performance Considerations

### Cold Start Time

- Depends on database size
- Examples:
  - 10 MB: ~200-500ms
  - 100 MB: ~1-2 seconds
  - 500 MB: ~5-10 seconds

### Storage Costs (S3 Standard, us-west-2)

For a 200MB database with 10 flushes/day:
- Storage: 0.2 GB × $0.023/GB = $0.0046/month
- PUT requests: 300/month × $0.005/1000 = $0.0015/month
- GET requests (cold starts): ~$0.001/month
- **Total: ~$0.01/month**

### Optimization Tips

1. **Minimize flush frequency**: Only flush when necessary
2. **Right-size /tmp**: Use smallest Lambda /tmp that fits your database
3. **Monitor database growth**: Alert when approaching Lambda limits
4. **Use Lambda warming**: Keep Lambda warm to avoid cold starts

## Troubleshooting

### Error: "S3 storage not configured"

You called `flush_to_s3()` on a local FoldDB instance. Check:

```rust
if db.has_s3_storage() {
    db.flush_to_s3().await?;
}
```

### Error: "Missing environment variable: DATAFOLD_S3_BUCKET"

Set required S3 environment variables:
```bash
export DATAFOLD_S3_BUCKET=my-bucket
export DATAFOLD_S3_REGION=us-west-2
```

### Slow cold starts

- Reduce database size (archive old data)
- Use Lambda provisioned concurrency
- Consider compression (future enhancement)

### Permission denied errors

Check IAM permissions for Lambda role. Needs `s3:GetObject`, `s3:PutObject`, and `s3:ListBucket`.

## Migration

### Local to S3

```bash
# 1. Ensure local database is flushed
# 2. Upload local database directory to S3 manually
aws s3 sync ./data s3://my-bucket/folddb/

# 3. Switch to S3 mode
export DATAFOLD_STORAGE_MODE=s3
export DATAFOLD_S3_BUCKET=my-bucket
export DATAFOLD_S3_REGION=us-west-2
```

### S3 to Local

```bash
# 1. Download from S3
aws s3 sync s3://my-bucket/folddb/ ./data

# 2. Use local mode
export DATAFOLD_STORAGE_MODE=local
export DATAFOLD_STORAGE_PATH=./data
```

## Examples

See the `examples/s3_storage/` directory for complete examples:
- `basic_s3.rs` - Simple S3 usage
- `lambda_handler.rs` - AWS Lambda integration
- `periodic_flush.rs` - Background flush task

