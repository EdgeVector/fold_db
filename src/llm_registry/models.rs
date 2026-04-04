//! Model constants, provider defaults, and generation parameters.
//!
//! Every hardcoded model ID across the workspace should reference these constants
//! so that model upgrades happen in one place.

// ---- Anthropic Models ----

/// Fast, cheap model for structured classification tasks.
/// Used by: schema service field classification (`fold_db`).
pub const ANTHROPIC_HAIKU: &str = "claude-haiku-4-5-20251001";

/// Default model for complex reasoning tasks (ingestion, query analysis, agents).
/// Used by: `fold_db_node` ingestion + LLM query service.
pub const ANTHROPIC_SONNET: &str = "claude-sonnet-4-20250514";

/// OpenRouter-style model path for Sonnet.
pub const ANTHROPIC_SONNET_OPENROUTER: &str = "anthropic/claude-sonnet-4-20250514";

// ---- Ollama Models ----

/// Default Ollama model for general text tasks (ingestion, queries).
/// Used by: `fold_db_node` Ollama backend, `fold_db` schema service classification.
/// Must be a model that works on most hardware (8B params).
/// The ingestion config in `fold_db_node` upgrades to larger models based on system RAM.
pub const OLLAMA_DEFAULT: &str = "llama3.1:8b";

/// Vision model for image captioning and classification.
/// Used by: `file_to_markdown` image extraction.
pub const OLLAMA_VISION: &str = "qwen3-vl:2b";

/// OCR model for text extraction from scanned documents and image-based PDFs.
/// Used by: `file_to_markdown` PDF/document extraction.
pub const OLLAMA_OCR: &str = "glm-ocr:latest";

// ---- Embedding Models ----

/// Sentence embedding model for semantic search indexing.
/// Used by: `fold_db` native index (`fastembed` crate, ONNX runtime).
/// Dimension: 384.
pub const EMBEDDING_MODEL: &str = "all-MiniLM-L6-v2";

/// Vector dimension produced by [`EMBEDDING_MODEL`].
pub const EMBEDDING_DIMENSION: usize = 384;

// ---- API Configuration ----

/// Anthropic Messages API base URL.
pub const ANTHROPIC_API_URL: &str = "https://api.anthropic.com";

/// Anthropic API version header value.
pub const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// Default Ollama server URL.
pub const OLLAMA_DEFAULT_URL: &str = "http://localhost:11434";

// ---- Temperature Presets ----

/// Deterministic output — classification, structured extraction.
pub const TEMPERATURE_DETERMINISTIC: f32 = 0.0;

/// Low creativity — ingestion schema analysis, query planning.
pub const TEMPERATURE_FOCUSED: f32 = 0.1;

/// Higher creativity — Ollama default, open-ended generation.
pub const TEMPERATURE_CREATIVE: f32 = 0.8;

// ---- Token Limits ----

/// Max output tokens for classification tasks — cloud APIs (small JSON responses).
pub const MAX_TOKENS_CLASSIFICATION: u32 = 100;

/// Max output tokens for Ollama classification — local tokenizers vary, so allow more headroom.
pub const MAX_TOKENS_CLASSIFICATION_OLLAMA: u32 = 256;

/// Max output tokens for ingestion / query analysis (large JSON + reasoning).
pub const MAX_TOKENS_ANALYSIS: u32 = 16_000;

// ---- Timeout Presets (seconds) ----

/// Quick LLM calls — cloud APIs (classification, small structured output).
pub const TIMEOUT_CLASSIFICATION: u64 = 15;

/// Ollama classification — local models need longer for cold starts.
pub const TIMEOUT_CLASSIFICATION_OLLAMA: u64 = 60;

/// Standard LLM calls (ingestion, query planning).
pub const TIMEOUT_STANDARD: u64 = 300;

// ---- Ollama Generation Parameter Defaults ----

/// Default context window for Ollama models (tokens).
pub const OLLAMA_NUM_CTX: u32 = 16_384;

/// Default max prediction tokens for Ollama.
pub const OLLAMA_NUM_PREDICT: u32 = 16_384;

/// Default top-p sampling for Ollama.
pub const OLLAMA_TOP_P: f32 = 0.95;

/// Default top-k sampling for Ollama (0 = disabled).
pub const OLLAMA_TOP_K: u32 = 0;

/// Default repeat penalty for Ollama.
pub const OLLAMA_REPEAT_PENALTY: f32 = 1.0;

/// Default presence penalty for Ollama.
pub const OLLAMA_PRESENCE_PENALTY: f32 = 0.0;

/// Default min-p threshold for Ollama.
pub const OLLAMA_MIN_P: f32 = 0.0;

// ---- Ingestion Limits ----

/// Maximum characters before a field value is truncated in prompts.
pub const PROMPT_FIELD_TRUNCATION_LIMIT: usize = 300;

/// Maximum bytes for LLM input before chunking kicks in.
pub const MAX_LLM_INPUT_BYTES: usize = 128 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_ids_are_non_empty() {
        assert!(!ANTHROPIC_HAIKU.is_empty());
        assert!(!ANTHROPIC_SONNET.is_empty());
        assert!(!ANTHROPIC_SONNET_OPENROUTER.is_empty());
        assert!(!OLLAMA_DEFAULT.is_empty());
        assert!(!OLLAMA_VISION.is_empty());
        assert!(!OLLAMA_OCR.is_empty());
        assert!(!EMBEDDING_MODEL.is_empty());
    }

    #[test]
    fn api_urls_are_valid() {
        assert!(ANTHROPIC_API_URL.starts_with("https://"));
        assert!(OLLAMA_DEFAULT_URL.starts_with("http://"));
    }

    #[test]
    fn temperatures_in_valid_range() {
        assert!((0.0..=2.0).contains(&TEMPERATURE_DETERMINISTIC));
        assert!((0.0..=2.0).contains(&TEMPERATURE_FOCUSED));
        assert!((0.0..=2.0).contains(&TEMPERATURE_CREATIVE));
    }
}
