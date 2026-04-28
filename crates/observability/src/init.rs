//! Initialization helpers for each runtime target (node / Lambda / Tauri / CLI).
//!
//! Each helper composes the FMT + RELOAD + RING layers into a `Registry`,
//! installs a no-op [`opentelemetry_sdk::trace::TracerProvider`] so every
//! span carries a real W3C `trace_id` / `span_id` (lifted into RING entries
//! and Sentry tags), optionally adds the ERROR-only Sentry sink when
//! [`OBS_SENTRY_DSN`](crate::layers::error::OBS_SENTRY_DSN_ENV) is set, and
//! installs the result as the global tracing subscriber. The returned
//! [`ObsGuard`] holds the non-blocking writer's worker handle, the
//! [`RingHandle`] / [`ReloadHandle`] used by the rest of the binary, and the
//! Sentry shutdown handle whose `Drop` flushes any pending event queue.
//!
//! ## Per-target shape
//!
//! | helper        | FMT target               | RELOAD | RING | Sentry |
//! |---------------|--------------------------|--------|------|--------|
//! | [`init_node`]   | `~/.folddb/observability.jsonl` (or `OBS_FILE_PATH`) | yes | yes | env-gated |
//! | [`init_lambda`] | stdout                   | yes    | no   | env-gated |
//! | [`init_tauri`]  | inherits from embedded server, else delegates to [`init_node`] | conditional | conditional | inherits |
//! | [`init_cli`]    | stderr                   | no     | no   | no    |
//!
//! ## CLI is intentionally bare
//!
//! The Sentry transport amortizes its per-batch cost over many events. CLI
//! processes are short-lived one-shots: setup cost dominates and the
//! transport's batched flushes would not fire before the process exits. We
//! deliberately omit the Sentry layer from [`init_cli`] rather than ship
//! events that never flush.
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
//! - `service_name` must be non-empty (whitespace-only is also rejected).
//!   A bad value panics with a message containing `service.name` — this is a
//!   programming error, not a runtime one. The same rule applies to every
//!   `init_*` helper.
//! - After `init_*` returns successfully, the global TracerProvider's
//!   `Resource` is guaranteed to carry a `service.name` attribute equal to
//!   the input. The post-build verification in [`build_noop_traces_layer`]
//!   panics if that invariant is ever violated, and [`installed_service_name`]
//!   returns the verified value for inspection (used by integration tests).
//! - The returned [`ObsGuard`] **must** be held for the lifetime of the
//!   binary. Dropping it stops the FMT worker thread mid-flush and triggers
//!   Sentry shutdown; any log lines or events still in flight after that
//!   point are lost.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{fs, io};

use once_cell::sync::OnceCell;
use opentelemetry::global;
use opentelemetry::trace::{TraceResult, TracerProvider as _};
use opentelemetry::{Context, Key, KeyValue};
use opentelemetry_sdk::export::trace::SpanData;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{
    Sampler, Span as SdkSpan, SpanProcessor, Tracer, TracerProvider as SdkTracerProvider,
};
use opentelemetry_sdk::Resource;
use tracing_log::LogTracer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

use crate::layers::error::{build_error_layer, SentryGuard};
use crate::layers::fmt::{build_fmt_writer, FmtGuard, FmtTarget, RedactingFormat};
use crate::layers::reload::{build_reload_layer, ReloadHandle};
use crate::layers::ring::{build_ring_layer, RingHandle, OBS_RING_CAPACITY};
use crate::ObsError;

/// Override for the node log file path. Read once per `init_node` call.
const OBS_FILE_PATH_ENV: &str = "OBS_FILE_PATH";

/// Process-global guard against double init. Set on the first successful
/// `init_*` call; remains set for the lifetime of the process.
static INIT_ONCE: OnceCell<()> = OnceCell::new();

/// Process-global cache of the `service.name` that the most recent successful
/// `init_*` call committed to. Set after the post-build Resource verification
/// passes. Exposed via [`installed_service_name`] for tests and operators that
/// need to confirm what the observability stack ultimately advertised.
static SERVICE_NAME: OnceCell<&'static str> = OnceCell::new();

