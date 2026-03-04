//! Shared Handler Layer
//!
//! This module provides framework-agnostic business logic handlers that can be used by:
//! - HTTP Server (Actix-web routes)
//! - Lambda handlers (AWS Lambda)
//! - Any other transport layer
//!
//! # Architecture
//!
//! The handlers in this module are pure Rust functions that:
//! - Take typed request objects (not HTTP/Lambda specific)
//! - Return typed response objects wrapped in Result
//! - Are completely framework-agnostic
//!
//! The HTTP server and Lambda adapters are responsible for:
//! - Extracting request data from their specific formats
//! - Calling these shared handlers
//! - Wrapping responses in their specific formats (HttpResponse, json!(), etc.)
//!
//! # Response Envelope
//!
//! All handlers return responses that can be serialized to a standard envelope:
//! ```json
//! {
//!   "ok": true,
//!   "data": { ... },  // flattened into response
//!   "user_hash": "optional_user_context"
//! }
//! ```

pub mod ingestion;
pub mod llm;
pub mod llm_hydration;
pub mod llm_types;
pub mod logs;
pub mod mutation;
pub mod query;
pub mod response;
pub mod schema;
pub mod system;

// Re-export commonly used types
pub use response::{get_db_guard, ApiResponse, HandlerError, HandlerResult, IntoHandlerError};
