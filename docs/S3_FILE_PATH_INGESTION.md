# S3 File Path Ingestion

## Overview

The file ingestion API now supports direct S3 file paths as an alternative to uploading files. This allows you to process files already stored in S3 without re-uploading them, saving time and bandwidth.

## Benefits

✅ **No Re-upload Required** - Files already in S3 are downloaded directly for processing  
✅ **Bandwidth Savings** - Avoid uploading large files when they're already in S3  
✅ **Flexible Workflows** - Process files from any S3 location  
✅ **Same Processing Pipeline** - Files are converted to JSON and processed identically  

## API Usage

### HTTP API

**Endpoint:**
```
POST /api/ingestion/upload
Content-Type: multipart/form-data
```

### Programmatic API (Rust)

**For Lambda Functions and Rust Code:**

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};

// Async (returns immediately with progress_id)
let request = S3IngestionRequest::new("s3://bucket/file.json".to_string())
    .with_auto_execute(true);
let ingestion_config = IngestionConfig::from_env()?;
let response = ingest_from_s3_path_async(&request, &upload_storage, &progress_tracker, node, &ingestion_config).await?;

// Sync (waits for completion)
use datafold::ingestion::ingest_from_s3_path_sync;
let response = ingest_from_s3_path_sync(&request, &upload_storage, &progress_tracker, node, &ingestion_config).await?;
```

See [Lambda Example](../examples/lambda_s3_ingestion.rs) for complete AWS Lambda integration.

### Input Modes

You can provide **either** a file upload **or** an S3 file path (not both):

#### Mode 1: Traditional File Upload

```bash
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "file=@/path/to/local/file.json" \
  -F "autoExecute=true" \
  -F "trustDistance=0" \
  -F "pubKey=default"
```

#### Mode 2: S3 File Path (NEW)

```bash
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "s3FilePath=s3://my-bucket/path/to/file.json" \
  -F "autoExecute=true" \
  -F "trustDistance=0" \
  -F "pubKey=default"
```

### Request Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `file` | Binary | Conditional* | File to upload (traditional mode) |
| `s3FilePath` | String | Conditional* | S3 path (e.g., `s3://bucket/key`) (new mode) |
| `autoExecute` | Boolean | Optional | Auto-execute mutations (default: true) |
| `trustDistance` | Number | Optional | Trust distance (default: 0) |
| `pubKey` | String | Optional | Public key (default: "default") |

*Either `file` or `s3FilePath` must be provided, but not both.

### S3 Path Format

The S3 path must follow this format:

```
s3://bucket-name/path/to/file.ext
```

Examples:
- `s3://my-data-bucket/tweets.json`
- `s3://analytics/2024/data.csv`
- `s3://uploads/users/john/document.pdf`

### Response

Both modes return the same response format:

```json
{
  "success": true,
  "progress_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "File upload and ingestion started. Use progress_id to track status.",
  "file_path": "/tmp/file.json",
  "duplicate": false
}
```

## How It Works

### Traditional File Upload Flow

```
User uploads file
  ↓
Save to storage (local or S3)
  ↓
Convert to JSON
  ↓
Process and ingest
```

### S3 File Path Flow

```
User provides S3 path
  ↓
Parse bucket and key from path
  ↓
Download file from S3 to /tmp
  ↓
Convert to JSON
  ↓
Process and ingest
```

**Key Difference:** When using S3 file path, the file is downloaded from the provided S3 location but **not** re-uploaded to the configured upload storage bucket.

## Requirements

### Backend Configuration

To use S3 file paths, your server must have S3 storage configured:

```bash
export DATAFOLD_UPLOAD_STORAGE_MODE=s3
export DATAFOLD_UPLOAD_S3_BUCKET=my-bucket
export DATAFOLD_UPLOAD_S3_REGION=us-west-2
```

If you're using local storage mode, attempting to use S3 file paths will return an error:

```json
{
  "success": false,
  "error": "Cannot download from S3 path when using local storage. Configure S3 storage or upload file directly."
}
```

### AWS Permissions

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

**Note:** The permissions need to cover any S3 bucket you want to read from, not just your configured upload bucket.

