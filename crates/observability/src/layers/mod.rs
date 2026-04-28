//! Subscriber layer implementations.
//!
//! - [`fmt`] — JSON formatter with redaction (T3, stub).
//! - [`reload`] — runtime [`tracing_subscriber::EnvFilter`] swap (T4).
//! - [`ring`] — bounded in-memory ring buffer for `/api/logs` (T5).
//! - [`web`] — broadcast fan-out for `/api/logs/stream` SSE (Phase 3 / T4).
//! - [`otlp_traces`] — OTLP HTTP/protobuf span exporter (Phase 4 / T1).
//! - [`span_metrics`] — pre-registered span-name → latency histogram
//!   recording for OTLP metrics export (Phase 4 / T2).
//! - [`otlp_metrics`] — OTLP HTTP/protobuf metrics exporter wrapping a
//!   [`opentelemetry_sdk::metrics::PeriodicReader`] (Phase 4 / T3).
//! - [`error`] — ERROR-only Sentry sink with W3C trace tagging
//!   (Phase 4 / T4).

pub mod error;
pub mod fmt;
pub mod otlp_metrics;
pub mod otlp_traces;
pub mod reload;
pub mod ring;
pub mod span_metrics;
pub mod web;

pub use error::{build_error_layer, ErrorLayer, SentryGuard, OBS_SENTRY_DSN_ENV};
pub use otlp_metrics::{
    build_otlp_metrics_meter_provider, OBS_OTLP_METRICS_ENDPOINT_ENV,
    OBS_OTLP_METRICS_INTERVAL_ENV, OBS_OTLP_METRICS_TIMEOUT_ENV,
};
pub use otlp_traces::{
    build_otlp_traces_layer, OtlpGuard, OBS_METER_SCOPE, OBS_OTLP_ENDPOINT_ENV,
    OBS_SPANS_DROPPED_METRIC,
};
pub use ring::{build_ring_layer, LogEntry, LogLevel, RingHandle, RingLayer, OBS_RING_CAPACITY};
pub use span_metrics::{
    build_span_metrics_layer, SpanMetricsLayer, ALLOWED_LABEL_KEYS, PRE_REGISTERED_SPANS,
};
pub use web::{build_web_layer, WebHandle, WebLayer, OBS_WEB_CAPACITY};
