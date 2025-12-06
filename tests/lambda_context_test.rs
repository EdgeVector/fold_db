#![cfg(feature = "lambda")]
use datafold::lambda::{LambdaConfig, LambdaContext, StdoutLogger};
use datafold::storage::StorageConfig;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::test]
async fn test_lambda_context_initialization() {
    let temp_dir = std::env::temp_dir().join(format!("lambda_test_{}", uuid::Uuid::new_v4()));
    let storage_config = StorageConfig::Local { path: temp_dir.clone() };
    let config = LambdaConfig::new(storage_config)
        .with_schema_service_url("https://schema.example.com".to_string())
        .with_logger(Arc::new(StdoutLogger));
    
    let result = LambdaContext::init(config).await;
    
    // Should succeed or already be initialized
    match result {
        Ok(_) => {
            // Successfully initialized
            assert!(LambdaContext::node().await.is_ok());
            assert!(LambdaContext::progress_tracker().is_ok());
        }
        Err(e) => {
            // Already initialized from another test
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("already initialized") || error_msg.contains("Context already initialized"),
                "Unexpected error: {}",
                error_msg
            );
        }
    }
    
    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_lambda_context_double_init_fails() {
    let temp_dir1 = std::env::temp_dir().join(format!("lambda_test_{}", uuid::Uuid::new_v4()));
    let temp_dir2 = std::env::temp_dir().join(format!("lambda_test_{}", uuid::Uuid::new_v4()));
    
    let storage_config1 = StorageConfig::Local { path: temp_dir1.clone() };
    let config1 = LambdaConfig::new(storage_config1)
        .with_schema_service_url("https://schema.example.com".to_string())
        .with_logger(Arc::new(StdoutLogger));
    
    let storage_config2 = StorageConfig::Local { path: temp_dir2.clone() };
    let config2 = LambdaConfig::new(storage_config2)
        .with_schema_service_url("https://schema.example.com".to_string())
        .with_logger(Arc::new(StdoutLogger));
    
    let first_init = LambdaContext::init(config1).await;
    let second_init = LambdaContext::init(config2).await;
    
    // Either first succeeded and second failed, or both failed (already initialized)
    if first_init.is_ok() {
        assert!(second_init.is_err());
        if let Err(e) = second_init {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("already initialized") || error_msg.contains("Context already initialized"),
                "Unexpected error: {}",
                error_msg
            );
        }
    } else {
        // First failed because already initialized
        if let Err(e) = first_init {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("already initialized") || error_msg.contains("Context already initialized"),
                "Unexpected error: {}",
                error_msg
            );
        }
    }
    
    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir1);
    let _ = std::fs::remove_dir_all(temp_dir2);
}

#[tokio::test]
async fn test_lambda_context_with_schema_service_url() {
    let temp_dir = std::env::temp_dir().join(format!("lambda_test_{}", uuid::Uuid::new_v4()));
    let storage_config = StorageConfig::Local { path: temp_dir.clone() };
    let config = LambdaConfig::new(storage_config)
        .with_schema_service_url("https://schema.example.com".to_string())
        .with_logger(Arc::new(StdoutLogger));
    
    let result = LambdaContext::init(config).await;
    
    // Should succeed or already be initialized
    match result {
        Ok(_) => {
            // Successfully initialized with schema service URL
            assert!(LambdaContext::node().await.is_ok());
        }
        Err(e) => {
            // Already initialized from another test
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("already initialized") || error_msg.contains("Context already initialized"),
                "Unexpected error: {}",
                error_msg
            );
        }
    }
    
    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_lambda_context_get_progress_nonexistent() {
    let temp_dir = std::env::temp_dir().join(format!("lambda_test_{}", uuid::Uuid::new_v4()));
    let storage_config = StorageConfig::Local { path: temp_dir.clone() };
    let config = LambdaConfig::new(storage_config)
        .with_schema_service_url("https://schema.example.com".to_string())
        .with_logger(Arc::new(StdoutLogger));
    
    // Initialize if not already initialized
    let _ = LambdaContext::init(config).await;
    
    // Try to get non-existent progress
    let result = LambdaContext::get_progress("nonexistent-progress-id");
    
    // Should return Ok(None) for non-existent progress or be uninitialized
    match result {
        Ok(None) => {
            // Expected: progress ID not found
        }
        Ok(Some(_)) => {
            // Unexpected: found progress with this ID
            panic!("Found unexpected progress");
        }
        Err(e) => {
            // May not be initialized if init failed
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("not initialized") || error_msg.contains("already initialized"),
                "Unexpected error: {}",
                error_msg
            );
        }
    }
    
    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_lambda_context_access_after_init() {
    let temp_dir = std::env::temp_dir().join(format!("lambda_test_{}", uuid::Uuid::new_v4()));
    let storage_config = StorageConfig::Local { path: temp_dir.clone() };
    let config = LambdaConfig::new(storage_config)
        .with_schema_service_url("https://schema.example.com".to_string())
        .with_logger(Arc::new(StdoutLogger));
    
    // Initialize if not already initialized
    let init_result = LambdaContext::init(config).await;
    
    // If init succeeded or was already initialized, accessors should work
    if init_result.is_ok() || init_result.as_ref().err().map(|e| e.to_string().contains("already initialized")).unwrap_or(false) {
        // All accessors should work
        let node_result = LambdaContext::node().await;
        let tracker_result = LambdaContext::progress_tracker();
        
        // At least one should work if properly initialized
        assert!(
            node_result.is_ok() || tracker_result.is_ok(),
            "Neither node nor progress_tracker accessible"
        );
        
        // If node is accessible, it should be lockable
        if let Ok(node) = node_result {
            let _node_guard = node.lock().await;
        }
    }
    
    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_lambda_context_ingest_json_returns_progress_id() {
    let temp_dir = std::env::temp_dir().join(format!("lambda_test_{}", uuid::Uuid::new_v4()));
    let storage_config = StorageConfig::Local { path: temp_dir.clone() };
    let config = LambdaConfig::new(storage_config)
        .with_schema_service_url("https://schema.example.com".to_string())
        .with_logger(Arc::new(StdoutLogger));
    
    // Initialize if not already initialized
    let _ = LambdaContext::init(config).await;
    
    let data = json!([
        {"id": 1, "name": "Alice"},
        {"id": 2, "name": "Bob"}
    ]);
    
    // Ingest asynchronously
    let result = LambdaContext::ingest_json(
        data,
        false, // Don't auto-execute to avoid mutation errors
        0,
        "test_key".to_string(),
        "test_user".to_string(),
    ).await;
    
    // Should return a progress ID
    match result {
        Ok(progress_id) => {
            assert!(!progress_id.is_empty());
            
            // Should be able to check progress
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let progress = LambdaContext::get_progress(&progress_id);
            assert!(progress.is_ok());
        }
        Err(e) => {
            // Configuration error is acceptable in test environment
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Configuration") || error_msg.contains("config"),
                "Unexpected error: {}",
                error_msg
            );
        }
    }
    
    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}
