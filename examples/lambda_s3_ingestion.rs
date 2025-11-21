//! AWS Lambda Basic Example
//!
//! This example demonstrates how to use DataFold's Lambda context API
//! in an AWS Lambda function.
//!
//! ## Setup
//!
//! 1. Add dependencies to your Lambda project's Cargo.toml:
//! ```toml
//! [dependencies]
//! datafold = { version = "0.1", features = ["lambda"] }
//! lambda_runtime = "0.13"
//! tokio = { version = "1", features = ["macros"] }
//! serde = { version = "1", features = ["derive"] }
//! serde_json = "1"
//! tracing = "0.1"
//! tracing-subscriber = { version = "0.3", features = ["env-filter"] }
//! ```
//!
//! 2. Configure your Lambda function with any required environment variables
//!    based on your use case.
//!
//! 3. Deploy to Lambda with your trigger (S3, API Gateway, etc.)
//!
//! 4. Set Lambda timeout based on your operations (typically 30s-5min).
//!
//! ## Build for Lambda
//!
//! ```bash
//! # For x86_64 architecture
//! cargo build --release --target x86_64-unknown-linux-gnu --features lambda
//!
//! # For ARM64 architecture (Graviton)
//! cargo build --release --target aarch64-unknown-linux-gnu --features lambda
//! ```

#[cfg(feature = "lambda")]
use datafold::lambda::{LambdaConfig, LambdaContext};
#[cfg(feature = "lambda")]
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
#[cfg(feature = "lambda")]
use serde_json::{json, Value};

/// Lambda handler function
#[cfg(feature = "lambda")]
async fn function_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    tracing::info!("Received event: {:?}", event.payload);

    // Access the DataFold node from the context
    let node = LambdaContext::node()?;
    
    // Example: Get node information
    let node_id = {
        let node_guard = node.lock().await;
        node_guard.get_node_id().to_string()
    };

    tracing::info!("Processing with node: {}", node_id);

    // Your Lambda logic here...
    // For ingestion operations, use the node and progress_tracker
    // For custom operations, you have full access to the DataFold node

    Ok(json!({
        "statusCode": 200,
        "body": {
            "message": "Processed successfully",
            "node_id": node_id
        }
    }))
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

    tracing::info!("Initializing Lambda context...");

    // Create Lambda configuration
    let mut config = LambdaConfig::new();

    // Optionally set schema service URL from environment
    if let Ok(schema_url) = std::env::var("SCHEMA_SERVICE_URL") {
        if !schema_url.is_empty() {
            config = config.with_schema_service_url(schema_url);
        }
    }

    // Initialize Lambda context once (reused across invocations)
    LambdaContext::init(config)
        .await
        .map_err(|e| format!("Failed to initialize Lambda context: {}", e))?;

    tracing::info!("Lambda initialized successfully");

    // Run Lambda runtime
    run(service_fn(function_handler)).await
}

#[cfg(not(feature = "lambda"))]
fn main() {
    println!("This example requires the 'lambda' feature flag.");
    println!("Run with: cargo run --example lambda_s3_ingestion --features lambda");
    println!();
    println!("Note: This is an example for AWS Lambda deployment.");
    println!("See the documentation in the file for setup instructions.");
}
