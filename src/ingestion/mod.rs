//! Ingestion module: AI-powered JSON data ingestion for FoldDB.
//!
//! Accepts JSON data, uses an AI backend to recommend a schema, then generates
//! and optionally executes mutations to persist the data.

pub mod ai_client;
pub mod ai_helpers;
pub mod batch_controller;
pub mod config;
pub mod decomposer;
pub mod error;
pub mod file_conversion;
pub mod file_upload;
pub mod ingestion_service;
pub mod key_extraction;
pub mod json_processor;
pub mod mutation_generator;
pub mod progress;
pub mod prompts;
pub mod routes;
pub mod smart_folder;
pub mod smart_folder_routes;
pub mod smart_folder_scanner;
pub mod structure_analyzer;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// File extensions recognized as images (lowercase, without the leading dot).
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif", "svg", "heic", "heif", "avif",
];

/// Returns `true` when `filename` ends with a known image extension (case-insensitive).
pub fn is_image_file(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    IMAGE_EXTENSIONS.iter().any(|ext| lower.ends_with(&format!(".{}", ext)))
}

// Public re-exports
pub use ai_helpers::AISchemaResponse;
pub use config::IngestionConfig;
pub use error::IngestionError;
pub use progress::{
    create_progress_tracker, IngestionProgress, IngestionResults, IngestionStep, ProgressService,
    ProgressTracker, SchemaWriteRecord,
};
pub use structure_analyzer::StructureAnalyzer;

/// Result type for ingestion operations
pub type IngestionResult<T> = Result<T, IngestionError>;

fn default_true() -> bool {
    true
}

fn default_pub_key() -> String {
    "default".to_string()
}

/// Request for processing JSON ingestion.
///
/// This is the canonical request type used by both the HTTP server and Lambda handlers.
/// Fields use serde defaults so callers can omit optional parameters.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct IngestionRequest {
    /// JSON data to ingest
    pub data: serde_json::Value,
    /// Whether to auto-execute mutations after generation
    #[serde(default = "default_true")]
    pub auto_execute: bool,
    /// Public key for the operation
    #[serde(default = "default_pub_key")]
    pub pub_key: String,
    /// Original source filename (for file uploads)
    #[serde(default)]
    pub source_file_name: Option<String>,
    /// Progress tracking ID (optional, generated if not provided)
    #[serde(default)]
    pub progress_id: Option<String>,
    /// SHA256 hash of the original source file content (hex string)
    #[serde(default)]
    pub file_hash: Option<String>,
    /// Source folder path (set when ingested via smart folder or batch)
    #[serde(default)]
    pub source_folder: Option<String>,
    /// Descriptive name from image vision model (schema metadata, not record data)
    #[serde(default)]
    pub image_descriptive_name: Option<String>,
}

/// Response from the ingestion process
#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
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
    /// All schemas and keys written during this ingestion
    pub schemas_written: Vec<SchemaWriteRecord>,
}

impl IngestionResponse {
    /// Create a successful ingestion response with progress tracking
    pub fn success_with_progress(
        progress_id: String,
        schema_used: String,
        new_schema_created: bool,
        mutations_generated: usize,
        mutations_executed: usize,
        schemas_written: Vec<SchemaWriteRecord>,
    ) -> Self {
        Self {
            success: true,
            progress_id: Some(progress_id),
            schema_used: Some(schema_used),
            new_schema_created,
            mutations_generated,
            mutations_executed,
            schemas_written,
            ..Default::default()
        }
    }

    /// Create a failed ingestion response
    pub fn failure(errors: Vec<String>) -> Self {
        Self { errors, ..Default::default() }
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
}

