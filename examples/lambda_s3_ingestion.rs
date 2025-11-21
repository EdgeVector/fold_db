//! Example: AWS Lambda S3 Event Ingestion
//!
//! This example demonstrates how to use DataFold's S3 ingestion API
//! in an AWS Lambda function triggered by S3 events.
//!
//! ## Setup
//!
//! 1. Configure environment variables:
//! ```bash
//! export DATAFOLD_STORAGE_MODE=s3
//! export DATAFOLD_S3_BUCKET=my-db-bucket
//! export DATAFOLD_S3_REGION=us-west-2
//! export DATAFOLD_UPLOAD_STORAGE_MODE=s3
//! export DATAFOLD_UPLOAD_S3_BUCKET=my-uploads-bucket
//! export DATAFOLD_UPLOAD_S3_REGION=us-west-2
//! # Optional: Can also pass API key directly in code
//! export FOLD_OPENROUTER_API_KEY=your-api-key
//! ```
//!
//! 2. Deploy to Lambda with S3 event trigger
//!
//! ## Lambda Handler
//!
//! ```rust,no_run
//! use datafold::{
//!     ingestion::{ingest_from_s3_path_async, S3IngestionRequest, IngestionConfig},
//!     datafold_node::http_server::AppState,
//! };
//! use lambda_runtime::{run, service_fn, Error, LambdaEvent};
//! use serde_json::{json, Value};
//!
//! async fn function_handler(
//!     event: LambdaEvent<Value>,
//!     state: &AppState,
//! ) -> Result<Value, Error> {
//!     // Parse S3 event
//!     let bucket = event.payload["Records"][0]["s3"]["bucket"]["name"]
//!         .as_str()
//!         .ok_or("Missing bucket name")?;
//!     let key = event.payload["Records"][0]["s3"]["object"]["key"]
//!         .as_str()
//!         .ok_or("Missing object key")?;
//!
//!     let s3_path = format!("s3://{}/{}", bucket, key);
//!     
//!     println!("Processing S3 file: {}", s3_path);
//!
//!     // Create ingestion request
//!     // Option 1: Pass API key directly (recommended for Lambda)
//!     let request = S3IngestionRequest::new(s3_path)
//!         .with_auto_execute(true)
//!         .with_trust_distance(0)
//!         .with_openrouter_api_key(std::env::var("FOLD_OPENROUTER_API_KEY")?);
//!
//!     // Ingest file asynchronously
//!     let response = ingest_from_s3_path_async(&request, &upload_storage, &progress_tracker, node, None).await?;
//!     
//!     // Option 2: Use environment-based config (legacy approach)
//!     // let ingestion_config = IngestionConfig::from_env()?;
//!     // let response = ingest_from_s3_path_async(&request, &upload_storage, &progress_tracker, node, Some(&ingestion_config)).await?;
//!
//!     if response.success {
//!         println!("Ingestion started: {:?}", response.progress_id);
//!         Ok(json!({
//!             "statusCode": 200,
//!             "body": json!({
//!                 "message": "Ingestion started",
//!                 "progress_id": response.progress_id,
//!             }).to_string()
//!         }))
//!     } else {
//!         Ok(json!({
//!             "statusCode": 500,
//!             "body": json!({
//!                 "message": "Ingestion failed",
//!                 "errors": response.errors,
//!             }).to_string()
//!         }))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     // Initialize DataFold
//!     let state = AppState::new(/* ... */);
//!     
//!     // Run Lambda handler
//!     run(service_fn(|event| function_handler(event, &state))).await
//! }
//! ```
//!
//! ## Synchronous Processing
//!
//! For synchronous processing (Lambda waits for ingestion to complete):
//!
//! ```rust,no_run
//! use datafold::ingestion::{ingest_from_s3_path_sync, S3IngestionRequest, IngestionConfig};
//!
//! async fn function_handler_sync(
//!     event: LambdaEvent<Value>,
//!     state: &AppState,
//! ) -> Result<Value, Error> {
//!     let s3_path = /* extract from event */;
//!     
//!     // Pass API key directly in request
//!     let request = S3IngestionRequest::new(s3_path)
//!         .with_openrouter_api_key(std::env::var("FOLD_OPENROUTER_API_KEY")?);
//!     
//!     // Wait for completion
//!     let response = ingest_from_s3_path_sync(&request, &upload_storage, &progress_tracker, node, None).await?;
//!
//!     Ok(json!({
//!         "statusCode": 200,
//!         "body": json!({
//!             "message": "Ingestion complete",
//!             "schema_used": response.schema_used,
//!             "mutations_executed": response.mutations_executed,
//!         }).to_string()
//!     }))
//! }
//! ```

fn main() {
    println!("This is an example file. See the documentation above for Lambda usage.");
}

