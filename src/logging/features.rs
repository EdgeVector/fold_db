//! Feature-specific logging macros and utilities
//!
//! This module provides convenient macros for logging in specific features/components
//! of the datafold system, allowing easy filtering and debugging.

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
            LogFeature::Transform => "datafold_node::transform",
            LogFeature::Network => "datafold_node::network",
            LogFeature::Database => "datafold_node::database",
            LogFeature::Schema => "datafold_node::schema",
            LogFeature::Query => "datafold_node::query",
            LogFeature::Mutation => "datafold_node::mutation",
            LogFeature::Permissions => "datafold_node::permissions",
            LogFeature::HttpServer => "datafold_node::http_server",
            LogFeature::TcpServer => "datafold_node::tcp_server",
            LogFeature::Ingestion => "datafold_node::ingestion",
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
