//! Subscriber layer implementations.
//!
//! - [`fmt`] — JSON formatter with redaction (T3, stub).
//! - [`reload`] — runtime [`tracing_subscriber::EnvFilter`] swap (T4).
//! - [`ring`] — bounded in-memory ring buffer for `/api/logs` (T5).
//! - [`web`] — broadcast fan-out for `/api/logs/stream` SSE (Phase 3 / T4).
//! - [`error`] — ERROR-only Sentry sink with W3C trace tagging
//!   (Phase 4 / T4).

pub mod error;
pub mod fmt;
pub mod reload;
pub mod ring;
pub mod web;

pub use error::{build_error_layer, ErrorLayer, SentryGuard, OBS_SENTRY_DSN_ENV};
pub use ring::{build_ring_layer, LogEntry, LogLevel, RingHandle, RingLayer, OBS_RING_CAPACITY};
pub use web::{build_web_layer, WebHandle, WebLayer, OBS_WEB_CAPACITY};