/// Returns the `service.name` that was passed to the most recent successful
/// `init_*` helper, or `None` if no helper has run yet (or the only call
/// panicked before claiming the slot).
///
/// Used by integration tests to assert that the value flowing through
/// `init_*` matches the value the global TracerProvider's Resource ended up
/// carrying.
pub fn installed_service_name() -> Option<&'static str> {
    SERVICE_NAME.get().copied()
}

/// RAII handle returned by every `init_*` helper.
///
/// Holds:
/// - the FMT layer's [`tracing_appender::non_blocking`] worker guard (when
///   this process did the install) so the background flush thread keeps
///   draining the queue,
/// - the [`RingHandle`] for in-process `/api/logs` queries (when RING is
///   wired — currently only [`init_node`] / [`init_tauri`] full path),
/// - the [`ReloadHandle`] for runtime `EnvFilter` updates (when RELOAD is
///   wired — every helper except [`init_cli`]),
/// - the [`CloudShutdown`] bag holding the Sentry guard. `Drop` runs the
///   guard's flush so the binary's exit path drains any in-flight events.
///   Always present in shape; the inner field is `None` when the matching
///   env var was not set at init time.
#[must_use = "ObsGuard must be held for the lifetime of the binary or log lines may be dropped"]
pub struct ObsGuard {
    fmt_guard: Option<FmtGuard>,
    ring: Option<RingHandle>,
    reload: Option<ReloadHandle>,
    cloud_shutdown: Option<CloudShutdown>,
}

/// Bag of cloud-side shutdown handles. Dropped by [`ObsGuard`]'s `Drop`.
///
/// Currently holds only the Sentry guard. Kept as a struct (rather than a
/// bare `Option<SentryGuard>`) so the existing test surface that asserts
/// "shape is engaged when env var is set" stays meaningful and so future
/// distributed-client sinks can attach here without reshaping `ObsGuard`.
struct CloudShutdown {
    sentry_guard: Option<SentryGuard>,
}

impl CloudShutdown {
    fn empty() -> Self {
        Self { sentry_guard: None }
    }

    fn is_engaged(&self) -> bool {
        self.sentry_guard.is_some()
    }
}

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
            .field(
                "cloud_shutdown",
                &self.cloud_shutdown.as_ref().map(|s| s.is_engaged()),
            )
            .finish()
    }
}

impl Drop for ObsGuard {
    fn drop(&mut self) {
        // Field drops do the rest: SentryGuard's inner `ClientInitGuard`
        // flushes on Drop, FmtGuard's Drop drains the writer queue.
        let _ = self.cloud_shutdown.take();
    }
}

// ---------------------------------------------------------------------------
// Public init helpers
// ---------------------------------------------------------------------------

