//! Initialization helpers for each runtime target (node / Lambda / Tauri / CLI).
//!
//! Phase 1 / T6. Each helper composes the FMT + RELOAD + RING layers into a
//! `Registry` and installs it as the global tracing subscriber. The returned
//! [`ObsGuard`] holds the non-blocking writer's worker handle plus the
//! [`RingHandle`] / [`ReloadHandle`] the rest of the binary uses to query the
//! ring buffer or swap the active filter at runtime.
//!
//! ## Per-target shape
//!
//! | helper        | FMT target               | RELOAD | RING |
//! |---------------|--------------------------|--------|------|
//! | [`init_node`]   | `~/.folddb/observability.jsonl` (or `OBS_FILE_PATH`) | yes | yes |
//! | [`init_lambda`] | stdout                   | yes    | no   |
//! | [`init_tauri`]  | inherits from embedded server, else delegates to [`init_node`] | conditional | conditional |
//! | [`init_cli`]    | stderr                   | no     | no   |
//!
//! ## Single-init invariant
//!
//! A process-global [`once_cell::sync::OnceCell`] enforces exactly one
//! installation. The first successful call wins; every subsequent call
//! returns [`crate::ObsError::AlreadyInitialized`] without panicking and
//! without touching the installed subscriber. [`init_tauri`] is the lone
//! exception — when it sees the cell already set, it assumes the embedded
//! fold_db server already booted observability and returns a degraded
//! "attached" guard rather than an error, so the Tauri shell can keep
//! running on top of the server's subscriber.
//!
//! ## Contract for callers
//!
//! - `service_name` must be non-empty. Empty input panics — this is a
//!   programming error, not a runtime one.
//! - The returned [`ObsGuard`] **must** be held for the lifetime of the
//!   binary. Dropping it stops the FMT worker thread mid-flush; any log
//!   lines still in the channel are lost.

use std::path::PathBuf;
use std::{fs, io};

use once_cell::sync::OnceCell;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
use tracing_log::LogTracer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

use crate::layers::fmt::{build_fmt_writer, FmtGuard, FmtTarget, RedactingFormat};
use crate::layers::reload::{build_reload_layer, ReloadHandle};
use crate::layers::ring::{build_ring_layer, RingHandle, OBS_RING_CAPACITY};
use crate::ObsError;

/// Override for the node log file path. Read once per `init_node` call.
const OBS_FILE_PATH_ENV: &str = "OBS_FILE_PATH";

/// Process-global guard against double init. Set on the first successful
/// `init_*` call; remains set for the lifetime of the process.
static INIT_ONCE: OnceCell<()> = OnceCell::new();

/// RAII handle returned by every `init_*` helper.
///
/// Holds:
/// - the FMT layer's [`tracing_appender::non_blocking`] worker guard (when
///   this process did the install) so the background flush thread keeps
///   draining the queue,
/// - the [`RingHandle`] for in-process `/api/logs` queries (when RING is
///   wired — currently only [`init_node`] / [`init_tauri`] full path),
/// - the [`ReloadHandle`] for runtime `EnvFilter` updates (when RELOAD is
///   wired — every helper except [`init_cli`]).
///
/// The OTLP shutdown handle slot is reserved for Phase 4; for now it is
/// always `None`.
#[must_use = "ObsGuard must be held for the lifetime of the binary or log lines may be dropped"]
pub struct ObsGuard {
    fmt_guard: Option<FmtGuard>,
    ring: Option<RingHandle>,
    reload: Option<ReloadHandle>,
    otlp_shutdown: Option<OtlpShutdown>,
}

/// Reserved for Phase 4 (OTLP exporter wiring). Holding the placeholder type
/// here lets us add the real shutdown call without changing `ObsGuard`'s shape.
struct OtlpShutdown;

impl ObsGuard {
    /// Handle to the in-memory ring buffer. `None` for targets that don't
    /// install the RING layer (Lambda, CLI) or for the Tauri "attached"
    /// degraded guard.
    pub fn ring(&self) -> Option<&RingHandle> {
        self.ring.as_ref()
    }

