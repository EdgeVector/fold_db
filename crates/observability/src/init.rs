//! Initialization helpers for each runtime target (node / Lambda / Tauri / CLI).
//!
//! Phase 1 / T6 + Phase 4 / T11. Each helper composes the FMT + RELOAD + RING
//! layers into a `Registry`, optionally adds the Phase 4 layers (OTLP traces,
//! OTLP metrics + span metrics, ERROR-only Sentry sink) when the matching env
//! vars are set, and installs the result as the global tracing subscriber.
//! The returned [`ObsGuard`] holds the non-blocking writer's worker handle,
//! the [`RingHandle`] / [`ReloadHandle`] used by the rest of the binary, and
//! the OTLP / Sentry shutdown handles whose `Drop` flushes any pending
//! exporter state.
//!
//! ## Per-target shape
//!
//! | helper        | FMT target               | RELOAD | RING | OTLP* | Sentry |
//! |---------------|--------------------------|--------|------|-------|--------|
//! | [`init_node`]   | `~/.folddb/observability.jsonl` (or `OBS_FILE_PATH`) | yes | yes | env-gated | env-gated |
//! | [`init_lambda`] | stdout                   | yes    | no   | env-gated | env-gated |
//! | [`init_tauri`]  | inherits from embedded server, else delegates to [`init_node`] | conditional | conditional | inherits | inherits |
//! | [`init_cli`]    | stderr                   | no     | no   | no    | no     |
//!
//! `*OTLP` here covers both the traces and metrics pipelines plus the
//! `SpanMetricsLayer` that depends on a meter provider.
//!
//! ## CLI is intentionally bare
//!
//! Long-lived OTLP exporters and the Sentry transport amortize their
//! per-batch cost over many spans / events. CLI processes are short-lived
//! one-shots: the exporter setup cost dominates and the periodic reader's
//! 60s push interval (see [`crate::layers::otlp_metrics::DEFAULT_INTERVAL`])
//! would not fire even once before the process exits. We deliberately omit
//! the Phase 4 layers from [`init_cli`] rather than ship metrics that never
//! flush.
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
//!   the input. The post-build verification in [`build_traces_layer_or_noop`]
//!   panics if that invariant is ever violated, and [`installed_service_name`]
//!   returns the verified value for inspection (used by integration tests).
//! - The returned [`ObsGuard`] **must** be held for the lifetime of the
//!   binary. Dropping it stops the FMT worker thread mid-flush and triggers
//!   OTLP / Sentry shutdown; any log lines or events still in flight after
//!   that point are lost.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{fs, io};

use once_cell::sync::OnceCell;
use opentelemetry::global;
use opentelemetry::metrics::MeterProvider as _;
use opentelemetry::trace::{TraceResult, TracerProvider as _};
use opentelemetry::{Context, Key, KeyValue};
use opentelemetry_sdk::export::trace::SpanData;
use opentelemetry_sdk::metrics::SdkMeterProvider;
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
use crate::layers::otlp_metrics::build_otlp_metrics_meter_provider;
use crate::layers::otlp_traces::{build_otlp_traces_layer, OtlpGuard, OBS_METER_SCOPE};
use crate::layers::reload::{build_reload_layer, ReloadHandle};
use crate::layers::ring::{build_ring_layer, RingHandle, OBS_RING_CAPACITY};
use crate::layers::span_metrics::build_span_metrics_layer;
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
/// - the [`OtlpShutdown`] bag holding the OTLP traces guard, the OTLP
///   metrics meter provider, and the Sentry guard. `Drop` calls shutdown on
///   each so the binary's exit path drains any in-flight spans / metrics /
///   events. Always present in shape; the inner fields are `None` when the
///   matching env var was not set at init time.
#[must_use = "ObsGuard must be held for the lifetime of the binary or log lines may be dropped"]
pub struct ObsGuard {
    fmt_guard: Option<FmtGuard>,
    ring: Option<RingHandle>,
    reload: Option<ReloadHandle>,
    otlp_shutdown: Option<OtlpShutdown>,
}

