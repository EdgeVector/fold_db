//! # Ingestion Module
//!
//! The ingestion module provides automated data ingestion capabilities for the DataFold system.
//! It takes JSON data, analyzes it against existing schemas using AI, and either maps it to
//! existing schemas or creates new ones as needed.
//!
//! ## Components
//!
//! * `core` - Main ingestion orchestrator
//! * `openrouter_service` - OpenRouter API integration for AI-powered schema analysis
//! * `schema_stripper` - Removes payment and permission data from schemas for AI analysis
//! * `mutation_generator` - Creates mutations from AI responses and JSON data
//! * `error` - Custom error types for ingestion operations
//! * `config` - Configuration structures for ingestion settings
//! * `routes` - HTTP route handlers for ingestion API endpoints
//!
//! ## Architecture
//!
//! The ingestion process follows these steps:
//! 1. Accept JSON input data
//! 2. Retrieve and strip available schemas (remove payment/permissions)
//! 3. Send data and schemas to OpenRouter AI for analysis
//! 4. Process AI response to determine schema usage or creation
//! 5. Create new schema if needed and set to approved
//! 6. Generate mutations to store the JSON data
//! 7. Execute mutations to persist the data

pub mod ai_schema_response;
pub mod config;
pub mod core;
pub mod error;
pub mod mutation_generator;
pub mod ollama_service;
pub mod openrouter_service;
pub mod routes;
pub mod schema_stripper;
pub mod simple_service;

// Public re-exports
pub use ai_schema_response::AISchemaResponse;
pub use config::IngestionConfig;
pub use core::IngestionCore;
pub use error::IngestionError;

/// Result type for ingestion operations
pub type IngestionResult<T> = Result<T, IngestionError>;

/// Response from the ingestion process
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IngestionResponse {
    /// Whether the ingestion was successful
    pub success: bool,
    /// Name of the schema used (existing or newly created)
    pub schema_used: Option<String>,
    /// Whether a new schema was created
    pub new_schema_created: bool,
    /// Number of mutations generated
    pub mutations_generated: usize,
    /// Number of mutations successfully executed
    pub mutations_executed: usize,
    /// Any errors that occurred during processing
    pub errors: Vec<String>,
}

impl IngestionResponse {
    /// Create a successful ingestion response
    pub fn success(
        schema_used: String,
        new_schema_created: bool,
        mutations_generated: usize,
        mutations_executed: usize,
    ) -> Self {
        Self {
            success: true,
            schema_used: Some(schema_used),
            new_schema_created,
            mutations_generated,
            mutations_executed,
            errors: Vec::new(),
        }
    }

    /// Create a failed ingestion response
    pub fn failure(errors: Vec<String>) -> Self {
        Self {
            success: false,
            schema_used: None,
            new_schema_created: false,
            mutations_generated: 0,
            mutations_executed: 0,
            errors,
        }
    }

    /// Add an error to the response
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.success = false;
    }
}