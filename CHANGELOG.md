# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.14] - 2025-11-21

### Changed
- **Updated file_to_json dependency**: Upgraded from 0.1.4 to 0.1.5 with improved API
  - Migrated from `Converter::from_env()` to explicit `Converter::new(config)`
  - Better configuration management and error handling
  - Added proper OpenRouter configuration mapping from fold_db settings

### Fixed
- **File conversion configuration**: Fixed OpenRouter API integration by properly mapping fold_db's `FOLD_OPENROUTER_API_KEY` environment variable to file_to_json's expected configuration, resolving 404 errors during file conversion

## [0.1.13] - 2025-11-20

### Fixed
- **S3 Download with Automatic Region Detection**: Fixed S3 downloads when using local storage mode by implementing automatic bucket region detection. The system now queries the bucket location via the S3 API and creates a properly configured client, enabling seamless S3 ingestion regardless of storage configuration.

## [0.1.12] - 2025-11-20

### Fixed
- **S3 Download in Local Mode**: Initial fix for S3 downloads in local storage mode (improved in 0.1.13 with automatic region detection).

## [0.1.4] - 2024-11-18

### Added
- **S3 File Path Ingestion**: Process files already in S3 without re-uploading
  - HTTP API support via `s3FilePath` parameter in `/api/ingestion/upload`
  - Programmatic API with `ingest_from_s3_path_async` and `ingest_from_s3_path_sync` functions
  - UI toggle between file upload and S3 path input modes
  - Full AWS Lambda integration support
- New `S3IngestionRequest` type for programmatic S3 ingestion
- `UploadStorage::download_from_s3_path()` method for downloading from any S3 location
- Lambda and simple usage examples in `examples/` directory
- Comprehensive documentation in `docs/S3_FILE_PATH_INGESTION.md`

### Changed
- Updated file ingestion API to accept either traditional file upload or S3 file path
- Enhanced `IngestionError` with `FileConversionFailed` and `StorageError` variants
- Improved README with S3 ingestion examples and Lambda integration guide

### Fixed
- Better error handling for S3 path validation and download failures

## [0.1.3] - 2024

### Added
- AI-powered data ingestion with automatic schema creation
- Real-time progress tracking for ingestion operations
- File upload with AI-powered conversion (PDF, CSV, JSON, etc.)
- S3 storage support for serverless deployments
- DynamoDB schema store for distributed schema management

### Changed
- Enhanced schema validation and approval workflow
- Improved error handling and logging

## [0.1.2] - 2024

### Added
- Initial public release
- Core database functionality
- Schema-based data storage
- Basic ingestion support

[0.1.4]: https://github.com/shiba4life/fold_db/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/shiba4life/fold_db/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/shiba4life/fold_db/releases/tag/v0.1.2

