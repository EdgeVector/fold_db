//! Security utility functions and helpers

use crate::security::{
    ConditionalEncryption, MessageVerifier, PublicKeyInfo, SecurityResult,
};
use std::sync::Arc;

/// Security manager that combines all security functionality
pub struct SecurityManager {
    /// Message verifier for signature verification
    pub verifier: Arc<MessageVerifier>,
    /// Conditional encryption for data at rest
    pub encryption: Arc<ConditionalEncryption>,
    /// Security configuration
    pub config: crate::security::SecurityConfig,
}

impl SecurityManager {
    /// Create a new security manager without persistence
    pub fn new(config: crate::security::SecurityConfig) -> SecurityResult<Self> {
        let verifier = Arc::new(MessageVerifier::new(300)); // 5 minute timestamp drift

        let encryption = Arc::new(ConditionalEncryption::new(
            config.encrypt_at_rest,
            config.master_key,
        )?);

        Ok(Self {
            verifier,
            encryption,
            config,
        })
    }

    /// Create a new security manager with database persistence
    pub async fn new_with_persistence(
        config: crate::security::SecurityConfig,
        db_ops: Arc<crate::db_operations::DbOperations>,
    ) -> SecurityResult<Self> {
        let verifier = Arc::new(MessageVerifier::new_with_persistence(300, db_ops).await?);

        let encryption = Arc::new(ConditionalEncryption::new(
            config.encrypt_at_rest,
            config.master_key,
        )?);

        Ok(Self {
            verifier,
            encryption,
            config,
        })
    }

    /// Get the system public key if it exists.
    pub fn get_system_public_key(&self) -> SecurityResult<Option<PublicKeyInfo>> {
        self.verifier.get_system_public_key()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_manager_creation() {
        let config = crate::security::SecurityConfig {
            require_tls: true,
            encrypt_at_rest: true,
            master_key: Some([0; 32]),
        };
        let manager = SecurityManager::new(config).unwrap();
        assert!(manager.get_system_public_key().unwrap().is_none());
    }
}
