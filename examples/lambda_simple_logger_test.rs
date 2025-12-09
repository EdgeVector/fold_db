//! Simple Lambda Logger Test Example
//!
//! This example demonstrates how to use the built-in LambdaContext::test_logger()
//! method to test your logger implementation from any Lambda function.
//!
//! ## Usage
//!
//! Deploy this Lambda and invoke with:
//! ```json
//! {
//!   "user_id": "test_user_123"
//! }
//! ```
//!
//! ## Build for Lambda
//!
//! ```bash
//! # For x86_64 architecture
//! cargo build --release --target x86_64-unknown-linux-gnu --features lambda --example lambda_simple_logger_test
//!
//! # For ARM64 architecture (Graviton)
//! cargo build --release --target aarch64-unknown-linux-gnu --features lambda --example lambda_simple_logger_test
//! ```

#[cfg(feature = "lambda")]
use datafold::lambda::{LambdaConfig, LambdaContext, LambdaLogging, StdoutLogger};
#[cfg(feature = "lambda")]
use datafold::storage::StorageConfig;
#[cfg(feature = "lambda")]
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
#[cfg(feature = "lambda")]
use serde_json::{json, Value};
#[cfg(feature = "lambda")]
use std::sync::Arc;

/// Lambda handler that tests the logger using the built-in test_logger() method
#[cfg(feature = "lambda")]
async fn function_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    tracing::info!("Logger test handler invoked");

    // Extract user_id from event payload
    let user_id = event.payload
        .get("user_id")
        .and_then(|v| v.as_str())
        .unwrap_or("anonymous");

    tracing::info!("Testing logger for user: {}", user_id);

    // Use the built-in test_logger method from LambdaContext
    match LambdaContext::test_logger(user_id).await {
        Ok(result) => {
            tracing::info!("Logger test completed successfully");
            Ok(json!({
                "statusCode": 200,
                "body": result
            }))
        }
        Err(e) => {
            tracing::error!("Logger test failed: {}", e);
            Ok(json!({
                "statusCode": 500,
                "body": {
                    "success": false,
                    "error": e.to_string()
                }
            }))
        }
    }
}

#[cfg(feature = "lambda")]
#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing for CloudWatch logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time() // CloudWatch adds timestamps
        .init();

    tracing::info!("Initializing Lambda Logger Test...");

    // Create Lambda configuration with StdoutLogger
    // In production, replace StdoutLogger with your custom logger (e.g., DynamoDbLogger)
    let storage_config = StorageConfig::Local { 
        path: std::env::temp_dir() 
    };
    let config = LambdaConfig::new(storage_config, LambdaLogging::Stdout);

    // Initialize Lambda context once (reused across invocations)
    LambdaContext::init(config)
        .await
        .map_err(|e| format!("Failed to initialize Lambda context: {}", e))?;

    tracing::info!("Lambda Logger Test initialized successfully");
    tracing::info!("Ready to process logger test requests");

    // Run Lambda runtime
    run(service_fn(function_handler)).await
}

#[cfg(not(feature = "lambda"))]
fn main() {
    println!("This example requires the 'lambda' feature flag.");
    println!("Run with: cargo run --example lambda_simple_logger_test --features lambda");
    println!();
    println!("This is a simple example for AWS Lambda deployment that uses");
    println!("the built-in LambdaContext::test_logger() method.");
    println!();
    println!("Deploy to Lambda and invoke with:");
    println!(r#"  {{ "user_id": "test_user_123" }}"#);
}
