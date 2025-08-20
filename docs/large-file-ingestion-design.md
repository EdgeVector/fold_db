# Large File Ingestion System Design

## Overview

This document outlines the design for enhancing the DataFold ingestion system to handle large files efficiently. The current ingestion system is designed for processing individual JSON objects or small arrays, but lacks the capability to process large files that may contain millions of records or gigabytes of data.

## Current System Analysis

### Existing Architecture
The current ingestion system consists of:
- **IngestionCore**: Main orchestrator that processes JSON data
- **OpenRouterService**: AI-powered schema analysis and recommendation
- **SchemaStripper**: Removes payment/permission data for AI analysis
- **MutationGenerator**: Creates mutations from JSON data
- **SimpleIngestionService**: Simplified service working with DataFoldNode

### Current Limitations
1. **Memory Constraints**: Loads entire JSON into memory
2. **Single Transaction**: Processes all data in one operation
3. **No Streaming**: Cannot handle files larger than available memory
4. **No Progress Tracking**: No visibility into long-running operations
5. **No Resume Capability**: Cannot resume interrupted operations
6. **No Batch Processing**: Processes all records at once

## Design Goals

### Primary Objectives
1. **Handle Files of Any Size**: Process files from KB to TB without memory constraints
2. **Maintain Performance**: Efficient processing regardless of file size
3. **Provide Progress Visibility**: Real-time progress tracking and status updates
4. **Enable Resume Capability**: Resume interrupted operations from last checkpoint
5. **Support Multiple Formats**: JSON, CSV, NDJSON, and other structured formats
6. **Batch Processing**: Process data in configurable batch sizes

### Secondary Objectives
1. **Resource Management**: Efficient memory and CPU utilization
2. **Error Handling**: Graceful handling of malformed data and system failures
3. **Monitoring**: Comprehensive logging and metrics collection
4. **Scalability**: Support for distributed processing in the future

## Architecture Design

### High-Level Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   File Input    │───▶│  Stream Parser   │───▶│  Batch Buffer   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │                        │
                                ▼                        ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │  Schema Analyzer │    │  Mutation Gen   │
                       └──────────────────┘    └─────────────────┘
                                │                        │
                                ▼                        ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │   AI Service     │    │  Batch Writer   │
                       └──────────────────┘    └─────────────────┘
                                │                        │
                                ▼                        ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │  Schema Manager  │    │   Database      │
                       └──────────────────┘    └─────────────────┘
