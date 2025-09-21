//! # Infrastructure Factory - Consolidated Infrastructure Creation Patterns
//!
//! This module consolidates all the repeated infrastructure creation patterns
//! found throughout the codebase to eliminate massive duplication.

use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::fold_db_core::managers::AtomManager;
use crate::fold_db_core::services::field_retrieval::service::FieldRetrievalService;
use crate::logging::features::{log_feature, LogFeature};
use crate::schema::core::SchemaCore;
use crate::schema::SchemaError;
use std::sync::Arc;

/// Consolidated factory for creating common infrastructure components
pub struct InfrastructureFactory;

impl InfrastructureFactory {
    /// Create a shared MessageBus instance
    /// Consolidates: Arc::new(MessageBus::new()) pattern found 15+ times
    pub fn create_message_bus() -> Arc<MessageBus> {
        Arc::new(MessageBus::new())
    }

    /// Create shared database operations
    /// Consolidates: Arc::new(db_ops) pattern found 10+ times
    pub fn create_db_ops(db: sled::Db) -> Result<Arc<DbOperations>, sled::Error> {
        let db_ops = DbOperations::new(db)?;
        Ok(Arc::new(db_ops))
    }

    /// Create a complete test infrastructure bundle
    /// Consolidates all the test setup duplication across test files
    pub fn create_test_infrastructure() -> Result<TestInfrastructure, SchemaError> {
        let message_bus = Self::create_message_bus();
        let (db_ops, _) = crate::testing_utils::TestDatabaseFactory::create_test_environment()
            .map_err(|e| SchemaError::InvalidData(e.to_string()))?;

        Ok(TestInfrastructure {
            message_bus,
            db_ops,
        })
    }

    /// Create a complete production infrastructure bundle
    pub fn create_production_infrastructure(
        db: sled::Db,
        schema_path: &str,
    ) -> Result<ProductionInfrastructure, SchemaError> {
        let message_bus = Self::create_message_bus();
        let db_ops =
            Self::create_db_ops(db).map_err(|e| SchemaError::InvalidData(e.to_string()))?;

        let atom_manager = AtomManager::new((*db_ops).clone(), Arc::clone(&message_bus));
        let schema_manager = Arc::new(
            SchemaCore::new(schema_path, Arc::clone(&db_ops), Arc::clone(&message_bus))
                .map_err(|e| SchemaError::InvalidData(e.to_string()))?,
        );

        let field_retrieval_service = FieldRetrievalService::new(Arc::clone(&message_bus));

        Ok(ProductionInfrastructure {
            message_bus,
            db_ops,
            atom_manager,
            schema_manager,
            field_retrieval_service,
        })
    }
}

/// Bundle of infrastructure components for testing
pub struct TestInfrastructure {
    pub message_bus: Arc<MessageBus>,
    pub db_ops: Arc<DbOperations>,
}

/// Bundle of infrastructure components for production
pub struct ProductionInfrastructure {
    pub message_bus: Arc<MessageBus>,
    pub db_ops: Arc<DbOperations>,
    pub atom_manager: AtomManager,
    pub schema_manager: Arc<SchemaCore>,
    pub field_retrieval_service: FieldRetrievalService,
}

/// Consolidated logging utilities with standard emoji patterns
/// Consolidates the repeated emoji logging patterns: 🔧, ✅, ❌, 🎯, 🔍, 🔄
pub struct InfrastructureLogger;

impl InfrastructureLogger {
    /// Log operation start - replaces 🔧 pattern
    pub fn log_operation_start(component: &str, operation: &str, details: &str) {
        log_feature!(
            LogFeature::Database,
            info,
            "🔧 {}: {} - {}",
            component,
            operation,
            details
        );
    }

    /// Log operation success - replaces ✅ pattern  
    pub fn log_operation_success(component: &str, operation: &str, details: &str) {
        log_feature!(
            LogFeature::Database,
            info,
            "✅ {}: {} - {}",
            component,
            operation,
            details
        );
    }

    /// Log operation failure - replaces ❌ pattern
    pub fn log_operation_error(component: &str, operation: &str, error: &str) {
        log_feature!(
            LogFeature::Database,
            error,
            "❌ {}: {} - {}",
            component,
            operation,
            error
        );
    }

    /// Log debug information - replaces 🎯 pattern
    pub fn log_debug_info(component: &str, info: &str) {
        log_feature!(
            LogFeature::Database,
            info,
            "🎯 DEBUG {}: {}",
            component,
            info
        );
    }

    /// Log investigation/search - replaces 🔍 pattern
    pub fn log_investigation(component: &str, info: &str) {
        log_feature!(LogFeature::Database, info, "🔍 {}: {}", component, info);
    }

    /// Log processing/execution - replaces 🔄 pattern
    pub fn log_processing(component: &str, info: &str) {
        log_feature!(LogFeature::Database, info, "🔄 {}: {}", component, info);
    }

    /// Log warning with standard pattern
    pub fn log_warning(component: &str, warning: &str) {
        log_feature!(LogFeature::Database, warn, "⚠️ {}: {}", component, warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_message_bus() {
        let bus = InfrastructureFactory::create_message_bus();
        assert!(!Arc::ptr_eq(
            &bus,
            &InfrastructureFactory::create_message_bus()
        ));
    }

    #[test]
    fn test_create_test_infrastructure() {
        let infra = InfrastructureFactory::create_test_infrastructure();
        assert!(infra.is_ok());
    }

    #[test]
    fn test_logging_utilities() {
        // Test that logging doesn't panic
        InfrastructureLogger::log_operation_start("Test", "operation", "details");
        InfrastructureLogger::log_operation_success("Test", "operation", "details");
        InfrastructureLogger::log_operation_error("Test", "operation", "error");
        InfrastructureLogger::log_debug_info("Test", "info");
        InfrastructureLogger::log_investigation("Test", "info");
        InfrastructureLogger::log_processing("Test", "info");
        InfrastructureLogger::log_warning("Test", "warning");
    }
}
