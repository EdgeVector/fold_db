//! Feature-specific logging macros and utilities
//!
//! This module provides convenient macros for logging in specific features/components
//! of the fold_db system, allowing easy filtering and debugging.

/// Feature categories for logging
#[derive(Debug, Clone)]
pub enum LogFeature {
    Transform,
    Network,
    Database,
    Schema,
    Query,
    Mutation,
    Permissions,
    HttpServer,
    TcpServer,
    Ingestion,
}

impl LogFeature {
    /// Get the target string for this feature
    pub fn target(&self) -> &'static str {
        match self {
            LogFeature::Transform => "fold_node::transform",
            LogFeature::Network => "fold_node::network",
            LogFeature::Database => "fold_node::database",
            LogFeature::Schema => "fold_node::schema",
            LogFeature::Query => "fold_node::query",
            LogFeature::Mutation => "fold_node::mutation",
            LogFeature::Permissions => "fold_node::permissions",
            LogFeature::HttpServer => "fold_node::http_server",
            LogFeature::TcpServer => "fold_node::tcp_server",
            LogFeature::Ingestion => "fold_node::ingestion",
        }
    }
}

/// Generic logging macro for all features
#[macro_export]
macro_rules! log_feature {
    ($feature:expr, $level:ident, $($arg:tt)*) => {
        log::$level!(target: $feature.target(), $($arg)*)
    };
}

pub use crate::log_feature;

// Performance monitoring helper
pub struct PerformanceTimer {
    start: std::time::Instant,
    feature: LogFeature,
    operation: String,
}

impl PerformanceTimer {
    pub fn new(feature: LogFeature, operation: String) -> Self {
        log::debug!(target: feature.target(), "Starting timed operation: {}", operation);
        Self {
            start: std::time::Instant::now(),
            feature,
            operation,
        }
    }

    pub fn finish(self) {
        let duration = self.start.elapsed();
        log::info!(
            target: self.feature.target(),
            "Operation '{}' completed in {:?}",
            self.operation,
            duration
        );
    }
}