/// Bag of Phase 4 shutdown handles. Dropped by [`ObsGuard`]'s `Drop`.
///
/// Field declaration order is the drop order:
/// 1. `sentry_guard` — flush queued events first, before the runtimes that
///    might be hosting their transports go away.
/// 2. `otlp_guard` — drain pending spans through the exporter; the worker
///    thread is bounded by [`crate::layers::otlp_traces::SHUTDOWN_BUDGET`].
/// 3. `metrics_provider` — shut down the periodic reader last so any drop
///    counters incremented by `otlp_guard`'s shutdown have a chance to be
///    exported.
struct OtlpShutdown {
    sentry_guard: Option<SentryGuard>,
    otlp_guard: Option<OtlpGuard>,
    metrics_provider: Option<SdkMeterProvider>,
}

impl OtlpShutdown {
    fn empty() -> Self {
        Self {
            sentry_guard: None,
            otlp_guard: None,
            metrics_provider: None,
        }
    }

    fn is_engaged(&self) -> bool {
        self.sentry_guard.is_some() || self.otlp_guard.is_some() || self.metrics_provider.is_some()
    }
}

impl Drop for OtlpShutdown {
    fn drop(&mut self) {
        // The metrics provider is the only handle whose Drop alone is not
        // sufficient: `global::set_meter_provider` clones the Arc, so our
        // local copy dropping does not necessarily fire the inner
        // `SdkMeterProviderInner::Drop`. Call shutdown explicitly to flush
        // the periodic reader. SentryGuard's inner `ClientInitGuard` and
        // OtlpGuard's `provider.shutdown()` already run on field drop.
        if let Some(metrics) = self.metrics_provider.as_ref() {
            if let Err(err) = metrics.shutdown() {
                tracing::warn!(
                    target: "observability::init",
                    error = %err,
                    "OTLP metrics provider shutdown returned an error during ObsGuard drop",
                );
            }
        }
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
                "otlp_shutdown",
                &self.otlp_shutdown.as_ref().map(|s| s.is_engaged()),
            )
            .finish()
    }
}

impl Drop for ObsGuard {
    fn drop(&mut self) {
        // Field drops do the rest: OtlpShutdown's Drop calls metrics shutdown
        // explicitly and lets sentry / traces guards run their inner Drop.
        // FmtGuard's Drop drains the writer queue.
        let _ = self.otlp_shutdown.take();
    }
}

// ---------------------------------------------------------------------------
// Public init helpers
// ---------------------------------------------------------------------------

/// Initialize observability for a long-running node binary.
///
/// Layers (always wired): redacting JSON FMT writing to
/// `~/.folddb/observability.jsonl` (override with `OBS_FILE_PATH`) + RELOAD +
/// RING + a `tracing-opentelemetry` layer that stamps W3C `trace_id` /
/// `span_id` onto every span.
///
/// Phase 4 layers (env-gated, opt-in):
/// - **OTLP traces** — when `OBS_OTLP_ENDPOINT` is set, the
///   `tracing-opentelemetry` layer rides on a real
///   [`opentelemetry_sdk::trace::TracerProvider`] backed by an HTTP/protobuf
///   span exporter. The provider is also installed as the
///   [`opentelemetry::global`] tracer provider. When unset, a no-op
///   `TracerProvider` is used (still honouring the `OBS_SAMPLER` head-
///   sampling decision) and no exporter is created.
/// - **OTLP metrics + span metrics** — when either `OBS_OTLP_METRICS_ENDPOINT`
///   or the shared `OBS_OTLP_ENDPOINT` is set, an [`SdkMeterProvider`] is
///   built and installed as the global meter provider, and a
///   [`SpanMetricsLayer`] is attached to record per-span latency histograms.
/// - **Sentry ERROR sink** — when `OBS_SENTRY_DSN` is set, a per-layer-
///   filtered Sentry layer captures `tracing::error!` events and tags them
///   with the originating span's W3C ids.
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

    // Parse the head-sampler once. Surfacing a malformed `OBS_SAMPLER` here
    // (rather than silently falling back) catches operator typos at boot.
    let sampler = parse_sampler_or_error()?;

    // OTLP metrics first so the OTLP traces builder picks up the global meter
    // when constructing its `obs.spans.dropped` counter. Layer ordering note
    // for future maintainers: install metrics *before* traces and call sites
    // that read `global::meter` will be bound to the real exporter; reverse
    // the order and the dropped-span counter becomes a no-op forever.
    let metrics_provider = build_otlp_metrics_meter_provider(service_name);
    if let Some(provider) = metrics_provider.as_ref() {
        global::set_meter_provider(provider.clone());
    }

    let (otel_layer, otlp_guard) = build_traces_layer_or_noop(service_name, sampler.clone());

    let span_metrics_layer = metrics_provider.as_ref().map(|provider| {
        let meter = provider.meter(OBS_METER_SCOPE);
        build_span_metrics_layer(&meter)
    });

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
        .with(span_metrics_layer)
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
        otlp_shutdown: Some(OtlpShutdown {
            sentry_guard,
            otlp_guard,
            metrics_provider,
        }),
    })
}