## UI Integration

The web UI includes a mode toggle to switch between file upload and S3 path input:

### Upload File Mode (Default)

- Drag and drop interface
- File browser
- Traditional file upload

### S3 File Path Mode

- Text input for S3 path
- Real-time validation
- Same processing pipeline

To switch modes in the UI:
1. Navigate to the File Upload tab
2. Select "S3 File Path" radio button
3. Enter your S3 path
4. Click "Process S3 File"

## Use Cases

### 1. Processing ETL Pipeline Output

```bash
# Your ETL writes to S3
aws s3 cp output.json s3://etl-bucket/output/

# Process it directly
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "s3FilePath=s3://etl-bucket/output/output.json" \
  -F "autoExecute=true"
```

### 2. Batch Processing Existing S3 Files

```bash
# List all files in S3
aws s3 ls s3://data-bucket/files/ --recursive | while read -r line; do
  file=$(echo $line | awk '{print $4}')
  
  # Process each file
  curl -X POST http://localhost:9001/api/ingestion/upload \
    -F "s3FilePath=s3://data-bucket/$file" \
    -F "autoExecute=true"
done
```

### 3. Lambda Triggered Ingestion

When a file is uploaded to S3, trigger Lambda to process it:

```python
import json
import urllib.request

def lambda_handler(event, context):
    # Get S3 bucket and key from event
    bucket = event['Records'][0]['s3']['bucket']['name']
    key = event['Records'][0]['s3']['object']['key']
    s3_path = f"s3://{bucket}/{key}"
    
    # Prepare multipart form data
    boundary = '----WebKitFormBoundary7MA4YWxkTrZu0gW'
    body = (
        f'--{boundary}\r\n'
        f'Content-Disposition: form-data; name="s3FilePath"\r\n\r\n'
        f'{s3_path}\r\n'
        f'--{boundary}\r\n'
        f'Content-Disposition: form-data; name="autoExecute"\r\n\r\n'
        f'true\r\n'
        f'--{boundary}--\r\n'
    ).encode()
    
    # Call ingestion API
    req = urllib.request.Request(
        'https://your-folddb-api.com/api/ingestion/upload',
        data=body,
        headers={'Content-Type': f'multipart/form-data; boundary={boundary}'}
    )
    
    with urllib.request.urlopen(req) as response:
        result = json.loads(response.read())
        print(f"Ingestion started: {result['progress_id']}")
    
    return {'statusCode': 200}
```

## Error Handling

### Invalid S3 Path Format

```json
{
  "success": false,
  "error": "Invalid S3 path format. Expected 's3://bucket/key', got: invalid-path"
}
```

### S3 Download Failed

```json
{
  "success": false,
  "error": "Failed to download S3 file: NoSuchKey: The specified key does not exist."
}
```

### Both File and S3 Path Provided

```json
{
  "success": false,
  "error": "Cannot provide both 'file' and 's3FilePath' - use one or the other"
}
```

### Local Storage Mode (S3 Not Configured)

```json
{
  "success": false,
  "error": "Cannot download from S3 path when using local storage. Configure S3 storage or upload file directly."
}
```

## Testing

### Test with curl

```bash
# Test S3 path ingestion
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "s3FilePath=s3://your-bucket/test.json" \
  -F "autoExecute=true" \
  -v

# Test error handling (invalid path)
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "s3FilePath=invalid-path" \
  -F "autoExecute=true"

# Test error handling (both file and s3FilePath)
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "file=@test.json" \
  -F "s3FilePath=s3://bucket/file.json" \
  -F "autoExecute=true"
```

### Test with UI

1. Start the server: `./run_http_server.sh`
2. Navigate to http://localhost:9001
3. Go to File Upload tab
4. Switch to "S3 File Path" mode
5. Enter: `s3://your-bucket/path/to/file.json`
6. Click "Process S3 File"
7. Watch the progress bar for status

## Implementation Details

### Backend Changes

**Files Modified:**
- `src/ingestion/multipart_parser.rs` - Added S3 path field parsing
- `src/storage/upload_storage.rs` - Added `download_from_s3_path()` method
- `src/ingestion/file_upload.rs` - Updated API documentation

