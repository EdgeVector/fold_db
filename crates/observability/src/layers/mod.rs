//! Subscriber layer implementations.
//!
//! - [`fmt`] — JSON formatter with redaction (T3, stub).
//! - [`reload`] — runtime [`tracing_subscriber::EnvFilter`] swap (T4).
//! - [`ring`] — bounded in-memory ring buffer for `/api/logs` (T5).
//! - [`web`] — broadcast fan-out for `/api/logs/stream` SSE (Phase 3 / T4).
//! - [`otlp_traces`] — OTLP HTTP/protobuf span exporter (Phase 4 / T1).
//! - [`span_metrics`] — pre-registered span-name → latency histogram
//!   recording for OTLP metrics export (Phase 4 / T2).

pub mod fmt;
pub mod otlp_traces;
pub mod reload;
pub mod ring;
pub mod span_metrics;
pub mod web;

pub use otlp_traces::{build_otlp_traces_layer, OtlpGuard, OBS_OTLP_ENDPOINT_ENV};
pub use ring::{build_ring_layer, LogEntry, LogLevel, RingHandle, RingLayer, OBS_RING_CAPACITY};
pub use span_metrics::{
    build_span_metrics_layer, SpanMetricsLayer, ALLOWED_LABEL_KEYS, PRE_REGISTERED_SPANS,
};
pub use web::{build_web_layer, WebHandle, WebLayer, OBS_WEB_CAPACITY};
