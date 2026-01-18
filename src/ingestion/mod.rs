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
//! * `mutation_generator` - Creates mutations from AI responses and JSON data
//! * `error` - Custom error types for ingestion operations
//! * `config` - Configuration structures for ingestion settings
//! * `routes` - HTTP route handlers for ingestion API endpoints
//!
//! ## Architecture
//!
//! The ingestion process follows these steps:
//! 1. Accept JSON input data
//! 2. Retrieve available schemas from schema service
//! 3. Send data and schemas to AI for analysis
//! 4. Process AI response to determine schema usage or creation
//! 5. Create new schema if needed and set to approved
//! 6. Generate mutations to store the JSON data
//! 7. Execute mutations to persist the data

pub mod ai_schema_response;
pub mod config;
pub mod core;
pub mod error;
pub mod file_upload;
pub mod ingestion_spawner;
pub mod json_processor;
pub mod multipart_parser;
pub mod mutation_generator;
pub mod ollama_service;
pub mod openrouter_service;
pub mod progress;
pub mod routes;

pub mod simple_service;
pub mod structure_analyzer;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

// Public re-exports
pub use ai_schema_response::AISchemaResponse;
pub use config::IngestionConfig;
pub use core::IngestionCore;
pub use error::IngestionError;
pub use progress::{
    create_progress_tracker, IngestionProgress, IngestionResults, IngestionStep, ProgressService,
    ProgressTracker,
};
pub use structure_analyzer::StructureAnalyzer;

/// Result type for ingestion operations
pub type IngestionResult<T> = Result<T, IngestionError>;

/// Response from the ingestion process
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IngestionResponse {
    /// Whether the ingestion was successful
    pub success: bool,
    /// Progress ID for tracking the ingestion process
    pub progress_id: Option<String>,
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
            progress_id: None,
            schema_used: Some(schema_used),
            new_schema_created,
            mutations_generated,
            mutations_executed,
            errors: Vec::new(),
        }
    }

    /// Create a successful ingestion response with progress tracking
    pub fn success_with_progress(
        progress_id: String,
        schema_used: String,
        new_schema_created: bool,
        mutations_generated: usize,
        mutations_executed: usize,
    ) -> Self {
        Self {
            success: true,
            progress_id: Some(progress_id),
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
            progress_id: None,
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

/// Status information for the ingestion service
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct IngestionStatus {
    /// Whether ingestion is enabled
    pub enabled: bool,
    /// Whether ingestion is properly configured and ready
    pub configured: bool,
    /// AI provider being used (OpenRouter or Ollama)
    pub provider: String,
    /// Model name being used
    pub model: String,
    /// Whether mutations are auto-executed by default
    pub auto_execute_mutations: bool,
    /// Default trust distance for mutations
    pub default_trust_distance: u32,
}

/// Simplified schema representation for AI analysis
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimplifiedSchema {
    /// Schema name
    pub name: String,
    /// Schema fields
    pub fields: HashMap<String, serde_json::Value>,
}

/// Map of simplified schemas keyed by schema name
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimplifiedSchemaMap {
    /// Schemas keyed by name
    pub schemas: HashMap<String, SimplifiedSchema>,
}

impl SimplifiedSchemaMap {
    /// Create a new empty schema map
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Add a schema to the map
    pub fn insert(&mut self, name: String, schema: SimplifiedSchema) {
        self.schemas.insert(name, schema);
    }

    /// Get the number of schemas in the map
    pub fn len(&self) -> usize {
        self.schemas.len()
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    /// Convert to JSON Value for AI API calls
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(&self.schemas)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
    }
}

impl Default for SimplifiedSchemaMap {
    fn default() -> Self {
        Self::new()
    }
}
