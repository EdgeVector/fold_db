//! Security operations for Lambda context

use crate::ingestion::IngestionError;
use serde_json::Value;

use super::context::LambdaContext;

impl LambdaContext {
    /// Get the security manager's system public key
    ///
    /// Returns the system-level public key for security operations.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     if let Some(key_info) = LambdaContext::get_system_public_key().await? {
    ///         println!("System public key: {:?}", key_info);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_system_public_key() -> Result<Option<Value>, IngestionError> {
        let node_mutex = Self::node().await?;
        let node = node_mutex.lock().await;
        
        // Use OperationProcessor or direct access
        // Since we are in LambdaContext, we can use the node directly or OperationProcessor helper
        // Using direct access for now as implemented in system.rs previously
        let security_manager = node.get_security_manager();
        let key_info = security_manager.get_system_public_key()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get system public key: {}", e)))?;
        
        Ok(key_info.map(|k| serde_json::to_value(k).unwrap_or(serde_json::json!({}))))
    }
}
