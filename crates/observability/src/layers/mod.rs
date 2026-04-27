//! Subscriber layer implementations.
//!
//! - [`fmt`] — JSON formatter with redaction (T3, stub).
//! - [`reload`] — runtime [`tracing_subscriber::EnvFilter`] swap (T4).
//! - [`ring`] — bounded in-memory ring buffer for `/api/logs` (T5).

pub mod fmt;
pub mod reload;
pub mod ring;

pub use ring::{build_ring_layer, LogEntry, LogLevel, RingHandle, RingLayer, OBS_RING_CAPACITY};
