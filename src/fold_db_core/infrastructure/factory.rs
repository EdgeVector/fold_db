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
use std::sync::Arc;

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