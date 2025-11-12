# Upload Storage Configuration

## Overview

FoldDB supports two storage backends for uploaded files:
- **Local Filesystem** (default) - Files stored in `data/uploads/`
- **Amazon S3** - Files stored in an S3 bucket

This separation allows the database to remain in Sled/S3 sync while uploaded files (which can be large) go directly to S3, improving Lambda cold start times.

## Benefits of S3 Upload Storage

### For AWS Lambda Deployments

✅ **Faster cold starts** - No need to download uploaded files  
✅ **Unlimited storage** - Not limited by Lambda's ephemeral storage  
✅ **Cost-effective** - S3 is cheaper than Lambda storage  
✅ **Scalable** - Handle large file uploads without affecting database size  
✅ **Cleaner separation** - Blob storage separate from database

### Storage Comparison

| Aspect | Local | S3 |
|--------|-------|-----|
| **Setup** | Zero config | Requires S3 bucket + IAM |
| **Cold start** | Slower (downloads files) | Faster (no download) |
| **Cost** | Free (local disk) | ~$0.023/GB/month |
| **Scalability** | Limited by disk | Unlimited |
| **Lambda-friendly** | ❌ Uses ephemeral storage | ✅ No local storage needed |

## Configuration

### Environment Variables

#### Local Storage (Default)

```bash
# Uses local filesystem (default)
DATAFOLD_UPLOAD_STORAGE_MODE=local
DATAFOLD_UPLOAD_PATH=data/uploads  # Optional, defaults to data/uploads
```

#### S3 Storage

```bash
# Use S3 for uploads
DATAFOLD_UPLOAD_STORAGE_MODE=s3
DATAFOLD_UPLOAD_S3_BUCKET=my-uploads-bucket
DATAFOLD_UPLOAD_S3_REGION=us-west-2
DATAFOLD_UPLOAD_S3_PREFIX=uploads  # Optional, defaults to "uploads"
```

### AWS Permissions

Your Lambda execution role needs these S3 permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:GetObject",
        "s3:HeadObject"
      ],
      "Resource": "arn:aws:s3:::my-uploads-bucket/*"
    }
  ]
}
```

## Usage Examples

### Local Development

```bash
# Start server with default local storage
./run_http_server.sh

# Files will be saved to: data/uploads/
```

### AWS Lambda with S3

```bash
# Set environment variables
export DATAFOLD_UPLOAD_STORAGE_MODE=s3
export DATAFOLD_UPLOAD_S3_BUCKET=my-folddb-uploads
export DATAFOLD_UPLOAD_S3_REGION=us-west-2

# Deploy to Lambda
# Files will be uploaded to: s3://my-folddb-uploads/uploads/
```

### Mixed Configuration

You can use different storage for the database vs. uploads:

```bash
# Database: S3 sync (for persistence)
export DATAFOLD_STORAGE_MODE=s3
export DATAFOLD_S3_BUCKET=my-db-bucket
export DATAFOLD_S3_REGION=us-west-2
export DATAFOLD_S3_PREFIX=database

# Uploads: Direct to S3
export DATAFOLD_UPLOAD_STORAGE_MODE=s3
export DATAFOLD_UPLOAD_S3_BUCKET=my-uploads-bucket  # Can be same or different bucket
export DATAFOLD_UPLOAD_S3_REGION=us-west-2
export DATAFOLD_UPLOAD_S3_PREFIX=uploads
```

## Upload Flow

### Local Storage

```
User uploads file
  ↓
Save to data/uploads/HASH_filename
  ↓
Process directly from data/uploads/HASH_filename (no /tmp needed)
  ↓
Convert to JSON
  ↓
Ingest into database
```

### S3 Storage

```
User uploads file
  ↓
Upload to S3: s3://bucket/uploads/HASH_filename (permanent storage)
  ↓
Also save to /tmp/HASH_filename (for processing, since file_to_json needs local file)
  ↓
Process from /tmp/HASH_filename
  ↓
Convert to JSON
  ↓
Ingest into database
  ↓
OS cleans up /tmp automatically
```

## File Naming

Files are stored with content-based hashing for deduplication:

```
Format: {HASH}_{ORIGINAL_FILENAME}
Example: 52a86a1f1babfb3d_tweets.js

