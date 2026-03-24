//! Centralized LLM registry for model definitions, provider configs, and prompt templates.
//!
//! All LLM model IDs, API configurations, and prompt templates live here so that
//! every project in the workspace (`fold_db_node`, `file_to_json`, `file_to_markdown`,
//! `exemem-infra`) can reference a single source of truth.
//!
//! # Usage
//!
//! ```rust
//! use fold_db::llm_registry::models;
//! use fold_db::llm_registry::prompts;
//!
//! // Model constants
//! let model = models::ANTHROPIC_SONNET;
//! let api_url = models::ANTHROPIC_API_URL;
//!
//! // Prompt templates
//! let header = prompts::ingestion::PROMPT_HEADER;
//! let prompt = prompts::classification::build_classification_prompt("email", "user email address");
//! ```

pub mod models;
pub mod prompts;
