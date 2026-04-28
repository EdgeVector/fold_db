//! ERROR layer — promote `tracing::error!` events into Sentry issues.
//!
//! Phase 4 / T4. The layer wraps [`sentry_tracing::layer`] with its own
//! per-layer filter ([`FilterFn`]) so Sentry only ever sees ERROR-level
//! events, regardless of how loose the global RELOAD filter is.
//! Each captured Sentry event is enriched with the W3C `trace_id` and
//! `span_id` lifted off the parent span's [`OtelData`] extension (set by
//! `tracing-opentelemetry`) so an alert page can be cross-referenced against
//! local logs filtered by the same trace id.
//!
//! ## When the layer is wired
//!
//! [`build_error_layer`] returns `None` whenever `OBS_SENTRY_DSN` is unset,
//! making the layer a strict opt-in. When set, the layer takes ownership of a
//! [`sentry::ClientInitGuard`] (re-exported as [`SentryGuard`]) that the
//! caller must hold for the lifetime of the binary — dropping it flushes any
//! buffered events.
//!
//! ## Layer composition
//!
//! Sentry is one of several sinks the registry feeds. Composing with
//! `Layer::with_filter` keeps the ERROR-only filter local to this layer and
//! independent of the rest of the pipeline. Concretely, the node binary will
//! end up with:
//!
//! ```text
//! Registry::default()
//!     .with(reload_layer)        // global RELOAD filter (info/debug/...)
//!     .with(otel_layer)          // attaches OtelData -> spans
//!     .with(fmt_layer)           // JSONL on disk
//!     .with(ring_layer)          // /api/logs
//!     .with(web_layer)           // SSE fan-out
//!     .with(error_layer)         // <-- this layer, ERROR-only Sentry sink
//! ```

use std::env;

use sentry_tracing::EventMapping;
use tracing::{Level, Metadata, Subscriber};
use tracing_opentelemetry::OtelData;
use tracing_subscriber::filter::{filter_fn, FilterFn, Filtered};
use tracing_subscriber::layer::Layer;
use tracing_subscriber::registry::LookupSpan;

/// Environment variable that gates Sentry initialization. When unset (or
/// empty), [`build_error_layer`] returns `None` and the rest of the pipeline
/// runs unchanged.
pub const OBS_SENTRY_DSN_ENV: &str = "OBS_SENTRY_DSN";

/// RAII guard returned by [`build_error_layer`].
///
/// Wraps the [`sentry::ClientInitGuard`] that the SDK hands back from
/// [`sentry::init`]. Holding the guard keeps the Sentry transport alive;
/// dropping it triggers a final flush. Stored inside [`crate::ObsGuard`] so
/// the binary's existing lifetime contract carries over.
#[must_use = "SentryGuard must be held for the lifetime of the binary or buffered events may be dropped"]
pub struct SentryGuard {
    _client: sentry::ClientInitGuard,
}

impl std::fmt::Debug for SentryGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SentryGuard").finish_non_exhaustive()
    }
}

/// Concrete return type of [`build_error_layer`]. Wrapping `SentryLayer`
/// inside a [`Filtered`] keeps the ERROR-only filter scoped to this layer
/// (the global RELOAD filter is left alone) and lets callers compose the
/// result with `.with(...)` without naming the layer's full type.
///
/// The wrapped filter is a [`FilterFn`] (not a plain `EnvFilter`) because
/// per-layer `EnvFilter("error")` would also hide INFO/DEBUG *spans* from
/// the layer — `Context::event_span` then returns `None` and we lose the
/// trace context. Letting all spans through but only enabling ERROR
/// *events* keeps `event_span` working while still gating Sentry to errors.
pub type ErrorLayer<S> = Filtered<sentry_tracing::SentryLayer<S>, FilterFn, S>;

/// Build the per-layer filter used by [`build_error_layer`]: pass every span
/// through (so trace context lookup succeeds in `on_event`) but only allow
/// `Level::ERROR` events to reach Sentry.
fn error_only_event_filter() -> FilterFn {
    filter_fn(|meta: &Metadata<'_>| {
        if meta.is_event() {
            *meta.level() == Level::ERROR
        } else {
            true
        }
    })
}