/// Initialize observability for a long-running node binary.
///
/// Layers (always wired): redacting JSON FMT writing to
/// `~/.folddb/observability.jsonl` (override with `OBS_FILE_PATH`) + RELOAD +
/// RING + a `tracing-opentelemetry` layer riding a no-op
/// [`opentelemetry_sdk::trace::TracerProvider`] that stamps W3C `trace_id` /
/// `span_id` onto every span.
///
/// Optional ERROR sink: when `OBS_SENTRY_DSN` is set, a per-layer-filtered
/// Sentry layer captures `tracing::error!` events and tags them with the
/// originating span's W3C ids.
///
/// Also installs the W3C [`TraceContextPropagator`] globally and the
/// `tracing-log` bridge so third-party `log::*` calls flow through the
/// subscriber.
pub fn init_node(service_name: &'static str, _version: &str) -> Result<ObsGuard, ObsError> {
    assert_service_name(service_name);
    try_claim_init(&INIT_ONCE)?;
    install_log_tracer();

    let path = default_node_log_path()?;
    let (writer, fmt_guard) = build_fmt_writer(FmtTarget::File(path))?;
    let (reload_layer, reload) = build_reload_layer::<Registry>(default_env_filter());
    let (ring_layer, ring) = build_ring_layer(OBS_RING_CAPACITY);

    let otel_layer = build_noop_traces_layer(service_name);

    let (error_layer, sentry_guard) = match build_error_layer() {
        Some((layer, guard)) => (Some(layer), Some(guard)),
        None => (None, None),
    };

    let fmt_layer = tracing_subscriber::fmt::layer()
        .event_format(RedactingFormat::from_env_with_service(service_name))
        .with_writer(writer);

    // RELOAD is innermost so its `S = Registry` type binding matches; the
    // remaining layers are generic over `S` and the compiler infers each one
    // from the composition site. By the time RING's `on_event` runs, OTel's
    // `on_new_span` has already attached `OtelData` to the parent span, so
    // RING's extension lookup finds the trace/span ids regardless of layer
    // ordering at this level. The Sentry layer goes last so its `event_span`
    // lookup sees the OtelData attached upstream.
    let subscriber = Registry::default()
        .with(reload_layer)
        .with(otel_layer)
        .with(fmt_layer)
        .with(ring_layer)
        .with(error_layer);
    install_subscriber(subscriber)?;
    install_globals();
    record_service_name(service_name);

    Ok(ObsGuard {
        fmt_guard: Some(fmt_guard),
        ring: Some(ring),
        reload: Some(reload),
        cloud_shutdown: Some(CloudShutdown { sentry_guard }),
    })
}

/// Initialize observability for an AWS Lambda handler.
///
/// Layers: redacting JSON FMT to stdout + RELOAD + the same no-op
/// TracerProvider as [`init_node`] + the env-gated Sentry layer. Lambda's own
/// log capture pipes stdout to CloudWatch, so a file appender would be wasted
/// IO. RING is omitted — Lambda invocations are too short-lived for an
/// in-process query buffer to be useful.
pub fn init_lambda(service_name: &'static str, _version: &str) -> Result<ObsGuard, ObsError> {
    assert_service_name(service_name);
    try_claim_init(&INIT_ONCE)?;
    install_log_tracer();

    let (writer, fmt_guard) = build_fmt_writer(FmtTarget::Stdout)?;
    let (reload_layer, reload) = build_reload_layer::<Registry>(default_env_filter());

    let otel_layer = build_noop_traces_layer(service_name);

    let (error_layer, sentry_guard) = match build_error_layer() {
        Some((layer, guard)) => (Some(layer), Some(guard)),
        None => (None, None),
    };

    let fmt_layer = tracing_subscriber::fmt::layer()
        .event_format(RedactingFormat::from_env())
        .with_writer(writer);

    let subscriber = Registry::default()
        .with(reload_layer)
        .with(otel_layer)
        .with(fmt_layer)
        .with(error_layer);
    install_subscriber(subscriber)?;
    install_globals();
    record_service_name(service_name);

    Ok(ObsGuard {
        fmt_guard: Some(fmt_guard),
        ring: None,
        reload: Some(reload),
        cloud_shutdown: Some(CloudShutdown { sentry_guard }),
    })
}

