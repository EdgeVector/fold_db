# API Key Configuration Update

## Summary

Updated the S3 ingestion API to allow OpenRouter API keys to be passed as configuration parameters instead of requiring them to be read from environment variables.

## Changes Made

### Core API Changes

#### 1. `S3IngestionRequest` Struct
**File:** `src/ingestion/s3_ingestion.rs`

Added new field:
```rust
pub ingestion_config: Option<IngestionConfig>
```

Added new builder methods:
- `with_ingestion_config(config: IngestionConfig)` - Set complete configuration
- `with_openrouter_api_key(api_key: String)` - Convenience method to set just the API key
- `with_openrouter_config(api_key: String, model: String, base_url: String)` - Set OpenRouter-specific config

#### 2. Function Signatures

**`ingest_from_s3_path_async`** and **`ingest_from_s3_path_sync`**

**Before:**
```rust
pub async fn ingest_from_s3_path_async(
    request: &S3IngestionRequest,
    upload_storage: &UploadStorage,
    progress_tracker: &ProgressTracker,
    node: Arc<Mutex<DataFoldNode>>,
    ingestion_config: &IngestionConfig,
) -> Result<IngestionResponse, IngestionError>
```

**After:**
```rust
pub async fn ingest_from_s3_path_async(
    request: &S3IngestionRequest,
    upload_storage: &UploadStorage,
    progress_tracker: &ProgressTracker,
    node: Arc<Mutex<DataFoldNode>>,
    ingestion_config: Option<&IngestionConfig>,
) -> Result<IngestionResponse, IngestionError>
```

#### 3. Configuration Priority

The functions now check for configuration in this order:
1. Explicitly passed `ingestion_config` parameter
2. Configuration in `request.ingestion_config` (set via builder methods)
3. Environment variables via `IngestionConfig::from_env()`

### Updated Documentation

1. **`docs/S3_INGESTION_API_KEY_CONFIG.md`** (NEW)
   - Comprehensive guide on all configuration methods
   - Migration guide from old API
   - Best practices and security notes
   - Lambda and batch processing examples

2. **`docs/S3_FILE_PATH_INGESTION.md`**
   - Updated examples to show new API

3. **`docs/S3_INGESTION_QUICK_START.md`**
   - Updated with direct API key passing examples

4. **`docs/S3_INGESTION_API_GUIDE.md`**
   - Updated function signatures
   - Added examples of new configuration methods

### Updated Examples

1. **`examples/lambda_s3_ingestion.rs`**
   - Shows passing API key directly (recommended for Lambda)
   - Includes legacy approach as commented alternative

2. **`examples/simple_s3_ingestion.rs`**
   - Updated all examples to use direct API key passing

### New Tests

Added tests in `src/ingestion/s3_ingestion.rs`:
- `test_s3_ingestion_request_with_api_key()` - Tests `with_openrouter_api_key()`
- `test_s3_ingestion_request_with_config()` - Tests `with_ingestion_config()`

## Usage Examples

### New Recommended Approach

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};

let request = S3IngestionRequest::new("s3://bucket/file.json".to_string())
    .with_auto_execute(true)
    .with_openrouter_api_key("your-api-key".to_string());

let response = ingest_from_s3_path_async(
    &request,
    &upload_storage,
    &progress_tracker,
    node,
    None  // Config is in the request
).await?;
```

### Backwards Compatible

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest, IngestionConfig};

let ingestion_config = IngestionConfig::from_env()?;
let request = S3IngestionRequest::new("s3://bucket/file.json".to_string());

let response = ingest_from_s3_path_async(
    &request,
    &upload_storage,
    &progress_tracker,
    node,
    Some(&ingestion_config)  // Pass config explicitly
).await?;
```

## Breaking Changes

**None!** The API is fully backwards compatible:
- Old code using `&IngestionConfig` will work by wrapping it in `Some()`
- New code can pass `None` and configure via the request builder
- Environment variable approach still works via `IngestionConfig::from_env()`

## Benefits

1. **Flexibility** - API keys can be passed programmatically
2. **Lambda-friendly** - Perfect for AWS Lambda functions
3. **Multi-tenant Support** - Different API keys per request
4. **Security** - No need to expose environment variables
5. **Backwards Compatible** - Existing code continues to work

## Testing

All tests pass:
- ✅ Unit tests: `cargo test --lib ingestion::s3_ingestion` - 5 tests passed
- ✅ Integration tests: `cargo test --lib ingestion` - 46 tests passed  
- ✅ Full test suite: `cargo test` - All tests passed
- ✅ Linting: `cargo clippy` - No warnings

## Files Modified

**Source Code:**
- `src/ingestion/s3_ingestion.rs`

**Documentation:**
- `docs/S3_INGESTION_API_KEY_CONFIG.md` (NEW)
- `docs/S3_FILE_PATH_INGESTION.md`
- `docs/S3_INGESTION_QUICK_START.md`
- `docs/S3_INGESTION_API_GUIDE.md`

**Examples:**
- `examples/lambda_s3_ingestion.rs`
- `examples/simple_s3_ingestion.rs`

**Changelog:**
- `CHANGELOG_API_KEY_CONFIG.md` (THIS FILE)

## Migration Path

For users of the public API:

1. **No immediate action required** - existing code will continue to work
2. **Recommended update** - start using the new builder pattern for cleaner code
3. **Lambda users** - switch to passing API key directly in request for better secret management

## Future Considerations

- Consider adding similar builder methods for Ollama configuration
- Could extend this pattern to other ingestion endpoints
- May want to add validation at request build time

