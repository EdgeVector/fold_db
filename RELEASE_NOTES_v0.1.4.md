# Release Notes - DataFold v0.1.4

## 🎉 What's New: S3 File Path Ingestion

Version 0.1.4 introduces powerful S3 file path ingestion capabilities, enabling you to process files already stored in S3 without re-uploading them. Perfect for serverless deployments and AWS Lambda functions!

## ✨ Key Features

### 1. HTTP API Support
Process S3 files via REST API:
```bash
curl -X POST http://localhost:9001/api/ingestion/upload \
  -F "s3FilePath=s3://my-bucket/data.json" \
  -F "autoExecute=true"
```

### 2. Programmatic Rust API (NEW!)
Integrate directly in Lambda or Rust applications:

```rust
use datafold::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};

// Async ingestion (returns immediately with progress_id)
let request = S3IngestionRequest::new("s3://bucket/file.json".to_string());
let response = ingest_from_s3_path_async(&request, &state).await?;

// Or sync ingestion (waits for completion)
use datafold::ingestion::ingest_from_s3_path_sync;
let response = ingest_from_s3_path_sync(&request, &state).await?;
```

### 3. Web UI Integration
Toggle between file upload and S3 path input directly in the browser.

## 🚀 Use Cases

- **AWS Lambda Functions** - Process S3 events programmatically
- **ETL Pipelines** - Ingest pipeline outputs already in S3
- **Batch Processing** - Process existing S3 files at scale
- **Data Lakes** - Integration with S3-based data lakes

## 📦 What's Included

### New Modules
- `datafold::ingestion::S3IngestionRequest` - Builder API for S3 ingestion
- `datafold::ingestion::ingest_from_s3_path_async` - Async ingestion function
- `datafold::ingestion::ingest_from_s3_path_sync` - Sync ingestion function

### New Examples
- `examples/lambda_s3_ingestion.rs` - Complete AWS Lambda integration
- `examples/simple_s3_ingestion.rs` - Basic usage examples

### Documentation
- `docs/S3_FILE_PATH_INGESTION.md` - Comprehensive guide
- Updated README with S3 ingestion examples
- CHANGELOG.md with version history

## 🔧 Requirements

For S3 file path ingestion:
- S3 storage mode configured (`DATAFOLD_UPLOAD_STORAGE_MODE=s3`)
- AWS credentials with `s3:GetObject` permissions

## 📊 Benefits

- ✅ **No Re-upload Required** - Save bandwidth and time
- ✅ **Lambda-Ready** - Perfect for serverless architectures
- ✅ **Flexible** - HTTP, UI, or programmatic access
- ✅ **Same Pipeline** - Identical processing for all input methods

## 🔄 Migration

No breaking changes! This is a backward-compatible feature addition. All existing code continues to work unchanged.

## 📚 Documentation

- [Main README](README.md) - Updated with S3 examples
- [S3 Ingestion Guide](docs/S3_FILE_PATH_INGESTION.md) - Detailed documentation
- [Lambda Example](examples/lambda_s3_ingestion.rs) - AWS Lambda integration
- [Simple Example](examples/simple_s3_ingestion.rs) - Basic usage

## 🐛 Bug Fixes

None - this is a pure feature release.

## ⚠️ Breaking Changes

None - fully backward compatible.

## 🙏 Acknowledgments

Thanks to all contributors and users providing feedback!

## 📥 Installation

```bash
# Update to the latest version
cargo update datafold

# Or install from scratch
cargo install datafold@0.1.4
```

## 🔗 Links

- **Crate:** https://crates.io/crates/datafold
- **Docs:** https://docs.rs/datafold
- **Repository:** https://github.com/shiba4life/fold_db
- **Issues:** https://github.com/shiba4life/fold_db/issues

---

**Released:** 2024-11-18
**Version:** 0.1.4
**License:** MIT OR Apache-2.0