/// Build the Sentry ERROR-layer when `OBS_SENTRY_DSN` is set.
///
/// Returns `None` when the env var is unset or empty so callers can treat
/// "no DSN configured" as a clean no-op without branching on errors. When
/// set, the function:
///
/// 1. Calls [`sentry::init`] with `release = CARGO_PKG_VERSION`. The returned
///    [`sentry::ClientInitGuard`] is wrapped in [`SentryGuard`] for lifetime
///    management.
/// 2. Builds a [`sentry_tracing::SentryLayer`] with a custom `event_mapper`
///    that lifts `trace_id` / `span_id` from the parent span's [`OtelData`]
///    extension and attaches them to the outgoing event as Sentry tags.
/// 3. Wraps the layer in [`Layer::with_filter`] using
///    [`error_only_event_filter`] so only `tracing::error!` events ever
///    reach Sentry while leaving span tracking untouched (a per-layer
///    `EnvFilter("error")` would also hide INFO/DEBUG spans, which would
///    break `Context::event_span` and lose the trace context).
pub fn build_error_layer<S>() -> Option<(ErrorLayer<S>, SentryGuard)>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let dsn = match env::var(OBS_SENTRY_DSN_ENV) {
        Ok(v) if !v.is_empty() => v,
        _ => return None,
    };

    let options = sentry::ClientOptions {
        release: Some(env!("CARGO_PKG_VERSION").into()),
        ..Default::default()
    };
    let client = sentry::init((dsn, options));

    let layer = sentry_tracing::layer()
        .event_mapper(event_mapper_with_trace_context::<S>)
        .with_filter(error_only_event_filter());

    Some((layer, SentryGuard { _client: client }))
}