Where:
- HASH: First 16 characters of SHA256 hash of file contents
- ORIGINAL_FILENAME: Original filename from upload
```

**Benefits:**
- Automatic deduplication (same content = same hash)
- Prevents overwrites of different files with same name
- Preserves original filename for reference

## Cost Estimation

### S3 Upload Storage Costs (us-west-2)

**Scenario:** 1000 file uploads/month, average 500KB per file

**Monthly costs:**
- Storage: 0.5 GB × $0.023/GB = **$0.012**
- PUT requests: 1000 × $0.005/1000 = **$0.005**
- GET requests (downloads): 1000 × $0.0004/1000 = **$0.0004**
- **Total: ~$0.02/month**

### Lambda Impact

**Without S3 uploads (Local storage in Lambda):**
- Cold start downloads: 50+ files × 500KB = 25MB download
- Cold start time: +500ms-1s
- Lambda memory needed: +25MB
- Processing: Direct from downloaded files

**With S3 uploads:**
- Cold start downloads: 0 (uploads go directly to S3, no download needed)
- Cold start time: No impact from stored files
- Lambda memory needed: No extra memory for stored files
- Processing: Download only current upload to /tmp (temporary, cleaned up)
- /tmp usage: Only current file being processed (typically < 10MB)

**Savings:** Faster cold starts + lower memory requirements + efficient /tmp usage

## Troubleshooting

### Error: "Missing environment variable: DATAFOLD_UPLOAD_S3_BUCKET"

You set `DATAFOLD_UPLOAD_STORAGE_MODE=s3` but didn't provide the bucket name.

**Fix:**
```bash
export DATAFOLD_UPLOAD_S3_BUCKET=your-bucket-name
export DATAFOLD_UPLOAD_S3_REGION=us-west-2
```

### Error: "Failed to save file: S3 error"

Check AWS permissions. Your Lambda role needs `s3:PutObject` permission.

### Files not appearing in S3

Check the S3 prefix. Files are uploaded to:
```
s3://{BUCKET}/{PREFIX}/{HASH}_{FILENAME}
```

Default prefix is `uploads/`, so files appear at:
```
s3://my-bucket/uploads/52a86a1f1babfb3d_tweets.js
```

### Permission denied in Lambda

Ensure Lambda execution role has:
1. `s3:PutObject` - for uploads
2. `s3:GetObject` - for reading during processing
3. `s3:HeadObject` - for checking file existence

## Implementation Details

### Architecture

The upload storage abstraction (`UploadStorage`) provides a unified interface:

```rust
pub enum UploadStorage {
    Local { path: PathBuf },
    S3 { bucket: String, prefix: String, client: Client },
}
```

**Methods:**
- `save_file(filename, data)` - Save file to storage
- `save_file_if_not_exists(filename, data)` - Atomically save file only if it doesn't exist (prevents race conditions)
- `read_file(filename)` - Read file from storage
- `file_exists(filename)` - Check if file exists
- `get_display_path(filename)` - Get display path for logging

### Atomic Duplicate Detection

The storage abstraction provides **race-free duplicate detection** through the `save_file_if_not_exists` method:

**Local Storage:**
- Uses `OpenOptions::create_new(true)` for atomic file creation
- Fails with `AlreadyExists` error if file already exists
- Prevents race condition between check and create

**S3 Storage:**
- Uses conditional PUT with `if-none-match: *` header
- Only creates object if it doesn't already exist
- Returns `PreconditionFailed` (412) if object exists

This ensures that concurrent uploads of the same file are detected atomically, preventing duplicate ingestion even under high concurrency.

### Processing Optimization

The implementation is optimized to minimize disk I/O:

**Local Storage:**
- File is saved once to permanent location (`data/uploads/`)
- Processing happens directly from that location
- No temporary `/tmp` files created

**S3 Storage:**
- File is saved to S3 for permanent storage
- Also saved to `/tmp` for immediate processing (since `file_to_json` requires local file path)
- `/tmp` file is automatically cleaned up by OS

This approach minimizes disk writes while ensuring compatibility with the `file_to_json` library which requires local file paths.

### Integration Points

1. **Multipart Parser** (`src/ingestion/multipart_parser.rs`)
   - Receives UploadStorage from AppState
   - Uses abstraction to save uploaded files
   - Works with both Local and S3 backends

2. **File Upload Handler** (`src/ingestion/file_upload.rs`)
   - Passes upload_storage to multipart parser
   - No other changes needed

3. **HTTP Server** (`src/datafold_node/http_server.rs`)
   - Initializes upload storage from environment
   - Adds to AppState for request handlers

## Best Practices

### Development
- Use **local storage** for development
- Simpler setup, faster iteration
- No AWS costs
- No temporary `/tmp` files (processes directly from `data/uploads/`)

### Production
- Use **S3 storage** for Lambda deployments
- Better scalability and performance
- Lower cold start times
- Efficient `/tmp` usage (only current upload)

### Hybrid
- Database: Local storage (fast queries)
- Uploads: S3 storage (blob storage)
- Best of both worlds for serverless deployments

### Performance Tips
- **Local storage**: Zero overhead - single write, process in place
- **S3 storage**: Dual write (S3 + /tmp) but no cold start penalty
- Uploaded files are automatically deduplicated using content hashing

## Migration

### Local to S3

```bash
# 1. Upload existing files to S3
aws s3 sync data/uploads/ s3://my-bucket/uploads/

# 2. Switch to S3 mode
export DATAFOLD_UPLOAD_STORAGE_MODE=s3
export DATAFOLD_UPLOAD_S3_BUCKET=my-bucket
export DATAFOLD_UPLOAD_S3_REGION=us-west-2

# 3. Restart server
./run_http_server.sh
```

### S3 to Local

```bash
# 1. Download files from S3
aws s3 sync s3://my-bucket/uploads/ data/uploads/

# 2. Switch to local mode
export DATAFOLD_UPLOAD_STORAGE_MODE=local

# 3. Restart server
./run_http_server.sh
```

## Related Documentation

- [S3 Configuration](S3_CONFIGURATION.md) - S3 setup for database storage
- [S3 Implementation Summary](S3_IMPLEMENTATION_SUMMARY.md) - Database S3 sync implementation
- [Ingestion Engine](ingestion_engine.md) - File ingestion process

