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

/// Generic logging macro for all features.
///
/// Expands to a `match` on the `LogFeature` so each arm hands `tracing::$level!`
/// a `&'static str` target literal — `tracing` stores `target` in a static
/// `Metadata` callsite, so it cannot accept a runtime method call like
/// `feature.target()`. The match keeps targets in lockstep with
/// `LogFeature::target()` above; both must be updated together.
#[macro_export]
macro_rules! log_feature {
    ($feature:expr, $level:ident, $($arg:tt)*) => {{
        match $feature {
            $crate::logging::features::LogFeature::Transform => {
                tracing::$level!(target: "fold_node::transform", $($arg)*)
            }
            $crate::logging::features::LogFeature::Network => {
                tracing::$level!(target: "fold_node::network", $($arg)*)
            }
            $crate::logging::features::LogFeature::Database => {
                tracing::$level!(target: "fold_node::database", $($arg)*)
            }
            $crate::logging::features::LogFeature::Schema => {
                tracing::$level!(target: "fold_node::schema", $($arg)*)
            }
            $crate::logging::features::LogFeature::Query => {
                tracing::$level!(target: "fold_node::query", $($arg)*)
            }
            $crate::logging::features::LogFeature::Mutation => {
                tracing::$level!(target: "fold_node::mutation", $($arg)*)
            }
            $crate::logging::features::LogFeature::Permissions => {
                tracing::$level!(target: "fold_node::permissions", $($arg)*)
            }
            $crate::logging::features::LogFeature::HttpServer => {
                tracing::$level!(target: "fold_node::http_server", $($arg)*)
            }
            $crate::logging::features::LogFeature::TcpServer => {
                tracing::$level!(target: "fold_node::tcp_server", $($arg)*)
            }
            $crate::logging::features::LogFeature::Ingestion => {
                tracing::$level!(target: "fold_node::ingestion", $($arg)*)
            }
        }
    }};
}

pub use crate::log_feature;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
    use tracing_subscriber::registry::{LookupSpan, Registry};

    #[derive(Default)]
    struct MessageVisitor {
        message: String,
    }

    impl Visit for MessageVisitor {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.message = format!("{:?}", value);
            }
        }
    }

    #[derive(Clone, Default)]
    struct CaptureLayer {
        captured: Arc<Mutex<Vec<(String, String, String)>>>,
    }

    impl<S> Layer<S> for CaptureLayer
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let meta = event.metadata();
            let mut visitor = MessageVisitor::default();
            event.record(&mut visitor);
            self.captured.lock().unwrap().push((
                meta.target().to_string(),
                meta.level().to_string(),
                visitor.message,
            ));
        }
    }

    /// `log_feature!` must route through `tracing::$level!`, not `log::$level!`.
    /// Assert by installing a tracing-only subscriber and confirming the event
    /// arrives with the feature's `target()` and the requested level.
    #[test]
    fn log_feature_routes_through_tracing() {
        let layer = CaptureLayer::default();
        let captured = layer.captured.clone();
        let subscriber = Registry::default().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            log_feature!(LogFeature::Schema, info, "schema event {}", 7);
            log_feature!(LogFeature::HttpServer, warn, "server warn");
        });

        let entries = captured.lock().unwrap();
        assert_eq!(
            entries.len(),
            2,
            "expected two captured events: {entries:?}"
        );

        assert_eq!(entries[0].0, LogFeature::Schema.target());
        assert_eq!(entries[0].1, "INFO");
        assert!(
            entries[0].2.contains("schema event 7"),
            "message body should contain the formatted args, got {:?}",
            entries[0].2,
        );

        assert_eq!(entries[1].0, LogFeature::HttpServer.target());
        assert_eq!(entries[1].1, "WARN");
        assert!(
            entries[1].2.contains("server warn"),
            "message body should contain the literal, got {:?}",
            entries[1].2,
        );
    }
}