```

### Core Components

#### 1. Stream Parser
- **Purpose**: Parse large files without loading entire content into memory
- **Implementation**: Use streaming parsers (serde_json::StreamDeserializer, csv::Reader)
- **Features**: 
  - Memory-efficient parsing
  - Support for multiple file formats
  - Error recovery for malformed data

#### 2. Batch Buffer
- **Purpose**: Accumulate records into processable batches
- **Implementation**: Configurable batch size with memory limits
- **Features**:
  - Configurable batch sizes (default: 1000 records)
  - Memory usage monitoring
  - Automatic batch flushing

#### 3. Schema Analyzer
- **Purpose**: Analyze data structure and determine schema compatibility
- **Implementation**: Extend existing AI service for batch analysis
- **Features**:
  - Batch schema analysis
  - Schema evolution detection
  - Conflict resolution

#### 4. Batch Writer
- **Purpose**: Write batches to database efficiently
- **Implementation**: Use database batch operations and transactions
- **Features**:
  - Batch database operations
  - Transaction management
  - Rollback on failure

#### 5. Progress Tracker
- **Purpose**: Track processing progress and provide status updates
- **Implementation**: Persistent progress tracking with checkpoints
- **Features**:
  - Real-time progress updates
  - Checkpoint persistence
  - Resume capability

## Implementation Details

### New Data Structures

#### LargeFileIngestionRequest
```rust
#[derive(Debug, serde::Deserialize)]
pub struct LargeFileIngestionRequest {
    /// File path or URL to process
    pub file_path: String,
    /// File format (json, csv, ndjson, etc.)
    pub file_format: FileFormat,
    /// Batch size for processing
    pub batch_size: Option<usize>,
    /// Whether to auto-execute mutations
    pub auto_execute: Option<bool>,
    /// Trust distance for mutations
    pub trust_distance: Option<u32>,
    /// Public key for mutations
    pub pub_key: Option<String>,
    /// Resume from checkpoint if available
    pub resume_from_checkpoint: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub enum FileFormat {
    Json,
    Csv,
    Ndjson,
    Parquet,
    Avro,
}
```

#### IngestionProgress
```rust
#[derive(Debug, serde::Serialize)]
pub struct IngestionProgress {
    /// Unique job identifier
    pub job_id: String,
    /// Current status
    pub status: IngestionStatus,
    /// Total records to process
    pub total_records: Option<usize>,
    /// Records processed so far
    pub records_processed: usize,
    /// Current batch being processed
    pub current_batch: usize,
    /// Total batches
    pub total_batches: Option<usize>,
    /// Processing start time
    pub start_time: DateTime<Utc>,
    /// Estimated completion time
    pub estimated_completion: Option<DateTime<Utc>>,
    /// Any errors encountered
    pub errors: Vec<String>,
    /// Checkpoint for resume capability
    pub checkpoint: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub enum IngestionStatus {
    Pending,
    Processing,
    Paused,
    Completed,
    Failed,
    Resuming,
}
```

#### BatchProcessor
```rust
pub struct BatchProcessor {
    config: BatchProcessorConfig,
    schema_analyzer: Arc<SchemaAnalyzer>,
    mutation_generator: Arc<MutationGenerator>,
    progress_tracker: Arc<ProgressTracker>,
}

impl BatchProcessor {
    /// Process a batch of records
    pub async fn process_batch(
        &self,
        batch: Vec<serde_json::Value>,
        batch_number: usize,
    ) -> Result<BatchResult, IngestionError> {
        // Analyze batch structure
        let schema_analysis = self.schema_analyzer.analyze_batch(&batch).await?;
        
        // Generate mutations for batch
        let mutations = self.mutation_generator.generate_batch_mutations(
            &batch,
            &schema_analysis,
        )?;
        
        // Execute mutations if auto-execute is enabled
        let executed_count = if self.config.auto_execute {
            self.execute_batch_mutations(&mutations).await?
        } else {
            0
        };
        
        Ok(BatchResult {
            batch_number,
            records_processed: batch.len(),
            mutations_generated: mutations.len(),
            mutations_executed: executed_count,
            schema_changes: schema_analysis.changes,
        })
    }
}
```

### Configuration

#### LargeFileIngestionConfig
```rust
#[derive(Debug, Clone)]
pub struct LargeFileIngestionConfig {
    /// Default batch size
    pub default_batch_size: usize,
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Memory limit per batch (in bytes)
    pub max_batch_memory: usize,
    /// Maximum concurrent batch processors
    pub max_concurrent_batches: usize,
    /// Checkpoint interval (batches)
    pub checkpoint_interval: usize,
    /// Progress update interval (milliseconds)
    pub progress_update_interval: Duration,
    /// Temporary file directory
    pub temp_dir: PathBuf,
    /// Whether to enable resume capability
    pub enable_resume: bool,
}

impl Default for LargeFileIngestionConfig {
    fn default() -> Self {
        Self {
            default_batch_size: 1000,
            max_batch_size: 10000,
            max_batch_memory: 100 * 1024 * 1024, // 100MB
            max_concurrent_batches: 4,
            checkpoint_interval: 10,
            progress_update_interval: Duration::from_millis(1000),
            temp_dir: std::env::temp_dir().join("datafold_ingestion"),
            enable_resume: true,
        }
    }
}
```

### API Endpoints

#### Start Large File Ingestion
```rust
/// POST /api/ingestion/large-file/start
pub async fn start_large_file_ingestion(
    request: web::Json<LargeFileIngestionRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    // Validate file exists and is accessible
    // Start background processing job
    // Return job ID for progress tracking
}
```

#### Get Ingestion Progress
```rust
/// GET /api/ingestion/large-file/{job_id}/progress
pub async fn get_ingestion_progress(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    // Return current progress for job
}
```

#### Pause/Resume Ingestion
```rust
/// POST /api/ingestion/large-file/{job_id}/pause
/// POST /api/ingestion/large-file/{job_id}/resume
/// POST /api/ingestion/large-file/{job_id}/cancel
```

## Processing Workflow

### 1. File Validation and Preparation
1. Validate file exists and is accessible
2. Determine file size and format
3. Create temporary working directory
4. Initialize progress tracking

### 2. Schema Analysis Phase
1. Read first batch to analyze structure
2. Send to AI service for initial schema recommendation
3. Create or identify existing schema
4. Validate schema against subsequent batches

### 3. Batch Processing Phase
1. Read file in configurable batch sizes
2. Parse records according to file format
3. Validate data against schema
4. Generate mutations for batch
5. Execute mutations (if auto-execute enabled)
6. Update progress and create checkpoints

### 4. Completion and Cleanup
1. Finalize all batches
2. Update schema if changes detected
3. Generate final report
4. Clean up temporary files
5. Update progress status

## Error Handling and Recovery

### Error Categories
1. **File Errors**: Corrupted files, permission issues
2. **Schema Errors**: Incompatible data structures
3. **Database Errors**: Connection issues, constraint violations
4. **System Errors**: Memory issues, disk space

### Recovery Strategies
1. **Batch-Level Recovery**: Skip failed batches, continue with others
2. **Checkpoint Recovery**: Resume from last successful checkpoint
3. **Schema Evolution**: Handle schema changes during processing
4. **Partial Success**: Report partial completion with error details

## Performance Considerations

### Memory Management
- **Streaming Parsing**: Never load entire file into memory
- **Batch Limits**: Configurable batch sizes with memory monitoring
- **Garbage Collection**: Explicit cleanup between batches
- **Memory Pooling**: Reuse memory buffers where possible

### CPU Optimization
- **Parallel Processing**: Process multiple batches concurrently
- **Async I/O**: Non-blocking file and database operations
- **Batch Operations**: Use database batch operations
- **Efficient Parsing**: Optimized parsers for each format

### I/O Optimization
- **Buffered Reading**: Efficient file reading with appropriate buffer sizes
- **Batch Database Operations**: Minimize database round trips
- **Checkpoint Optimization**: Efficient checkpoint persistence
- **Temporary File Management**: Use fast storage for temporary files

## Monitoring and Observability

### Metrics Collection
- **Processing Rate**: Records per second
- **Memory Usage**: Peak and average memory consumption
- **Batch Performance**: Average batch processing time
- **Error Rates**: Errors per batch, total errors
- **Schema Changes**: Number and type of schema modifications

### Logging
- **Structured Logging**: JSON-formatted logs with correlation IDs
- **Progress Logging**: Regular progress updates
- **Error Logging**: Detailed error information with context
- **Performance Logging**: Timing information for optimization

### Health Checks
- **Resource Monitoring**: Memory, CPU, disk usage
- **Queue Monitoring**: Batch processing queue status
- **Database Health**: Connection and performance status
- **File System Health**: Temporary directory and disk space

## Security Considerations

### File Access Control
- **Path Validation**: Prevent directory traversal attacks
- **File Size Limits**: Configurable maximum file sizes
- **Format Validation**: Strict format validation to prevent injection attacks
- **Temporary File Security**: Secure temporary file creation and cleanup

### Data Validation
- **Input Sanitization**: Validate all input data before processing
- **Schema Validation**: Strict schema compliance checking
- **Size Limits**: Prevent oversized records or fields
- **Content Validation**: Validate data content, not just structure

## Testing Strategy

### Unit Tests
- **Parser Tests**: Test each file format parser independently
- **Batch Processor Tests**: Test batch processing logic
- **Schema Analysis Tests**: Test schema analysis and evolution
- **Error Handling Tests**: Test various error scenarios

### Integration Tests
- **End-to-End Tests**: Test complete ingestion workflows
- **Database Integration**: Test database operations and transactions
- **File System Integration**: Test file handling and cleanup
- **API Integration**: Test all API endpoints

### Performance Tests
- **Large File Tests**: Test with files of various sizes
- **Memory Usage Tests**: Verify memory usage stays within limits
- **Concurrency Tests**: Test multiple concurrent ingestion jobs
- **Stress Tests**: Test system behavior under load

## Migration and Compatibility

### Backward Compatibility
- **Existing API**: Maintain all existing ingestion endpoints
- **Configuration**: Support existing configuration options
- **Schema Compatibility**: Maintain existing schema management
- **Data Format**: Support existing data formats

### Gradual Rollout
- **Feature Flags**: Enable/disable large file ingestion
- **A/B Testing**: Compare performance with existing system
- **Rollback Plan**: Ability to revert to existing system
- **Monitoring**: Comprehensive monitoring during rollout

## Future Enhancements

### Distributed Processing
- **Worker Nodes**: Distribute processing across multiple nodes
- **Load Balancing**: Balance load across available workers
- **Fault Tolerance**: Handle worker node failures gracefully
- **Scalability**: Scale horizontally based on load

### Advanced Formats
- **Compressed Files**: Support for gzip, bzip2, etc.
- **Database Dumps**: Direct import from database exports
- **Streaming Sources**: Real-time data ingestion
- **Cloud Storage**: Direct integration with S3, GCS, etc.

### Machine Learning Integration
- **Auto-Schema Detection**: Improved schema detection algorithms
- **Data Quality Assessment**: Automatic data quality scoring
- **Anomaly Detection**: Identify and flag data anomalies
- **Predictive Processing**: Optimize processing based on data patterns

## Conclusion

This large file ingestion system design addresses the current limitations of the DataFold ingestion system while maintaining compatibility with existing functionality. The streaming architecture ensures efficient memory usage, while the batch processing approach provides performance and reliability. The comprehensive error handling and progress tracking enable robust operation with large datasets.

The design is modular and extensible, allowing for future enhancements such as distributed processing and additional file format support. The implementation prioritizes performance, reliability, and user experience while maintaining the security and data integrity standards of the existing system.
