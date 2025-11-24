//! Lambda-optimized API
//!
//! This module provides a simplified interface for AWS Lambda functions
//! that eliminates complex initialization and reduces cold start times.
//!
//! # Quick Start
//!
//! ```ignore
//! use datafold::lambda::{LambdaContext, LambdaConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Initialize once during cold start
//!     let config = LambdaConfig::new();
//!     LambdaContext::init(config).await.expect("Failed to initialize");
//!     
//!     // Access node in handler
//!     let node = LambdaContext::node();
//!     // Use node for operations...
//! }
//! ```

pub mod config;
pub mod context;
pub mod logging;
pub mod types;

// Re-export public API
pub use config::{AIConfig, AIProvider, LambdaConfig, OllamaConfig, OpenRouterConfig};
pub use context::LambdaContext;
pub use logging::{LogBridge, LogEntry, LogLevel, Logger, NoOpLogger, StdoutLogger, UserLogger};
pub use types::{
    AIQueryResponse, CompleteQueryResponse, ConversationMessage, FollowupRequest, 
    FollowupResponse, QueryContext, QueryPlanInfo,
};