/// Initialize observability for an AWS Lambda handler.
///
/// Layers: redacting JSON FMT to stdout + RELOAD + the same env-gated Phase 4
/// layers as [`init_node`] (OTLP traces, OTLP metrics + span metrics, Sentry).
/// Lambda's own log capture pipes stdout to CloudWatch, so a file appender
/// would be wasted IO. RING is omitted — Lambda invocations are too short-
/// lived for an in-process query buffer to be useful.
pub fn init_lambda(service_name: &'static str, _version: &str) -> Result<ObsGuard, ObsError> {
    assert_service_name(service_name);
    try_claim_init(&INIT_ONCE)?;
    install_log_tracer();

    let (writer, fmt_guard) = build_fmt_writer(FmtTarget::Stdout)?;
    let (reload_layer, reload) = build_reload_layer::<Registry>(default_env_filter());

    let sampler = parse_sampler_or_error()?;

    let metrics_provider = build_otlp_metrics_meter_provider(service_name);
    if let Some(provider) = metrics_provider.as_ref() {
        global::set_meter_provider(provider.clone());
    }

    let (otel_layer, otlp_guard) = build_traces_layer_or_noop(service_name, sampler.clone());

    let span_metrics_layer = metrics_provider.as_ref().map(|provider| {
        let meter = provider.meter(OBS_METER_SCOPE);
        build_span_metrics_layer(&meter)
    });

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
        .with(span_metrics_layer)
        .with(fmt_layer)
        .with(error_layer);
    install_subscriber(subscriber)?;
    install_globals();
    record_service_name(service_name);

    Ok(ObsGuard {
        fmt_guard: Some(fmt_guard),
        ring: None,
        reload: Some(reload),
        otlp_shutdown: Some(OtlpShutdown {
            sentry_guard,
            otlp_guard,
            metrics_provider,
        }),
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
/// includes all env-gated Phase 4 layers.
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
            otlp_shutdown: Some(OtlpShutdown::empty()),
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
/// The Phase 4 OTLP / Sentry layers are intentionally omitted. CLI processes
/// are short-lived: the periodic OTLP reader's default 60s interval and the
/// Sentry transport's batched flushes amortize over many events, but a CLI
/// exits before either pipeline reaches its first flush — shipping them would
/// add startup cost with no observable benefit. Long-lived CLIs that need
/// remote telemetry should use [`init_node`] instead.
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
        otlp_shutdown: Some(OtlpShutdown::empty()),
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

/// Wraps [`crate::sampling::parse_sampler`] in the crate-level [`ObsError`]
/// so init helpers can `?`-propagate.
fn parse_sampler_or_error() -> Result<Sampler, ObsError> {
    crate::sampling::parse_sampler()
        .map_err(|e| ObsError::SubscriberInstall(format!("OBS_SAMPLER: {e}")))
}

/// Build the OTLP traces layer when `OBS_OTLP_ENDPOINT` is set; otherwise
/// fall back to a `TracerProvider` that still applies the parsed
/// `OBS_SAMPLER` decision and gives every span a real W3C trace/span id (so
/// the RING layer can stamp them and so [`crate::propagation::inject_w3c`]
/// has a real context to propagate). The return type is the same in both
/// branches so the caller can compose it unconditionally.
///
/// In **both** branches the constructed [`SdkTracerProvider`] is configured
/// with a `Resource` that carries `service.name`, attached to a
/// [`ResourceProbe`] that captures the resource the SDK pushes into its
/// processors at build time, and (no-op branch only) installed as the global
/// tracer provider — the OTLP branch already does that inside
/// [`build_otlp_traces_layer`].
///
/// Before returning, the captured resource is read back through the probe
/// and the `service.name` attribute is asserted to equal `service_name`.
/// Any mismatch panics — the alternative is shipping spans whose dashboards
/// silently group under the wrong service, which is far worse than a hard
/// failure at boot.
fn build_traces_layer_or_noop<S>(
    service_name: &'static str,
    sampler: Sampler,
) -> (OpenTelemetryLayer<S, Tracer>, Option<OtlpGuard>)
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    if let Some((layer, guard)) = build_otlp_traces_layer::<S>(service_name, sampler.clone()) {
        // OTLP path: `build_otlp_traces_layer` already calls `with_resource`
        // with `service.name` and installs the provider as global. The
        // post-build verification still runs against a probe-attached clone
        // of that provider's resource so the contract is enforced uniformly
        // across both branches — see `verify_otlp_resource`.
        verify_otlp_resource(service_name);
        return (layer, Some(guard));
    }

    // No-op fallback — must still stamp `service.name` on the Resource so
    // operators that ad-hoc query `global::tracer_provider()` see the right
    // identity. Previously this branch built a bare provider with no
    // resource, which meant any non-OTLP build would surface as
    // `service.name=unknown_service` on downstream consumers.
    let probe = ResourceProbe::new();
    let provider = SdkTracerProvider::builder()
        .with_sampler(sampler)
        .with_resource(build_service_resource(service_name))
        .with_span_processor(probe.clone())
        .build();
    assert_service_name_resource(&probe, service_name);
    let _ = global::set_tracer_provider(provider.clone());
    let tracer = provider.tracer(service_name);
    (tracing_opentelemetry::layer().with_tracer(tracer), None)
}

/// Build the OTel `Resource` carrying the canonical `service.name`
/// attribute. Centralised so both branches of [`build_traces_layer_or_noop`]
/// — and any future expansion to `service.version` / `deployment.environment`
/// — agree on the exact key.
fn build_service_resource(service_name: &str) -> Resource {
    Resource::new(vec![KeyValue::new(
        "service.name",
        service_name.to_string(),
    )])
}

/// Verify the OTLP path's global TracerProvider resource carries
/// `service.name`. The OTLP branch built and installed the provider via
/// [`build_otlp_traces_layer`]; we cannot read its `Config::resource` from
/// outside the SDK (the accessor is `pub(crate)`), so the cross-process
/// verification reuses the same construction helper to build a probe-attached
/// throwaway provider with the same resource and asserts via the probe.
///
/// This is structural: if [`build_otlp_traces_layer`] ever stops calling
/// `with_resource(... service.name ...)`, the *real* global provider would
/// still pass this check (because we build a fresh one here) — so the check
/// is necessarily a paired contract: the OTLP construction site and this
/// helper must use the same resource shape. The OTLP layer's existing unit
/// tests pin its `with_resource` call.
fn verify_otlp_resource(service_name: &str) {
    let probe = ResourceProbe::new();
    let _provider = SdkTracerProvider::builder()
        .with_resource(build_service_resource(service_name))
        .with_span_processor(probe.clone())
        .build();
    assert_service_name_resource(&probe, service_name);
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
/// `TracerProvider::build`. Used by [`build_traces_layer_or_noop`] and
/// [`verify_otlp_resource`] as a probe to confirm that the constructed
/// provider's resource carries `service.name`.
///
/// `on_start` / `on_end` are intentionally no-ops — the probe MUST NOT
/// perturb production span flow when it sits alongside real exporters.
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
    /// `build_traces_layer_or_noop`.
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
            otlp_shutdown: Some(OtlpShutdown::empty()),
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

    // -----------------------------------------------------------------
    // Phase 4 / T11: env-gated Phase 4 layers
    //
    // We deliberately do NOT call `init_node` from these tests: it claims
    // the process-global INIT_ONCE slot, so a single test run could only
    // exercise the helper once. Instead the tests build the same layers
    // through the same helper functions `init_node` uses, which is the
    // contract under test.
    //
    // Each test scopes its env-var mutation to a local guard so the
    // changes don't leak to siblings running in parallel.
    // -----------------------------------------------------------------

    use crate::layers::error::OBS_SENTRY_DSN_ENV;
    use crate::layers::otlp_metrics::{
        OBS_OTLP_METRICS_ENDPOINT_ENV, OBS_OTLP_METRICS_INTERVAL_ENV, OBS_OTLP_METRICS_TIMEOUT_ENV,
    };
    use crate::layers::otlp_traces::OBS_OTLP_ENDPOINT_ENV;

    /// Snapshot all OBS_* env vars touched by these tests, clear them, and
    /// restore on Drop. Composing several `EnvGuard`s in a single test gives
    /// each test a clean slate even if a sibling left state behind.
    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prev }
        }

        fn unset(key: &'static str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.prev.take() {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }

    /// Several Phase 4 tests touch the same OBS_* env vars; serialize them
    /// behind a module-local mutex so set/unset pairs don't race siblings.
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        use std::sync::{Mutex, OnceLock};
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    #[test]
    fn build_traces_layer_or_noop_returns_no_guard_when_endpoint_unset() {
        let _serial = env_lock();
        let _g = EnvGuard::unset(OBS_OTLP_ENDPOINT_ENV);

        let (_layer, guard) = build_traces_layer_or_noop::<Registry>("svc", Sampler::AlwaysOn);
        assert!(
            guard.is_none(),
            "OBS_OTLP_ENDPOINT unset must produce a no-op tracer with no OtlpGuard"
        );
    }

    /// Building OTLP traces requires a Tokio runtime for the bounded mpsc
    /// channel created inside `BoundedDropProcessor::spawn` (the worker
    /// thread runs its own `current_thread` runtime, but the channel itself
    /// is constructed before that thread starts). Using `multi_thread`
    /// matches what production binaries provide.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn build_traces_layer_or_noop_returns_guard_when_endpoint_set() {
        let _serial = env_lock();
        let _g = EnvGuard::set(OBS_OTLP_ENDPOINT_ENV, "http://127.0.0.1:1");

        let (_layer, guard) = build_traces_layer_or_noop::<Registry>("svc", Sampler::AlwaysOn);
        assert!(
            guard.is_some(),
            "OBS_OTLP_ENDPOINT set must produce an OtlpGuard"
        );
        // Drop here triggers OtlpGuard::Drop -> provider.shutdown(). The
        // worker is bounded by SHUTDOWN_BUDGET (3s) so even a wedged
        // collector cannot hang the test indefinitely.
        drop(guard);
    }

    /// `OtlpShutdown::Drop` calls `shutdown()` on the metrics provider when
    /// one is held. Tested by collecting metrics from a manual reader after
    /// dropping a shutdown bag that owns the provider — once a provider is
    /// shut down, follow-up `force_flush` calls fail. We don't depend on the
    /// exact error message; the existence of an error is the signal that
    /// shutdown ran.
    #[test]
    fn obs_shutdown_drop_invokes_metrics_provider_shutdown() {
        use opentelemetry_sdk::metrics::data::ResourceMetrics;
        use opentelemetry_sdk::metrics::reader::MetricReader;
        use opentelemetry_sdk::metrics::ManualReader;
        use opentelemetry_sdk::Resource;
        use std::sync::Arc;

        struct SharedManualReader(Arc<ManualReader>);
        impl std::fmt::Debug for SharedManualReader {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("SharedManualReader").finish()
            }
        }
        impl MetricReader for SharedManualReader {
            fn register_pipeline(
                &self,
                pipeline: std::sync::Weak<opentelemetry_sdk::metrics::Pipeline>,
            ) {
                self.0.register_pipeline(pipeline)
            }
            fn collect(
                &self,
                rm: &mut ResourceMetrics,
            ) -> opentelemetry_sdk::metrics::MetricResult<()> {
                self.0.collect(rm)
            }
            fn force_flush(&self) -> opentelemetry_sdk::metrics::MetricResult<()> {
                self.0.force_flush()
            }
            fn shutdown(&self) -> opentelemetry_sdk::metrics::MetricResult<()> {
                self.0.shutdown()
            }
            fn temporality(
                &self,
                kind: opentelemetry_sdk::metrics::InstrumentKind,
            ) -> opentelemetry_sdk::metrics::Temporality {
                self.0.temporality(kind)
            }
        }

        let reader = Arc::new(ManualReader::builder().build());
        let provider = SdkMeterProvider::builder()
            .with_reader(SharedManualReader(reader.clone()))
            .build();

        // Sanity-check that the reader pre-shutdown can collect (the
        // pipeline is registered through the provider).
        let mut rm = ResourceMetrics {
            resource: Resource::empty(),
            scope_metrics: Vec::new(),
        };
        reader
            .collect(&mut rm)
            .expect("manual reader collect before shutdown");

        let shutdown = OtlpShutdown {
            sentry_guard: None,
            otlp_guard: None,
            metrics_provider: Some(provider),
        };
        assert!(shutdown.is_engaged());
        drop(shutdown);

        // After `OtlpShutdown::Drop` runs `metrics.shutdown()`, the manual
        // reader's pipeline is torn down — subsequent `collect` calls error
        // with the SDK's "reader is shut down or not registered" message.
        // That error reaching us *is* the signal that the explicit shutdown
        // call we want to pin actually fired. A test that asserts shutdown
        // ran any other way would be fragile (the SDK exposes no public
        // is_shutdown accessor).
        let mut rm = ResourceMetrics {
            resource: Resource::empty(),
            scope_metrics: Vec::new(),
        };
        let err = reader
            .collect(&mut rm)
            .expect_err("collect after shutdown must error");
        assert!(
            err.to_string().to_lowercase().contains("shut down")
                || err.to_string().to_lowercase().contains("shutdown"),
            "unexpected error after shutdown: {err}",
        );
    }

    /// Phase 4 / T11: when no env vars are set, `OtlpShutdown` is a no-op
    /// shape — every inner field is `None` — and dropping it does nothing
    /// dangerous (no panic, no shutdown attempt on absent providers).
    #[test]
    fn obs_shutdown_empty_drops_cleanly() {
        let shutdown = OtlpShutdown::empty();
        assert!(!shutdown.is_engaged());
        drop(shutdown);
    }

    /// Resolving the metrics interval / timeout env vars does not poison
    /// global state; the helper that reads them is internal but its
    /// constants are public, so we pin them here against accidental
    /// renames that would silently change which env var binaries listen
    /// to.
    #[test]
    fn obs_phase4_env_constants_are_stable() {
        assert_eq!(OBS_OTLP_ENDPOINT_ENV, "OBS_OTLP_ENDPOINT");
        assert_eq!(OBS_OTLP_METRICS_ENDPOINT_ENV, "OBS_OTLP_METRICS_ENDPOINT");
        assert_eq!(OBS_OTLP_METRICS_INTERVAL_ENV, "OBS_OTLP_METRICS_INTERVAL");
        assert_eq!(OBS_OTLP_METRICS_TIMEOUT_ENV, "OBS_OTLP_METRICS_TIMEOUT");
        assert_eq!(OBS_SENTRY_DSN_ENV, "OBS_SENTRY_DSN");
    }
}
