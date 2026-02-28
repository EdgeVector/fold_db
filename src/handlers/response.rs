//! Standard API Response Types
//!
//! Provides a unified response envelope that both HTTP server and Lambda use.
//! These types are exported to TypeScript via ts-rs for frontend type safety.

use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Standard API response envelope
///
/// For progress endpoints, the structure is:
/// ```json
/// {
///   "ok": true,
///   "progress": [...],
///   "user_hash": "..."
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    /// Whether the operation succeeded
    pub ok: bool,
    /// The response data (field name varies by endpoint)
    #[serde(flatten)]
    pub data: Option<T>,
    /// Error message (only present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// User hash for context (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_hash: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            user_hash: None,
        }
    }

    /// Create a successful response with user context
    pub fn success_with_user(data: T, user_hash: impl Into<String>) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            user_hash: Some(user_hash.into()),
        }
    }
}

impl ApiResponse<()> {
    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(message.into()),
            user_hash: None,
        }
    }

}

/// Handler-level error types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub enum HandlerError {
    /// Request validation failed
    BadRequest(String),
    /// User not authenticated
    Unauthorized(String),
    /// Resource not found
    NotFound(String),
    /// Internal error
    Internal(String),
    /// Service unavailable
    ServiceUnavailable(String),
}

impl fmt::Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandlerError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            HandlerError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            HandlerError::NotFound(msg) => write!(f, "Not found: {}", msg),
            HandlerError::Internal(msg) => write!(f, "Internal error: {}", msg),
            HandlerError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {}", msg),
        }
    }
}

impl std::error::Error for HandlerError {}

/// Convert FoldDbError into the appropriate HandlerError variant
/// instead of wrapping everything as Internal(500).
impl From<crate::error::FoldDbError> for HandlerError {
    fn from(err: crate::error::FoldDbError) -> Self {
        match &err {
            crate::error::FoldDbError::Schema(schema_err) => match schema_err {
                crate::schema::types::SchemaError::NotFound(_) => {
                    HandlerError::NotFound(err.to_string())
                }
                crate::schema::types::SchemaError::InvalidPermission(_) => {
                    HandlerError::Unauthorized(err.to_string())
                }
                _ => HandlerError::BadRequest(err.to_string()),
            },
            crate::error::FoldDbError::Permission(_) => {
                HandlerError::Unauthorized(err.to_string())
            }
            crate::error::FoldDbError::Config(_) => HandlerError::BadRequest(err.to_string()),
            crate::error::FoldDbError::Serialization(_) => {
                HandlerError::BadRequest(err.to_string())
            }
            _ => HandlerError::Internal(err.to_string()),
        }
    }
}

impl HandlerError {
    /// Convert to HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            HandlerError::BadRequest(_) => 400,
            HandlerError::Unauthorized(_) => 401,
            HandlerError::NotFound(_) => 404,
            HandlerError::Internal(_) => 500,
            HandlerError::ServiceUnavailable(_) => 503,
        }
    }

    /// Convert to ApiResponse
    pub fn to_response(&self) -> ApiResponse<()> {
        ApiResponse::error(self.to_string())
    }

}

/// Result type for handlers
pub type HandlerResult<T> = Result<ApiResponse<T>, HandlerError>;

/// Simple success/failure response used across handlers.
///
/// Defined once here to avoid duplicate definitions in schema, transform, and logs handlers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct SuccessResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Acquire the FoldDB guard from a node, mapping errors to HandlerError::Internal.
///
/// Replaces the repeated pattern:
/// ```ignore
/// node.get_fold_db().await
///     .map_err(|e| HandlerError::Internal(format!("Failed to access database: {}", e)))?
/// ```
pub async fn get_db_guard(
    node: &crate::fold_node::node::FoldNode,
) -> Result<tokio::sync::OwnedMutexGuard<crate::fold_db_core::FoldDB>, HandlerError> {
    node.get_fold_db()
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to access database: {}", e)))
}