    /// Handle to swap the active `EnvFilter` at runtime. `None` for targets
    /// that don't install the RELOAD layer (CLI) or for the Tauri "attached"
    /// degraded guard.
    pub fn reload(&self) -> Option<&ReloadHandle> {
        self.reload.as_ref()
    }
}

impl std::fmt::Debug for ObsGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObsGuard")
            .field("fmt_guard", &self.fmt_guard.is_some())
            .field("ring", &self.ring.is_some())
            .field("reload", &self.reload.is_some())
            .field("otlp_shutdown", &self.otlp_shutdown.is_some())
            .finish()
    }
}

impl Drop for ObsGuard {
    fn drop(&mut self) {
        // Phase 4 will call `otlp_shutdown.shutdown()` here. Today the slot
        // is always None; the FmtGuard's own Drop drains the writer queue.
        if let Some(_otlp) = self.otlp_shutdown.take() {
            // no-op until Phase 4
        }
    }
}

// ---------------------------------------------------------------------------
// Public init helpers
// ---------------------------------------------------------------------------

/// Initialize observability for a long-running node binary.
///
/// Layers: redacting JSON FMT writing to `~/.folddb/observability.jsonl`
/// (override with `OBS_FILE_PATH`) + RELOAD + RING.
///
/// Also installs the W3C [`TraceContextPropagator`] globally and the
/// `tracing-log` bridge so third-party `log::*` calls flow through the
/// subscriber.
pub fn init_node(service_name: &'static str, _version: &str) -> Result<ObsGuard, ObsError> {
    assert_service_name(service_name);
    try_claim_init(&INIT_ONCE)?;

    let path = default_node_log_path()?;
    let (writer, fmt_guard) = build_fmt_writer(FmtTarget::File(path))?;
    let (reload_layer, reload) = build_reload_layer::<Registry>(default_env_filter());
    let (ring_layer, ring) = build_ring_layer(OBS_RING_CAPACITY);
    // No-op TracerProvider: gives every span a real W3C trace/span id so the
    // RING layer can stamp `trace_id` / `span_id` onto each entry and so
    // `propagation::inject_w3c` (Phase 2) has a real context to propagate.
    // No exporter is wired in Phase 1 — Phase 4 swaps this for OTLP.
    let tracer_provider = SdkTracerProvider::builder().build();
    let tracer = tracer_provider.tracer(service_name);
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    // The fmt layer is constructed inline so the compiler infers its
    // `Subscriber` type parameter from the composition site, which
    // includes the reload Layered<...> wrapping below.
    let fmt_layer = tracing_subscriber::fmt::layer()
        .event_format(RedactingFormat::from_env_with_service(service_name))
        .with_writer(writer);

    // RELOAD is innermost so its `S = Registry` type binding matches; the
    // remaining layers are generic over `S` and the compiler infers each one
    // from the composition site. By the time RING's `on_event` runs, OTel's
    // `on_new_span` has already attached `OtelData` to the parent span, so
    // RING's extension lookup finds the trace/span ids regardless of layer
    // ordering at this level.
    let subscriber = Registry::default()
        .with(reload_layer)
        .with(otel_layer)
        .with(fmt_layer)
        .with(ring_layer);
    install_subscriber(subscriber)?;
    install_globals();

    Ok(ObsGuard {
        fmt_guard: Some(fmt_guard),
        ring: Some(ring),
        reload: Some(reload),
        otlp_shutdown: None,
    })
}

