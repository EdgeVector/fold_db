//! Error types for the ingestion module

use thiserror::Error;

/// Errors that can occur during the ingestion process
#[derive(Error, Debug)]
pub enum IngestionError {
    /// OpenRouter API communication errors
    #[error("OpenRouter API error: {0}")]
    OpenRouterError(String),

    /// Ollama API communication errors
    #[error("Ollama API error: {0}")]
    OllamaError(String),

    /// HTTP request errors
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// JSON parsing errors
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Schema parsing errors from AI responses
    #[error("Schema parsing error: {0}")]
    SchemaParsingError(String),

    /// Mutation generation errors
    #[error("Mutation generation error: {0}")]
    MutationGenerationError(String),

    /// Schema creation errors
    #[error("Schema creation error: {0}")]
    SchemaCreationError(String),

    /// Configuration errors (missing API keys, etc.)
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Schema system errors
    #[error("Schema system error: {0}")]
    SchemaSystemError(#[from] crate::schema::SchemaError),

    /// Database errors
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Invalid input data
    #[error("Invalid input data: {0}")]
    InvalidInput(String),

    /// AI response validation errors
    #[error("AI response validation error: {0}")]
    AIResponseValidationError(String),

    /// Path parsing errors for JSON field paths
    #[error("Path parsing error: {0}")]
    PathParsingError(String),

    /// Field mapping errors
    #[error("Field mapping error: {0}")]
    FieldMappingError(String),

    /// File conversion errors (e.g., PDF to JSON conversion failed)
    #[error("File conversion failed")]
    FileConversionFailed,

    /// Storage errors (S3, file system, etc.)
    #[error("Storage error: {0}")]
    StorageError(String),
}

impl IngestionError {
    /// Create a new OpenRouter API error
    pub fn openrouter_error(msg: impl Into<String>) -> Self {
        Self::OpenRouterError(msg.into())
    }

    /// Create a new Ollama API error
    pub fn ollama_error(msg: impl Into<String>) -> Self {
        Self::OllamaError(msg.into())
    }

    /// Create a new schema parsing error
    pub fn schema_parsing_error(msg: impl Into<String>) -> Self {
        Self::SchemaParsingError(msg.into())
    }

    /// Create a new mutation generation error
    pub fn mutation_generation_error(msg: impl Into<String>) -> Self {
        Self::MutationGenerationError(msg.into())
    }

    /// Create a new configuration error
    pub fn configuration_error(msg: impl Into<String>) -> Self {
        Self::ConfigurationError(msg.into())
    }

    /// Create a new invalid input error
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create a new AI response validation error
    pub fn ai_response_validation_error(msg: impl Into<String>) -> Self {
        Self::AIResponseValidationError(msg.into())
    }

    /// Create a new path parsing error
    pub fn path_parsing_error(msg: impl Into<String>) -> Self {
        Self::PathParsingError(msg.into())
    }

    /// Create a new field mapping error
    pub fn field_mapping_error(msg: impl Into<String>) -> Self {
        Self::FieldMappingError(msg.into())
    }

    /// Create a new file conversion error
    pub fn file_conversion_failed() -> Self {
        Self::FileConversionFailed
    }

    /// Create a new storage error
    pub fn storage_error(msg: impl Into<String>) -> Self {
        Self::StorageError(msg.into())
    }
}

/// Result type for ingestion operations
pub type Result<T> = std::result::Result<T, IngestionError>;
