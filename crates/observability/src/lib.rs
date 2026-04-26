//! Shared telemetry substrate for the fold_db ecosystem.
//!
//! This crate is the home for all things `tracing` + OpenTelemetry across
//! `fold_db`, `fold_db_node`, `schema_service`, the Tauri desktop app, and
//! Lambda handlers. It deliberately has zero dependencies on `fold_db` core
//! so it can be consumed from sibling crates and external repos without
//! pulling the world.
//!
//! Phase 1 / T2 ships:
//!
//! - [`attrs`] — canonical attribute keys + the [`redact!`] / [`redact_id!`]
//!   macros used at log call-sites for PII opacity.
//! - [`propagation`] — W3C `traceparent` inject on `reqwest` egress and
//!   extract from `http::HeaderMap` on ingress.
//! - [`init`] and [`layers`] — empty stubs; future tasks (T3..T6) populate
//!   the FMT, RELOAD, RING layers and the `init_*` helpers.

pub mod attrs;
pub mod init;
pub mod layers;
pub mod propagation;

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