/// Initialize observability for an AWS Lambda handler.
///
/// Layers: redacting JSON FMT to stdout + RELOAD. Lambda's own log capture
/// pipes stdout to CloudWatch, so a file appender would be wasted IO. RING
/// is omitted — Lambda invocations are too short-lived for an in-process
/// query buffer to be useful.
pub fn init_lambda(service_name: &'static str, _version: &str) -> Result<ObsGuard, ObsError> {
    assert_service_name(service_name);
    try_claim_init(&INIT_ONCE)?;

    let (writer, fmt_guard) = build_fmt_writer(FmtTarget::Stdout)?;
    let (reload_layer, reload) = build_reload_layer::<Registry>(default_env_filter());
    let fmt_layer = tracing_subscriber::fmt::layer()
        .event_format(RedactingFormat::from_env())
        .with_writer(writer);

    let subscriber = Registry::default().with(reload_layer).with(fmt_layer);
    install_subscriber(subscriber)?;
    install_globals();

    Ok(ObsGuard {
        fmt_guard: Some(fmt_guard),
        ring: None,
        reload: Some(reload),
        otlp_shutdown: None,
    })
}

/// Initialize observability inside a Tauri shell.
///
/// The Tauri desktop app embeds a full fold_db server, which calls
/// [`init_node`] from `start_server()`. By the time the Tauri runtime
/// invokes this helper, the global subscriber is already installed — so we
/// detect that and return a degraded "attached" [`ObsGuard`] rather than
/// fail. When the embedded server has *not* run (e.g. dev shell pointed at
/// a remote server), we fall through to a full [`init_node`] install.
///
/// `app_handle` is taken by reference but unused in Phase 1; Phase 3 will
/// wire `tauri-plugin-log` as an additional sink. The generic parameter
/// avoids pulling Tauri into the observability crate's dependency graph —
/// callers in the desktop binary pass `&tauri::AppHandle`.
pub fn init_tauri<H>(
    service_name: &'static str,
    version: &str,
    _app_handle: &H,
) -> Result<ObsGuard, ObsError> {
    assert_service_name(service_name);

    if INIT_ONCE.get().is_some() {
        // Embedded server already initialized. We can't compose new layers
        // onto an installed global subscriber, so the Tauri shell rides on
        // top of the server's. Phase 3 will swap this for a real
        // tauri-plugin-log attachment.
        return Ok(ObsGuard {
            fmt_guard: None,
            ring: None,
            reload: None,
            otlp_shutdown: None,
        });
    }

    init_node(service_name, version)
}

/// Initialize observability for a short-lived CLI binary.
///
/// Layers: redacting JSON FMT to stderr only. No RELOAD (CLIs run to
/// completion — runtime filter swaps add no value), no RING (no in-process
/// reader on the other end), no file appender (no daemon to flush).
/// stderr is chosen so the CLI can keep stdout reserved for its own
/// program output.
pub fn init_cli(service_name: &'static str, _version: &str) -> Result<ObsGuard, ObsError> {
    assert_service_name(service_name);
    try_claim_init(&INIT_ONCE)?;

    let (writer, fmt_guard) = build_fmt_writer(FmtTarget::Stderr)?;
    let fmt_layer = tracing_subscriber::fmt::layer()
        .event_format(RedactingFormat::from_env())
        .with_writer(writer);

    let subscriber = Registry::default().with(fmt_layer);
    install_subscriber(subscriber)?;
    install_globals();

    Ok(ObsGuard {
        fmt_guard: Some(fmt_guard),
        ring: None,
        reload: None,
        otlp_shutdown: None,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[inline]
fn assert_service_name(name: &str) {
    assert!(!name.is_empty(), "service_name required");
}

/// Atomically claim the one-shot init slot. Returns
/// [`ObsError::AlreadyInitialized`] when another caller already set it.
fn try_claim_init(cell: &OnceCell<()>) -> Result<(), ObsError> {
    cell.set(()).map_err(|_| ObsError::AlreadyInitialized)
}

fn default_env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

/// Path the node binary appends JSON events to.
///
/// Order of resolution:
/// 1. `$OBS_FILE_PATH` if set — used as-is, with no parent-directory
///    creation. The caller chose the path; the caller is responsible.
/// 2. `~/.folddb/observability.jsonl` — `~/.folddb` is created if absent.
fn default_node_log_path() -> Result<PathBuf, ObsError> {
    if let Ok(p) = std::env::var(OBS_FILE_PATH_ENV) {
        return Ok(PathBuf::from(p));
    }
    let home = std::env::var("HOME").map_err(|_| {
        ObsError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "HOME not set; set OBS_FILE_PATH to choose a log path explicitly",
        ))
    })?;
    let mut dir = PathBuf::from(home);
    dir.push(".folddb");
    fs::create_dir_all(&dir)?;
    dir.push("observability.jsonl");
    Ok(dir)
}

fn install_subscriber<S>(subscriber: S) -> Result<(), ObsError>
where
    S: tracing::Subscriber + Send + Sync + 'static,
{
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| ObsError::SubscriberInstall(e.to_string()))
}