/// Custom `event_mapper` for [`sentry_tracing::SentryLayer`].
///
/// Every ERROR event is mapped to a [`sentry::protocol::Event`] (via the
/// crate's own [`sentry_tracing::event_from_event`] helper, which preserves
/// the standard message / target / fields layout). We then walk up to the
/// parent span via the `Context`, look up the [`OtelData`] extension that
/// `tracing-opentelemetry` attaches in `on_new_span`, and copy the W3C
/// `trace_id` / `span_id` onto the Sentry event as tags. When the span has
/// no `OtelData` (no OTel layer wired, or no parent span at all) we emit the
/// event without trace context — the event is still useful, it just won't
/// deep-link.
fn event_mapper_with_trace_context<S>(
    event: &tracing::Event<'_>,
    ctx: tracing_subscriber::layer::Context<'_, S>,
) -> EventMapping
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let mut sentry_event = sentry_tracing::event_from_event(event, Some(&ctx));

    if let Some(span_ref) = ctx.event_span(event) {
        let exts = span_ref.extensions();
        if let Some(otel_data) = exts.get::<OtelData>() {
            if let Some(trace_id) = otel_data.builder.trace_id {
                sentry_event
                    .tags
                    .insert("trace_id".to_string(), format!("{:032x}", trace_id));
            }
            if let Some(span_id) = otel_data.builder.span_id {
                sentry_event
                    .tags
                    .insert("span_id".to_string(), format!("{:016x}", span_id));
            }
        }
    }

    EventMapping::Event(sentry_event)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Mutex, OnceLock};

    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
    use tracing::subscriber::with_default;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    /// Serialize tests that touch the process-global `OBS_SENTRY_DSN` env var
    /// and the global Sentry hub. `cargo test` runs unit tests in parallel by
    /// default; without serialization, one test's `set_var` could be observed
    /// by another's `build_error_layer` call.
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_env_lock<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        f()
    }

    #[test]
    fn returns_none_when_dsn_unset() {
        with_env_lock(|| {
            let prev = std::env::var(OBS_SENTRY_DSN_ENV).ok();
            std::env::remove_var(OBS_SENTRY_DSN_ENV);

            let result = build_error_layer::<Registry>();
            assert!(
                result.is_none(),
                "build_error_layer must no-op when {OBS_SENTRY_DSN_ENV} is unset"
            );

            if let Some(v) = prev {
                std::env::set_var(OBS_SENTRY_DSN_ENV, v);
            }
        });
    }

    #[test]
    fn returns_none_when_dsn_empty() {
        with_env_lock(|| {
            let prev = std::env::var(OBS_SENTRY_DSN_ENV).ok();
            std::env::set_var(OBS_SENTRY_DSN_ENV, "");

            let result = build_error_layer::<Registry>();
            assert!(
                result.is_none(),
                "empty {OBS_SENTRY_DSN_ENV} must be treated like unset"
            );

            match prev {
                Some(v) => std::env::set_var(OBS_SENTRY_DSN_ENV, v),
                None => std::env::remove_var(OBS_SENTRY_DSN_ENV),
            }
        });
    }

    /// When the DSN is set, the layer composes into a `Registry` without
    /// panicking and the returned guard is `Some`. We use a syntactically
    /// valid placeholder DSN so `sentry::init` accepts it; the test transport
    /// captures any events so nothing leaves the process.
    #[test]
    fn returns_some_when_dsn_set_and_composes_in_registry() {
        with_env_lock(|| {
            let prev = std::env::var(OBS_SENTRY_DSN_ENV).ok();
            std::env::set_var(OBS_SENTRY_DSN_ENV, "https://public@o0.ingest.sentry.io/0");

            let (layer, _guard) = build_error_layer::<Registry>()
                .expect("build_error_layer must return Some when DSN set");

            // The composition itself is the assertion — `with_default` will
            // panic if the subscriber type doesn't satisfy the expected
            // bounds.
            let subscriber = Registry::default().with(layer);
            with_default(subscriber, || {
                tracing::info!("composed layer accepts events");
            });

            match prev {
                Some(v) => std::env::set_var(OBS_SENTRY_DSN_ENV, v),
                None => std::env::remove_var(OBS_SENTRY_DSN_ENV),
            }
        });
    }

    /// End-to-end: emit a `tracing::error!` inside an OpenTelemetry span and
    /// assert the captured Sentry event carries the correct `trace_id` tag.
    /// Uses `sentry::test::with_captured_events` so nothing hits the network.
    ///
    /// The sentry layer is constructed inline (rather than via
    /// [`build_error_layer`]) because `with_captured_events` already binds a
    /// test Sentry client, and inline construction lets the closure mapper
    /// pick up `S = Layered<OtelLayer, Registry>` via type inference. The
    /// real production path goes through `build_error_layer` and is exercised
    /// by `returns_some_when_dsn_set_and_composes_in_registry`.
    #[test]
    fn error_event_attaches_trace_id_tag() {
        with_env_lock(|| {
            let provider = SdkTracerProvider::builder().build();
            let tracer = provider.tracer("error-layer-test");
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

            let sentry_layer = sentry_tracing::layer()
                .event_mapper(event_mapper_with_trace_context)
                .with_filter(error_only_event_filter());

            let subscriber = Registry::default().with(otel_layer).with(sentry_layer);

            let captured_trace_id = std::cell::RefCell::new(String::new());
            let events = sentry::test::with_captured_events(|| {
                with_default(subscriber, || {
                    use opentelemetry::trace::TraceContextExt;
                    use tracing_opentelemetry::OpenTelemetrySpanExt;

                    let span = tracing::info_span!("unit-of-work");
                    let _enter = span.enter();
                    let trace_id_hex =
                        format!("{:032x}", span.context().span().span_context().trace_id());
                    *captured_trace_id.borrow_mut() = trace_id_hex;

                    tracing::error!("kaboom");
                });
            });

            assert_eq!(
                events.len(),
                1,
                "exactly one ERROR event must be captured by Sentry, got {} (events: {events:?})",
                events.len()
            );
            let event = &events[0];
            let trace_id_hex = captured_trace_id.borrow();
            assert_eq!(
                event.tags.get("trace_id"),
                Some(&*trace_id_hex),
                "captured Sentry event must carry the originating span's trace_id as a tag"
            );
            assert!(
                event.tags.get("span_id").is_some_and(|s| s.len() == 16),
                "span_id must be a 16-char hex string, got {:?}",
                event.tags.get("span_id")
            );
        });
    }

    /// Non-error events must be filtered out by the layer's per-layer
    /// `FilterFn` (events allowed only at `Level::ERROR`) — Sentry should
    /// never see them, even if the global RELOAD filter is wide open.
    #[test]
    fn non_error_events_are_filtered_out() {
        with_env_lock(|| {
            let sentry_layer = sentry_tracing::layer()
                .event_mapper(event_mapper_with_trace_context)
                .with_filter(error_only_event_filter());

            let subscriber = Registry::default().with(sentry_layer);

            let events = sentry::test::with_captured_events(|| {
                with_default(subscriber, || {
                    tracing::info!("info");
                    tracing::warn!("warn");
                });
            });

            assert!(
                events.is_empty(),
                "INFO/WARN must not reach Sentry, got: {events:?}"
            );
        });
    }
}
