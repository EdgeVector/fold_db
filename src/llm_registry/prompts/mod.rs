//! Centralized prompt templates for all LLM interactions across the workspace.
//!
//! Organized by domain:
//! - [`classification`] — Field sensitivity/domain classification
//! - [`ingestion`] — Schema analysis from raw data
//! - [`query`] — Natural language query planning and result interpretation
//! - [`smart_folder`] — File classification for ingestion scanning

pub mod classification;
pub mod ingestion;
pub mod query;
pub mod smart_folder;
