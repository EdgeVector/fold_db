//! Data structures and types for the mutation service.
//!
//! This module contains the core data structures used by the mutation service,
//! including normalized field contexts and request wrappers.

use serde_json::{Map, Value};

/// Lightweight normalized context emitted alongside FieldValueSetRequest payloads
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedFieldContext {
    pub hash: Option<String>,
    pub range: Option<String>,
    pub fields: Map<String, Value>,
}

/// Wrapper around the serialized request and reusable normalized context data
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedFieldValueRequest {
    pub request: crate::fold_db_core::infrastructure::message_bus::request_events::FieldValueSetRequest,
    pub context: NormalizedFieldContext,
}

/// Constants used throughout the mutation service
pub const MUTATION_SERVICE_SOURCE: &str = "mutation_service";
