//! Example: Simple S3 File Ingestion
//!
//! This example demonstrates basic usage of the S3 ingestion API
//! for processing files already stored in S3.

use datafold::{
    ingestion::{ingest_from_s3_path_async, ingest_from_s3_path_sync, S3IngestionRequest},
    datafold_node::http_server::AppState,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Note: This is a conceptual example. In practice, you'd initialize
    // AppState with proper configuration.
    
    println!("=== DataFold S3 Ingestion Example ===\n");

    // Example 1: Async ingestion (returns immediately with progress_id)
    println!("Example 1: Async Ingestion");
    example_async_ingestion().await?;

    // Example 2: Sync ingestion (waits for completion)
    println!("\nExample 2: Sync Ingestion");
    example_sync_ingestion().await?;

    // Example 3: Custom settings
    println!("\nExample 3: Custom Settings");
    example_custom_settings().await?;

    Ok(())
}

async fn example_async_ingestion() -> Result<(), Box<dyn std::error::Error>> {
    // In practice, initialize AppState with:
    // let state = AppState::new(...);
    
    let s3_path = "s3://my-bucket/data/tweets.json";
    
    // Pass API key directly in request
    let request = S3IngestionRequest::new(s3_path.to_string())
        .with_openrouter_api_key("your-api-key".to_string());
    
    // This would return immediately with a progress_id
    // let response = ingest_from_s3_path_async(&request, &upload_storage, &progress_tracker, node, None).await?;
    // println!("Started ingestion: {}", response.progress_id.unwrap());
    
    println!("Would ingest: {}", s3_path);
    println!("API key passed directly in request");
    println!("Returns immediately with progress_id for tracking");
    
    Ok(())
}

async fn example_sync_ingestion() -> Result<(), Box<dyn std::error::Error>> {
    let s3_path = "s3://my-bucket/data/users.json";
    
    let request = S3IngestionRequest::new(s3_path.to_string())
        .with_auto_execute(true)
        .with_openrouter_api_key("your-api-key".to_string());
    
    // This would wait for completion
    // let response = ingest_from_s3_path_sync(&request, &upload_storage, &progress_tracker, node, None).await?;
    // println!("Ingestion complete!");
    // println!("Schema: {}", response.schema_used.unwrap());
    // println!("Mutations executed: {}", response.mutations_executed);
    
    println!("Would ingest: {}", s3_path);
    println!("Waits for completion and returns full results");
    
    Ok(())
}

async fn example_custom_settings() -> Result<(), Box<dyn std::error::Error>> {
    let s3_path = "s3://my-bucket/sensitive/data.json";
    
    let request = S3IngestionRequest::new(s3_path.to_string())
        .with_auto_execute(false)  // Don't execute mutations automatically
        .with_trust_distance(5)    // Custom trust distance
        .with_pub_key("my-key".to_string());  // Custom authentication key
    
    println!("Would ingest: {}", s3_path);
    println!("Settings:");
    println!("  - Auto-execute: {}", request.auto_execute);
    println!("  - Trust distance: {}", request.trust_distance);
    println!("  - Public key: {}", request.pub_key);
    
    Ok(())
}

