
#![cfg(feature = "lambda")]

use async_trait::async_trait;
use datafold::lambda::{LambdaConfig, LambdaContext, Logger, LogEntry, LogLevel, LambdaLogging};
use std::sync::{Arc, Mutex};
use datafold::storage::DatabaseConfig as StorageConfig;
use std::time::Duration;

/// In-memory mock logger to verify integration
struct MockLogger {
    logs: Arc<Mutex<Vec<LogEntry>>>,
}

impl MockLogger {
    fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn get_logs(&self) -> Vec<LogEntry> {
        self.logs.lock().unwrap().clone()
    }
}

#[async_trait]
impl Logger for MockLogger {
    async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logs.lock().unwrap().push(entry);
        Ok(())
    }
}

#[tokio::test]
async fn test_lambda_logging_integration() {
    let temp_dir = std::env::temp_dir().join(format!("logging_test_{}", uuid::Uuid::new_v4()));
    let storage_config = StorageConfig::Local { path: temp_dir.clone() };
    
    // Create our mock logger
    let mock_logger = Arc::new(MockLogger::new());
    
    // Configure LambdaContext with this logger
    let config = LambdaConfig::new(storage_config, LambdaLogging::Custom(mock_logger.clone()));
        
    // Initialize context
    // This will install the LogBridge which redirects log::* macros to our mock logger
    let _ = LambdaContext::init(config).await;
    
    // Log using the standard Rust log facade (system log - no user_id)
    log::info!(target: "test_verification", "Verifying logging integration");
    
    // Log using request-scoped user logger
    let user_logger = LambdaContext::get_user_logger("test_user_123").expect("Failed to get user logger");
    user_logger.info("user_event", "User specific log").await.expect("Failed to log");
    
    // Log using implicit task-local context (LEAK PROOF)
    datafold::lambda::logging::run_with_user("implicit_user_456", async {
        log::info!(target: "implicit_event", "Implicit user log");
    }).await;
    
    // Logging is async (spawned task), so we need to wait a bit
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify logs were captured by our mock
    let captured_logs = mock_logger.get_logs();
    
    // Debug output
    println!("Captured logs: {:?}", captured_logs);
    
    // Check system log (None user_id)
    let has_system_log = captured_logs.iter().any(|l| 
        l.event_type == "test_verification" && 
        l.message == "Verifying logging integration" &&
        l.user_id.is_none()
    );
    
    // Check user log (Some user_id)
    let has_user_log = captured_logs.iter().any(|l| 
        l.event_type == "user_event" && 
        l.message == "User specific log" &&
        l.user_id.as_deref() == Some("test_user_123")
    );
    
    // Check implicit log (Some user_id)
    let has_implicit_log = captured_logs.iter().any(|l| 
        l.event_type == "implicit_event" && 
        l.message == "Implicit user log" &&
        l.user_id.as_deref() == Some("implicit_user_456")
    );
    
    assert!(has_system_log, "The mock logger should have captured the system log entry");
    assert!(has_user_log, "The mock logger should have captured the user log entry");
    assert!(has_implicit_log, "The mock logger should have captured the implicit log entry");
    
    // Clean up
    let _ = std::fs::remove_dir_all(temp_dir);
}
