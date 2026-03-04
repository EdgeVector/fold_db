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

    /// Schema creation errors
    #[error("Schema creation error: {0}")]
    SchemaCreationError(String),

    /// Configuration errors (missing API keys, etc.)
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Schema system errors
    #[error("Schema system error: {0}")]
    SchemaSystemError(#[from] crate::schema::SchemaError),

    /// Invalid input data
    #[error("Invalid input data: {0}")]
    InvalidInput(String),

    /// AI response validation errors
    #[error("AI response validation error: {0}")]
    AIResponseValidationError(String),

    /// File conversion errors (e.g., PDF to JSON conversion failed)
    #[error("File conversion failed: {0}")]
    FileConversionFailed(String),

    /// Storage errors (S3, file system, etc.)
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Security/Auth errors
    #[error("Security error: {0}")]
    SecurityError(String),

    /// Authentication errors (invalid or expired API key)
    #[error("{provider} authentication error: {message}")]
    AuthenticationError { provider: String, message: String },

    /// Rate limit errors (too many requests)
    #[error("{provider} rate limited: {message}")]
    RateLimitError { provider: String, message: String },

    /// Timeout errors (request took too long)
    #[error("{provider} request timed out: {message}")]
    TimeoutError { provider: String, message: String },

    /// Connection errors (cannot reach the AI service)
    #[error("{provider} connection error: {message}")]
    ConnectionError { provider: String, message: String },
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

    /// Returns a concise, user-friendly message suitable for UI display.
    pub fn user_message(&self) -> String {
        match self {
            Self::AuthenticationError { provider, .. } => {
                format!("{} API key is invalid or expired. Check your configuration.", provider)
            }
            Self::RateLimitError { provider, .. } => {
                format!("{} rate limit reached. Please wait a moment and try again.", provider)
            }
            Self::TimeoutError { provider, .. } => {
                format!("{} request timed out. The service may be slow or unavailable.", provider)
            }
            Self::ConnectionError { provider, message } => {
                format!("Cannot connect to {}. {}", provider, message)
            }
            Self::ConfigurationError(msg) => {
                format!("Configuration error: {}", msg)
            }
            Self::InvalidInput(msg) => {
                format!("Invalid input: {}", msg)
            }
            _ => self.to_string(),
        }
    }
}

/// Classify an HTTP error response from an LLM provider into a specific error variant.
pub fn classify_llm_error(provider: &str, status_code: u16, body: &str) -> IngestionError {
    match status_code {
        401 => IngestionError::AuthenticationError {
            provider: provider.to_string(),
            message: "API key invalid or expired".to_string(),
        },
        402 => IngestionError::ConfigurationError(
            format!("{}: insufficient credits — add funds to your account", provider),
        ),
        404 => IngestionError::ConfigurationError(
            format!("{}: model not found — check your model setting", provider),
        ),
        429 => IngestionError::RateLimitError {
            provider: provider.to_string(),
            message: format!("Too many requests. {}", truncate_body(body)),
        },
        500..=599 => IngestionError::ConnectionError {
            provider: provider.to_string(),
            message: format!("Server error (HTTP {}). Try again later.", status_code),
        },
        _ => IngestionError::OpenRouterError(
            format!("API request failed with status {}: {}", status_code, truncate_body(body)),
        ),
    }
}

/// Classify a transport-level (reqwest) error into a specific error variant.
pub fn classify_transport_error(provider: &str, err: &reqwest::Error) -> IngestionError {
    if err.is_timeout() {
        IngestionError::TimeoutError {
            provider: provider.to_string(),
            message: "Request exceeded the configured timeout".to_string(),
        }
    } else if err.is_connect() {
        IngestionError::ConnectionError {
            provider: provider.to_string(),
            message: format!("Could not connect. Is the service running? ({})", err),
        }
    } else {
        IngestionError::ConnectionError {
            provider: provider.to_string(),
            message: format!("Network error: {}", err),
        }
    }
}

/// Truncate a response body to avoid huge error messages.
fn truncate_body(body: &str) -> String {
    if body.len() > 200 {
        format!("{}...", &body[..200])
    } else {
        body.to_string()
    }
}

/// Result type for ingestion operations
pub type Result<T> = std::result::Result<T, IngestionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_llm_error_401() {
        let err = classify_llm_error("OpenRouter", 401, "Unauthorized");
        assert!(matches!(err, IngestionError::AuthenticationError { .. }));
        assert!(err.user_message().contains("invalid or expired"));
    }

    #[test]
    fn test_classify_llm_error_402() {
        let err = classify_llm_error("OpenRouter", 402, "Payment required");
        assert!(matches!(err, IngestionError::ConfigurationError(_)));
        assert!(err.user_message().contains("insufficient credits"));
    }

    #[test]
    fn test_classify_llm_error_404() {
        let err = classify_llm_error("OpenRouter", 404, "Not found");
        assert!(matches!(err, IngestionError::ConfigurationError(_)));
        assert!(err.user_message().contains("model not found"));
    }

    #[test]
    fn test_classify_llm_error_429() {
        let err = classify_llm_error("OpenRouter", 429, "Rate limited");
        assert!(matches!(err, IngestionError::RateLimitError { .. }));
        assert!(err.user_message().contains("rate limit"));
    }

    #[test]
    fn test_classify_llm_error_500() {
        let err = classify_llm_error("OpenRouter", 500, "Internal server error");
        assert!(matches!(err, IngestionError::ConnectionError { .. }));
        assert!(err.user_message().contains("Cannot connect"));
    }

    #[test]
    fn test_classify_llm_error_503() {
        let err = classify_llm_error("Ollama", 503, "Service unavailable");
        assert!(matches!(err, IngestionError::ConnectionError { .. }));
    }

    #[test]
    fn test_classify_llm_error_unknown_status() {
        let err = classify_llm_error("OpenRouter", 418, "I'm a teapot");
        assert!(matches!(err, IngestionError::OpenRouterError(_)));
    }

    #[test]
    fn test_user_message_all_variants() {
        let auth = IngestionError::AuthenticationError {
            provider: "TestProvider".to_string(),
            message: "bad key".to_string(),
        };
        assert!(auth.user_message().contains("TestProvider"));
        assert!(auth.user_message().contains("invalid or expired"));

        let rate = IngestionError::RateLimitError {
            provider: "TestProvider".to_string(),
            message: "slow down".to_string(),
        };
        assert!(rate.user_message().contains("rate limit"));

        let timeout = IngestionError::TimeoutError {
            provider: "TestProvider".to_string(),
            message: "took too long".to_string(),
        };
        assert!(timeout.user_message().contains("timed out"));

        let conn = IngestionError::ConnectionError {
            provider: "TestProvider".to_string(),
            message: "Is the service running?".to_string(),
        };
        assert!(conn.user_message().contains("Cannot connect"));
        assert!(conn.user_message().contains("Is the service running?"));

        let config = IngestionError::ConfigurationError("missing key".to_string());
        assert!(config.user_message().contains("missing key"));

        let input = IngestionError::InvalidInput("bad data".to_string());
        assert!(input.user_message().contains("bad data"));

        // Other variants fall through to Display
        let schema = IngestionError::SchemaParsingError("parse fail".to_string());
        assert!(schema.user_message().contains("parse fail"));
    }

    #[test]
    fn test_truncate_body_short() {
        let short = "short body";
        assert_eq!(truncate_body(short), "short body");
    }

    #[test]
    fn test_truncate_body_long() {
        let long = "x".repeat(300);
        let truncated = truncate_body(&long);
        assert!(truncated.len() < 210);
        assert!(truncated.ends_with("..."));
    }
}
