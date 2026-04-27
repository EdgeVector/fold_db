//! Subscriber layer implementations.
//!
//! - [`fmt`] — JSON formatter with redaction (T3, stub).
//! - [`reload`] — runtime [`tracing_subscriber::EnvFilter`] swap (T4).
//! - [`ring`] — bounded in-memory log buffer (T5, stub).

pub mod fmt;
pub mod reload;
pub mod ring;