**Key Functions:**

```rust
// Parse S3 path from multipart form
async fn handle_s3_file_path(
    s3_path: &str,
    upload_storage: &UploadStorage,
) -> Result<(PathBuf, String), HttpResponse>

// Download from any S3 bucket/key
pub async fn download_from_s3_path(
    &self,
    bucket: &str,
    key: &str,
) -> StorageResult<Vec<u8>>
```

### Frontend Changes

**Files Modified:**
- `src/datafold_node/static-react/src/components/tabs/FileUploadTab.jsx`

**New Features:**
- Mode toggle (Upload File / S3 File Path)
- S3 path input field with validation
- Dynamic button text and validation
- Context-aware help text

## Performance Considerations

### Network Transfer
- **Traditional Upload:** Local → Server → S3 (if S3 storage configured)
- **S3 Path:** S3 → Server (single transfer)

### Use S3 Path When:
- ✅ Files are already in S3
- ✅ Files are large (GB+)
- ✅ Processing multiple files from S3
- ✅ Triggered by S3 events (Lambda)

### Use Traditional Upload When:
- ✅ Files are local
- ✅ Files are small (MB)
- ✅ S3 storage not configured
- ✅ One-time manual uploads

## Security Considerations

### Access Control

The server's AWS credentials determine which S3 buckets can be accessed:
- Restrict IAM permissions to specific buckets if needed
- Use least-privilege principle
- Monitor S3 access logs

### Path Validation

The API validates S3 paths:
- Must start with `s3://`
- Must have bucket and key components
- Invalid formats are rejected

### No Re-upload Protection

S3 files are **not** deduplicated like uploaded files. If you process the same S3 path twice:
- It will download and process twice
- Two separate ingestion runs will occur
- Consider tracking processed files in your application

## Limitations

1. **S3 Storage Required:** Local storage mode cannot use S3 file paths
2. **Same Region Preferred:** Cross-region S3 transfers are slower
3. **AWS Credentials Required:** Server must have S3 access
4. **No Deduplication:** Unlike file uploads, S3 paths aren't deduplicated
5. **Temporary Download:** Files are downloaded to `/tmp` (cleaned up by OS)

## Migration Guide

### Updating Existing Workflows

If you have existing file upload workflows:

**Before:**
```bash
# Upload file from local
curl -X POST http://api/ingestion/upload \
  -F "file=@/local/data.json"
```

**After (if file is in S3):**
```bash
# Use S3 path instead
curl -X POST http://api/ingestion/upload \
  -F "s3FilePath=s3://bucket/data.json"
```

No other changes needed - the processing pipeline is identical.

## Troubleshooting

### Problem: "Cannot download from S3 path when using local storage"

**Solution:** Configure S3 upload storage:
```bash
export DATAFOLD_UPLOAD_STORAGE_MODE=s3
export DATAFOLD_UPLOAD_S3_BUCKET=my-bucket
export DATAFOLD_UPLOAD_S3_REGION=us-west-2
```

### Problem: "Failed to download from S3: Access Denied"

**Solution:** Check AWS credentials and IAM permissions:
```bash
aws s3 ls s3://your-bucket/your-file.json  # Test access
```

### Problem: "Invalid S3 path format"

**Solution:** Ensure path starts with `s3://` and has bucket and key:
```
✅ s3://bucket/file.json
✅ s3://bucket/path/to/file.json
❌ bucket/file.json
❌ https://bucket.s3.amazonaws.com/file.json
```

## Related Documentation

- [Upload Storage Configuration](UPLOAD_STORAGE.md) - Configure S3 upload storage
- [S3 Configuration](S3_CONFIGURATION.md) - S3 setup guide
- [Ingestion Engine](ingestion_engine.md) - File processing pipeline
- [API Documentation](../target/openapi.json) - Full API reference

## Changelog

### Version 0.1.3 (2024)
- ✨ Added S3 file path support to ingestion API
- ✨ Added UI toggle for S3 path vs file upload
- 🔧 Added `download_from_s3_path()` method to `UploadStorage`
- 📝 Updated API documentation
- ✅ All tests passing

