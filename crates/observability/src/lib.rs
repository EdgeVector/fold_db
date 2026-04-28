//! Shared telemetry substrate for the fold_db ecosystem.
//!
//! This crate is the home for all things `tracing` + OpenTelemetry across
//! `fold_db`, `fold_db_node`, `schema_service`, the Tauri desktop app, and
//! Lambda handlers. It deliberately has zero dependencies on `fold_db` core
//! so it can be consumed from sibling crates and external repos without
//! pulling the world.
//!
//! Phase 1 ships:
//!
//! - [`attrs`] — canonical attribute keys + the [`redact!`] / [`redact_id!`]
//!   macros used at log call-sites for PII opacity (T2).
//! - [`propagation`] — W3C `traceparent` inject on `reqwest` egress and
//!   extract from `http::HeaderMap` on ingress (T2).
//! - [`layers`] — FMT (redacting JSON formatter, T3), RELOAD (runtime
//!   `EnvFilter` swap, T4), RING (bounded in-memory log buffer, T5).
//! - [`init`] — `init_node` / `init_lambda` / `init_tauri` / `init_cli`
//!   helpers (T6) that compose the layers per binary type and return an
//!   [`ObsGuard`] for the lifetime of the process.

pub mod attrs;
pub mod init;
pub mod layers;
pub mod propagation;
pub mod sampling;

pub use init::{init_cli, init_lambda, init_node, init_tauri, installed_service_name, ObsGuard};
pub use sampling::{parse_sampler, parse_sampler_spec, SamplerParseError, OBS_SAMPLER_ENV};

/// Errors raised by `init_*` helpers and other crate-level operations.
#[derive(Debug, thiserror::Error)]
pub enum ObsError {
    /// `init_*` was called more than once for the same target.
    #[error("observability already initialized")]
    AlreadyInitialized,
    /// Could not install the global tracing subscriber.
    #[error("failed to install tracing subscriber: {0}")]
    SubscriberInstall(String),
    /// Could not open or write to the configured sink (e.g. log file).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
