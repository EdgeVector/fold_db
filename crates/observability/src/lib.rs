//! Shared telemetry substrate for the fold_db ecosystem.
//!
//! This crate is the home for all things `tracing` + Sentry trace tagging
//! across `fold_db`, `fold_db_node`, `schema_service`, the Tauri desktop
//! app, and Lambda handlers. It deliberately has zero dependencies on
//! `fold_db` core so it can be consumed from sibling crates and external
//! repos without pulling the world.
//!
//! What ships:
//!
//! - [`attrs`] — canonical attribute keys + the [`redact!`] / [`redact_id!`]
//!   macros used at log call-sites for PII opacity.
//! - [`propagation`] — W3C `traceparent` inject on `reqwest` egress and
//!   extract from `http::HeaderMap` on ingress.
//! - [`layers`] — FMT (redacting JSON formatter), RELOAD (runtime
//!   `EnvFilter` swap), RING (bounded in-memory log buffer), WEB (SSE
//!   broadcast), and the ERROR-only Sentry sink.
//! - [`init`] — `init_node` / `init_node_with_web` / `init_lambda` /
//!   `init_tauri` / `init_cli` helpers that compose the layers per binary
//!   type and return an [`ObsGuard`] (or [`NodeObsGuardWithWeb`] for the
//!   web-streaming variant) for the lifetime of the process.

pub mod attrs;
pub mod init;
pub mod layers;
pub mod propagation;

pub use init::{
    init_cli, init_lambda, init_node, init_node_with_web, init_tauri, installed_service_name,
    NodeObsGuardWithWeb, ObsGuard, ObsHandles,
};

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
