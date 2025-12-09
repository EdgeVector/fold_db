//! Output handlers for different logging destinations
//!
//! This module contains implementations for various log output types:
//! - Console output (with colors)
//! - File output (with rotation)
//! - Web streaming output
//! - Structured JSON output

pub mod console;
pub mod file;
pub mod web;
pub mod structured;
pub mod dynamodb;

pub use console::ConsoleOutput;
pub use file::FileOutput;
pub use web::WebOutput;
pub use structured::StructuredOutput;