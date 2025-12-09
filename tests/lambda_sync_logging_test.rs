#![cfg(feature = "lambda")]
use datafold::lambda::{LambdaConfig, LambdaContext, Logger, LogEntry, LogLevel, LambdaLogging};
use datafold::ingestion::IngestionError;
use datafold::StorageConfig;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

/// Test logger that monitors user_id in logs
#[derive(Clone)]
struct UserIdTestLogger {
    logs: Arc<Mutex<Vec<LogEntry>>>,
}

impl UserIdTestLogger {
    fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn get_logs(&self) -> Vec<LogEntry> {
        self.logs.lock().await.clone()
    }
    
    async fn get_logs_for_user(&self, user_id: &str) -> Vec<LogEntry> {
        let logs = self.logs.lock().await;
        logs.iter()
            .filter(|entry| entry.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect()
    }
}

#[async_trait]
impl Logger for UserIdTestLogger {
    async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut logs = self.logs.lock().await;
        logs.push(entry.clone());
        Ok(())
    }
}

#[tokio::test]
async fn test_sync_ingestion_logging_context() {
    // Create unique temp directory
    let temp_dir = std::env::temp_dir().join(format!("lambda_sync_logging_test_{}", uuid::Uuid::new_v4()));
    
    // Create test logger
    let test_logger = UserIdTestLogger::new();
    
    // Initialize Lambda context
    let storage_config = StorageConfig::Local { path: temp_dir.clone() };
    let config = LambdaConfig::new(storage_config, LambdaLogging::Custom(Arc::new(test_logger.clone())))
        .with_schema_service_url("https://schema.example.com".to_string());
    
    // Initialize context
    let _ = LambdaContext::init(config).await; 
    // Ignore error if already initialized (could happen if running in same process, though unlikely for integration tests)

    let test_data = json!([{"id": "1", "name": "Test Item"}]);
    let target_user_id = "target_tenant_user_123";

    println!("Starting synchronous ingestion for user: {}", target_user_id);

    // Call ingest_json_sync
    // We expect this to fail or succeed, but crucially we want to check logs emitted during execution
    let _ = LambdaContext::ingest_json_sync(
        test_data,
        false,
        0,
        "default".to_string(),
        target_user_id.to_string(),
    ).await;

    // Check logs
    let logs = test_logger.get_logs().await;
    println!("Total logs: {}", logs.len());
    
    // We expect at least some logs to be associated with target_user_id
    // SimpleIngestionService logs at various points
    let user_logs = test_logger.get_logs_for_user(target_user_id).await;
    println!("Logs for {}: {}", target_user_id, user_logs.len());

    assert!(
        logs.len() > 0,
        "Should have emitted logs"
    );

    // The CRITICAL assertion: did the logs pick up the user_id from the context?
    // If run_with_user was NOT working, these logs would have user_id = None
    // (Note: LogBridge reads the task-local variable)
    assert!(
        user_logs.len() > 0,
        "Failed to capture ANY logs with user_id={}. Logs emitted but missing user context!", target_user_id
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}