/// Initialize observability inside a Tauri shell.
///
/// The Tauri desktop app embeds a full fold_db server, which calls
/// [`init_node`] from `start_server()`. By the time the Tauri runtime
/// invokes this helper, the global subscriber is already installed — so we
/// detect that and return a degraded "attached" [`ObsGuard`] rather than
/// fail. When the embedded server has *not* run (e.g. dev shell pointed at
/// a remote server), we fall through to a full [`init_node`] install — which
/// includes the env-gated Sentry layer.
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
    install_log_tracer();

    if INIT_ONCE.get().is_some() {
        // Embedded server already initialized. We can't compose new layers
        // onto an installed global subscriber, so the Tauri shell rides on
        // top of the server's. Phase 3 will swap this for a real
        // tauri-plugin-log attachment.
        return Ok(ObsGuard {
            fmt_guard: None,
            ring: None,
            reload: None,
            cloud_shutdown: Some(CloudShutdown::empty()),
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
///
/// The Sentry layer is intentionally omitted. CLI processes are short-lived:
/// the Sentry transport's batched flushes amortize over many events, but a
/// CLI exits before the pipeline reaches its first flush — shipping it would
/// add startup cost with no observable benefit. Long-lived CLIs that need
/// remote error capture should use [`init_node`] instead.
pub fn init_cli(service_name: &'static str, _version: &str) -> Result<ObsGuard, ObsError> {
    assert_service_name(service_name);
    try_claim_init(&INIT_ONCE)?;
    install_log_tracer();

    let (writer, fmt_guard) = build_fmt_writer(FmtTarget::Stderr)?;
    let fmt_layer = tracing_subscriber::fmt::layer()
        .event_format(RedactingFormat::from_env())
        .with_writer(writer);

    let subscriber = Registry::default().with(fmt_layer);
    install_subscriber(subscriber)?;
    install_globals();
    record_service_name(service_name);

    Ok(ObsGuard {
        fmt_guard: Some(fmt_guard),
        ring: None,
        reload: None,
        cloud_shutdown: Some(CloudShutdown::empty()),
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Reject empty / whitespace-only `service_name` at the entry of every
/// `init_*` helper. The OTel `service.name` Resource attribute is the primary
/// dimension dashboards group by — landing in production with `service.name=""`
/// is a silent telemetry-loss hazard. We treat the bad input as a programming
/// error and panic immediately so the operator sees the failure at boot rather
/// than discovering it in their backend the next morning.
///
/// The panic message MUST include the literal `service.name` so callers can
/// match on it; integration tests rely on this string.
#[inline]
fn assert_service_name(name: &str) {
    assert!(
        !name.trim().is_empty(),
        "service.name must be non-empty (got {name:?})",
    );
}

/// Atomically claim the one-shot init slot. Returns
/// [`ObsError::AlreadyInitialized`] when another caller already set it.
fn try_claim_init(cell: &OnceCell<()>) -> Result<(), ObsError> {
    cell.set(()).map_err(|_| ObsError::AlreadyInitialized)
}

/// Stamp the verified `service_name` onto the process-global cache. Called
/// at the tail of every successful `init_*` (after subscriber install +
/// Resource verification). Subsequent calls — including legitimate ones from
/// nested helpers like [`init_tauri`] → [`init_node`] — silently no-op via
/// `OnceCell::set`'s "already set" semantics, which is the right behaviour:
/// `INIT_ONCE` already gates double-init.
fn record_service_name(service_name: &'static str) {
    let _ = SERVICE_NAME.set(service_name);
}

fn default_env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

/// Build a `tracing-opentelemetry` layer riding a no-op
/// [`SdkTracerProvider`] so every span gets a real W3C `trace_id` /
/// `span_id`. There is no exporter — the spans never leave the process — but
/// the ids are what the RING layer stamps onto entries and what the Sentry
/// layer attaches as tags, so they have to be real.
///
/// The constructed provider is configured with a `Resource` carrying
/// `service.name`, attached to a [`ResourceProbe`] that captures the resource
/// the SDK pushes into its processors at build time, and installed as the
/// global tracer provider. Before returning, the captured resource is read
/// back through the probe and the `service.name` attribute is asserted to
/// equal `service_name`. Any mismatch panics — the alternative is shipping
/// spans whose Sentry tags silently group under the wrong service, which is
/// far worse than a hard failure at boot.
fn build_noop_traces_layer<S>(service_name: &'static str) -> OpenTelemetryLayer<S, Tracer>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    let probe = ResourceProbe::new();
    let provider = SdkTracerProvider::builder()
        .with_sampler(Sampler::AlwaysOn)
        .with_resource(build_service_resource(service_name))
        .with_span_processor(probe.clone())
        .build();
    assert_service_name_resource(&probe, service_name);
    let _ = global::set_tracer_provider(provider.clone());
    let tracer = provider.tracer(service_name);
    tracing_opentelemetry::layer().with_tracer(tracer)
}

/// Build the OTel `Resource` carrying the canonical `service.name`
/// attribute. Centralised so any future expansion to `service.version` /
/// `deployment.environment` lands here.
fn build_service_resource(service_name: &str) -> Resource {
    Resource::new(vec![KeyValue::new(
        "service.name",
        service_name.to_string(),
    )])
}

/// Assert that the [`ResourceProbe`] captured a `Resource` whose
/// `service.name` attribute equals `expected`. Panics with a message
/// mentioning `service.name` on any failure mode (no resource captured,
/// missing key, wrong value).
fn assert_service_name_resource(probe: &ResourceProbe, expected: &str) {
    let resource = probe
        .captured()
        .expect("global TracerProvider Resource was never set during init — service.name missing");
    let value = resource
        .get(Key::from_static_str("service.name"))
        .expect("global TracerProvider Resource is missing the service.name attribute");
    let actual = value.as_str();
    assert_eq!(
        actual.as_ref(),
        expected,
        "service.name resource attribute mismatch: expected {expected:?}, got {actual:?}",
    );
}

/// SpanProcessor whose only job is to record the `Resource` that the SDK
/// pushes into it via [`SpanProcessor::set_resource`] during
/// `TracerProvider::build`. Used by [`build_noop_traces_layer`] as a probe
/// to confirm that the constructed provider's resource carries
/// `service.name`.
///
/// `on_start` / `on_end` are intentionally no-ops — the probe MUST NOT
/// perturb production span flow.
#[derive(Default, Clone)]
struct ResourceProbe {
    captured: Arc<Mutex<Option<Resource>>>,
}

impl ResourceProbe {
    fn new() -> Self {
        Self::default()
    }

    fn captured(&self) -> Option<Resource> {
        self.captured
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
}

impl std::fmt::Debug for ResourceProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceProbe")
            .field("captured", &self.captured().is_some())
            .finish()
    }
}

impl SpanProcessor for ResourceProbe {
    fn on_start(&self, _span: &mut SdkSpan, _cx: &Context) {}
    fn on_end(&self, _span: SpanData) {}
    fn force_flush(&self) -> TraceResult<()> {
        Ok(())
    }
    fn shutdown(&self) -> TraceResult<()> {
        Ok(())
    }
    fn set_resource(&mut self, resource: &Resource) {
        *self.captured.lock().unwrap_or_else(|p| p.into_inner()) = Some(resource.clone());
    }
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

/// Install the W3C text-map propagator. Idempotent — setting twice
/// overwrites. Called after `install_subscriber` succeeds.
fn install_globals() {
    global::set_text_map_propagator(TraceContextPropagator::new());
}

/// Wire the `log` → `tracing` bridge. Called BEFORE `set_global_default` so
/// any third-party `log::*` call between subscriber install and process exit
/// flows through tracing. Doing it first also means a `log::*!` emitted by
/// init code itself is captured rather than dropped.
///
/// `LogTracer::init` errors only when called twice in the same process;
/// that's expected for retries / multiple test cases and not actionable, so
/// we swallow the error.
fn install_log_tracer() {
    let _ = LogTracer::init();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "service.name")]
    fn empty_service_name_panics() {
        assert_service_name("");
    }

    /// Whitespace-only `service_name` would round-trip into the OTel
    /// `service.name` Resource attribute as a blank string — same dashboard
    /// hazard as an empty literal. Pin the rejection here.
    #[test]
    #[should_panic(expected = "service.name")]
    fn whitespace_service_name_panics() {
        assert_service_name("   \t\n");
    }

    #[test]
    fn non_empty_service_name_does_not_panic() {
        assert_service_name("ok");
    }

    /// `assert_service_name_resource` panics when the probe never received a
    /// Resource (i.e. `set_resource` never fired). The empty probe state is
    /// the realistic regression: a future TracerProvider builder change that
    /// stops calling `set_resource` on its processors would land here.
    #[test]
    #[should_panic(expected = "service.name")]
    fn assert_service_name_resource_panics_when_probe_empty() {
        let probe = ResourceProbe::new();
        assert_service_name_resource(&probe, "svc");
    }

    /// Probe captures the Resource pushed into it by the SDK during
    /// `TracerProvider::build` — so a built provider is enough to populate
    /// the probe and pass the assertion. This pins the wiring used by
    /// `build_noop_traces_layer`.
    #[test]
    fn assert_service_name_resource_passes_when_probe_captured_match() {
        let probe = ResourceProbe::new();
        let _provider = SdkTracerProvider::builder()
            .with_resource(build_service_resource("phase5-t5-pin"))
            .with_span_processor(probe.clone())
            .build();
        assert_service_name_resource(&probe, "phase5-t5-pin");
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
            cloud_shutdown: Some(CloudShutdown::empty()),
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

    /// Phase 3 / T7: `LogTracer::init` is wired so third-party `log::*` calls
    /// reach the active tracing subscriber. The fold_db crate has no log-crate
    /// dependency anymore, but transitive deps (reqwest, hyper, sled, …) still
    /// emit via `log::*`. This test pins the bridge in place: if anyone removes
    /// the LogTracer install, the captured event count drops to zero and the
    /// assertion fires.
    #[test]
    fn log_macros_route_through_tracing_via_log_tracer() {
        use std::sync::{Arc, Mutex};
        use tracing::field::{Field, Visit};
        use tracing::{Event, Subscriber};
        use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
        use tracing_subscriber::registry::{LookupSpan, Registry};

        #[derive(Default)]
        struct MessageVisitor {
            message: String,
        }
        impl Visit for MessageVisitor {
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.message = format!("{:?}", value);
                }
            }
        }
        #[derive(Clone, Default)]
        struct CaptureLayer {
            captured: Arc<Mutex<Vec<(String, String, String)>>>,
        }
        impl<S> Layer<S> for CaptureLayer
        where
            S: Subscriber + for<'a> LookupSpan<'a>,
        {
            fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
                let meta = event.metadata();
                let mut visitor = MessageVisitor::default();
                event.record(&mut visitor);
                self.captured.lock().unwrap().push((
                    meta.target().to_string(),
                    meta.level().to_string(),
                    visitor.message,
                ));
            }
        }

        // Install the bridge — `init_log_tracer` is what every `init_*`
        // helper calls. It is idempotent across tests in the same process,
        // so calling it directly here is safe.
        install_log_tracer();

        let layer = CaptureLayer::default();
        let captured = layer.captured.clone();
        let subscriber = Registry::default().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            log::info!("bridged_log_tracer_marker info {}", 42);
            log::warn!("bridged_log_tracer_marker warn");
        });

        // LogTracer normalizes every bridged record to target=`"log"` and
        // stashes the original target in a field; assert by message content
        // so the test does not couple to that internal mapping.
        let entries = captured.lock().unwrap();
        let test_entries: Vec<_> = entries
            .iter()
            .filter(|(_, _, msg)| msg.contains("bridged_log_tracer_marker"))
            .collect();

        assert_eq!(
            test_entries.len(),
            2,
            "log::* calls did not reach the tracing subscriber — LogTracer bridge missing? captured={entries:?}",
        );
        assert_eq!(test_entries[0].1, "INFO");
        assert!(
            test_entries[0].2.contains("info 42"),
            "info message body wrong: {:?}",
            test_entries[0].2,
        );
        assert_eq!(test_entries[1].1, "WARN");
        assert!(
            test_entries[1].2.contains("warn"),
            "warn message body wrong: {:?}",
            test_entries[1].2,
        );
    }

    /// `CloudShutdown::empty()` is the no-op shape — every inner field is
    /// `None` — and dropping it does nothing dangerous (no panic, no
    /// shutdown attempt on absent providers).
    #[test]
    fn cloud_shutdown_empty_drops_cleanly() {
        let shutdown = CloudShutdown::empty();
        assert!(!shutdown.is_engaged());
        drop(shutdown);
    }

    /// Pin the Sentry env-var name against accidental renames that would
    /// silently change which env var binaries listen to.
    #[test]
    fn obs_sentry_env_constant_is_stable() {
        use crate::layers::error::OBS_SENTRY_DSN_ENV;
        assert_eq!(OBS_SENTRY_DSN_ENV, "OBS_SENTRY_DSN");
    }
}