/// Install process-global telemetry plumbing that lives outside the
/// subscriber: the W3C text-map propagator and the `log` → `tracing` bridge.
///
/// Both are idempotent in practice — the propagator setter overwrites,
/// `LogTracer::init` returns `Err` on the second call which we deliberately
/// swallow. Called only after `install_subscriber` succeeds.
fn install_globals() {
    global::set_text_map_propagator(TraceContextPropagator::new());
    // `LogTracer::init` errors only when called twice in the same process;
    // that's expected for retries / multiple test cases and not actionable.
    let _ = LogTracer::init();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "service_name required")]
    fn empty_service_name_panics() {
        assert_service_name("");
    }

    #[test]
    fn non_empty_service_name_does_not_panic() {
        assert_service_name("ok");
    }

    #[test]
    fn try_claim_init_returns_err_on_second_call() {
        let cell: OnceCell<()> = OnceCell::new();
        try_claim_init(&cell).expect("first claim succeeds");
        let err = try_claim_init(&cell).expect_err("second claim must fail");
        assert!(matches!(err, ObsError::AlreadyInitialized), "got: {err:?}");
    }

    #[test]
    fn obs_file_path_env_overrides_default() {
        // Use a path with no $HOME dependency so the test is hermetic
        // regardless of the parent process environment.
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("custom.jsonl");
        let prev = std::env::var(OBS_FILE_PATH_ENV).ok();
        std::env::set_var(OBS_FILE_PATH_ENV, &target);

        let resolved = default_node_log_path().expect("path resolves");

        match prev {
            Some(v) => std::env::set_var(OBS_FILE_PATH_ENV, v),
            None => std::env::remove_var(OBS_FILE_PATH_ENV),
        }

        assert_eq!(resolved, target);
    }

    #[test]
    fn default_env_filter_falls_back_to_info() {
        // We can't easily assert the filter's parsed shape, but we can at
        // least confirm the function returns without panicking when no
        // `RUST_LOG` is set in a way that would parse-fail. The fallback
        // path in `unwrap_or_else` covers both unset and parse-error cases.
        let _filter = default_env_filter();
    }

    /// Smoke-test that `ObsGuard`'s public surface is what callers will
    /// touch — `ring()` / `reload()` accessors return the inner handles.
    #[test]
    fn obs_guard_accessors_match_inner_state() {
        let guard = ObsGuard {
            fmt_guard: None,
            ring: None,
            reload: None,
            otlp_shutdown: None,
        };
        assert!(guard.ring().is_none());
        assert!(guard.reload().is_none());
        // Debug impl exists and doesn't panic.
        let _ = format!("{guard:?}");
    }

    /// Confirm the public exports compile through `crate::*` so consumers
    /// can write `use observability::{init_node, init_lambda, ...}`.
    #[test]
    fn public_exports_resolve() {
        let _: fn(&'static str, &str) -> Result<ObsGuard, ObsError> = init_node;
        let _: fn(&'static str, &str) -> Result<ObsGuard, ObsError> = init_lambda;
        let _: fn(&'static str, &str) -> Result<ObsGuard, ObsError> = init_cli;
        // init_tauri is generic over H; bind it to () for the type-check.
        let _: fn(&'static str, &str, &()) -> Result<ObsGuard, ObsError> = init_tauri::<()>;
    }
}
