//! OTLP METRICS layer — exports OTel meter readings to a collector over
//! HTTP/protobuf via a periodic push.
//!
//! Phase 4 / T3. Companion to the [`otlp_traces`](super::otlp_traces) layer:
//! T1 ships span data, this ships everything created off
//! [`opentelemetry::global::meter`] — including the
//! [`obs.spans.dropped`](super::otlp_traces::OBS_SPANS_DROPPED_METRIC) self-
//! monitoring counter that the traces layer publishes.
//!
//! ## "OTLP off" is the default
//!
//! Like T1, the layer is gated on env vars. Returns `None` when neither
//! [`OBS_OTLP_METRICS_ENDPOINT_ENV`] nor the shared
//! [`super::otlp_traces::OBS_OTLP_ENDPOINT_ENV`] is set, so binaries can
//! always call [`build_otlp_metrics_meter_provider`] unconditionally and
//! only opt in when the operator configures a collector.
//!
//! ## Non-blocking contract
//!
//! Metrics are very different from spans on the hot path: instrument
//! recording (`counter.add`, `histogram.record`) is an in-memory aggregation
//! and never touches the network. The "drop on saturation" pattern that T1
//! needs for `on_end` does not apply here — there is no per-event send.
//!
//! What we *do* care about is:
//! - the periodic export call cannot hang the metrics SDK forever on a
//!   wedged collector — bounded by [`OBS_OTLP_METRICS_TIMEOUT_ENV`]
//!   (default [`DEFAULT_TIMEOUT`]);
//! - the periodic interval is short enough for operators to see fresh data
//!   in Honeycomb without overloading the collector — bounded by
//!   [`OBS_OTLP_METRICS_INTERVAL_ENV`] (default [`DEFAULT_INTERVAL`]).
//!
//! [`PeriodicReader`] handles the worker loop and an internal bounded
//! `mpsc(256)` channel. We just feed it an [`MetricExporter`] and a runtime.
//!
//! ## Runtime selection
//!
//! [`PeriodicReader`] uses [`opentelemetry_sdk::runtime::Tokio`], which means
//! the caller must install this provider from inside a Tokio runtime. Every
//! fold_db binary already has one (actix, tauri, lambda runtime), so this
//! mirrors the existing constraint placed by the OTLP traces layer.

use std::time::Duration;

use opentelemetry::KeyValue;
use opentelemetry_otlp::{MetricExporter, Protocol, WithExportConfig};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::Resource;

/// Primary env var that gates the OTLP metrics exporter when no metrics-
/// specific override is set. Shared with [`super::otlp_traces`] so a single
/// "OTLP on" toggle lights up both pipelines.
pub const OBS_OTLP_ENDPOINT_ENV: &str = "OBS_OTLP_ENDPOINT";

/// Per-signal override. Set this to ship metrics to a different collector
/// than traces (e.g. an in-region cardinality-aware metrics gateway while
/// traces fan out direct to Honeycomb).
pub const OBS_OTLP_METRICS_ENDPOINT_ENV: &str = "OBS_OTLP_METRICS_ENDPOINT";

/// Periodic export interval, in milliseconds. Operator override of the
/// default. Fed straight to [`PeriodicReaderBuilder::with_interval`].
///
/// [`PeriodicReaderBuilder::with_interval`]: opentelemetry_sdk::metrics::PeriodicReaderBuilder::with_interval
pub const OBS_OTLP_METRICS_INTERVAL_ENV: &str = "OBS_OTLP_METRICS_INTERVAL";

/// Per-export wall-clock timeout, in milliseconds. Caps how long the
/// [`PeriodicReader`] will wait on a single push before cancelling, so a
/// wedged collector cannot indefinitely starve the next interval.
pub const OBS_OTLP_METRICS_TIMEOUT_ENV: &str = "OBS_OTLP_METRICS_TIMEOUT";

/// Default push interval. Matches the OTel SDK's stock 60s — short enough
/// for a 1-minute-resolution dashboard, long enough that the export loop is
/// not a perceptible CPU/network user.
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(60);

/// Default per-export timeout. Half the default interval — leaves headroom
/// for a retry inside the same interval while bounding worst-case stall on
/// a wedged collector.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Build an OTLP-bound [`SdkMeterProvider`].
///
/// Returns `None` when neither [`OBS_OTLP_METRICS_ENDPOINT_ENV`] nor
/// [`OBS_OTLP_ENDPOINT_ENV`] is set (or both are empty / whitespace) — that's
/// the "OTLP off" state, not a failure.
///
/// Returns `None` on exporter construction error too: at startup we'd rather
/// run without metrics export than crash the binary because of a malformed
/// collector URL. The error is logged via `tracing::error!`.
pub fn build_otlp_metrics_meter_provider(service_name: &str) -> Option<SdkMeterProvider> {
    let endpoint = resolve_endpoint()?;

    let exporter = match MetricExporter::builder()
        .with_http()
        .with_endpoint(&endpoint)
        .with_protocol(Protocol::HttpBinary)
        .build()
    {
        Ok(e) => e,
        Err(err) => {
            tracing::error!(
                target: "observability::otlp_metrics",
                error = %err,
                endpoint = %endpoint,
                "failed to construct OTLP metric exporter; OTLP metrics disabled",
            );
            return None;
        }
    };

    let reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(resolve_duration_ms(
            OBS_OTLP_METRICS_INTERVAL_ENV,
            DEFAULT_INTERVAL,
        ))
        .with_timeout(resolve_duration_ms(
            OBS_OTLP_METRICS_TIMEOUT_ENV,
            DEFAULT_TIMEOUT,
        ))
        .build();

    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            service_name.to_string(),
        )]))
        .build();

    Some(provider)
}

/// Pick the metrics-specific endpoint if set, else fall back to the shared
/// `OBS_OTLP_ENDPOINT`. Whitespace-only values are treated as unset so that
/// `OBS_OTLP_ENDPOINT=""` reliably means "off".
fn resolve_endpoint() -> Option<String> {
    let from_metrics = std::env::var(OBS_OTLP_METRICS_ENDPOINT_ENV).ok();
    let from_shared = std::env::var(OBS_OTLP_ENDPOINT_ENV).ok();
    [from_metrics, from_shared]
        .into_iter()
        .flatten()
        .map(|v| v.trim().to_string())
        .find(|v| !v.is_empty())
}

/// Read a millisecond-valued env var, falling back to `default` if unset,
/// unparseable, or zero. Zero is treated as "use the default" because
/// [`PeriodicReaderBuilder`] silently ignores zero anyway and we'd rather
/// surface a single canonical default in tracing logs.
///
/// [`PeriodicReaderBuilder`]: opentelemetry_sdk::metrics::PeriodicReaderBuilder
fn resolve_duration_ms(env_key: &str, default: Duration) -> Duration {
    match std::env::var(env_key).ok().and_then(|v| v.parse::<u64>().ok()) {
        Some(0) | None => default,
        Some(ms) => Duration::from_millis(ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    /// Cargo runs unit tests in parallel within a single process. Anything
    /// that mutates the OBS_* env vars must serialize through this lock or
    /// it races with sibling tests in the module.
    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    /// RAII guard that snapshots an env var, lets the test mutate it, and
    /// restores the previous value (or unsets) on Drop. Without this, a
    /// failing test that early-exits leaks state to siblings.
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

    #[test]
    fn returns_none_when_no_endpoint_env_set() {
        let _serial = env_lock();
        let _g1 = EnvGuard::unset(OBS_OTLP_ENDPOINT_ENV);
        let _g2 = EnvGuard::unset(OBS_OTLP_METRICS_ENDPOINT_ENV);
        let provider = build_otlp_metrics_meter_provider("svc");
        assert!(
            provider.is_none(),
            "must be a no-op when neither endpoint env var is set",
        );
    }

    #[test]
    fn returns_none_when_endpoints_are_empty_or_whitespace() {
        let _serial = env_lock();
        let _g1 = EnvGuard::set(OBS_OTLP_ENDPOINT_ENV, "");
        let _g2 = EnvGuard::set(OBS_OTLP_METRICS_ENDPOINT_ENV, "   ");
        let provider = build_otlp_metrics_meter_provider("svc");
        assert!(
            provider.is_none(),
            "whitespace / empty endpoints must be treated as unset",
        );
    }

    #[tokio::test]
    async fn returns_some_with_metrics_specific_endpoint() {
        // PeriodicReader::build spawns onto opentelemetry_sdk::runtime::Tokio,
        // which requires an ambient runtime. The exporter constructs lazily
        // — a failed connect later is the worker's problem, not the build
        // path's. Point at an unused TCP port.
        let _serial = env_lock();
        let _g1 = EnvGuard::unset(OBS_OTLP_ENDPOINT_ENV);
        let _g2 = EnvGuard::set(OBS_OTLP_METRICS_ENDPOINT_ENV, "http://127.0.0.1:1");
        let provider = build_otlp_metrics_meter_provider("svc");
        assert!(provider.is_some(), "metrics-specific endpoint must build");
        // Shutdown is best-effort: drop ordering is what matters for the
        // bin's lifecycle, not the result of this call.
        if let Some(p) = provider {
            let _ = p.shutdown();
        }
    }

    #[tokio::test]
    async fn metrics_endpoint_overrides_shared_endpoint() {
        // When both are set, the metrics-specific value wins. The visible
        // signal is just "did it build" — but combined with the shared-only
        // test below this pins ordering: metrics > shared.
        let _serial = env_lock();
        let _g1 = EnvGuard::set(OBS_OTLP_ENDPOINT_ENV, "   "); // ignored
        let _g2 = EnvGuard::set(OBS_OTLP_METRICS_ENDPOINT_ENV, "http://127.0.0.1:1");
        let provider = build_otlp_metrics_meter_provider("svc");
        assert!(
            provider.is_some(),
            "metrics endpoint must override empty shared endpoint",
        );
        if let Some(p) = provider {
            let _ = p.shutdown();
        }
    }

    #[tokio::test]
    async fn falls_back_to_shared_endpoint_when_metrics_unset() {
        let _serial = env_lock();
        let _g1 = EnvGuard::set(OBS_OTLP_ENDPOINT_ENV, "http://127.0.0.1:1");
        let _g2 = EnvGuard::unset(OBS_OTLP_METRICS_ENDPOINT_ENV);
        let provider = build_otlp_metrics_meter_provider("svc");
        assert!(
            provider.is_some(),
            "shared endpoint must be the fallback when metrics-specific is unset",
        );
        if let Some(p) = provider {
            let _ = p.shutdown();
        }
    }

    #[test]
    fn resolve_duration_ms_uses_default_for_unset_or_zero_or_garbage() {
        // No env lock — the values we set are read synchronously inside the
        // function, and we use a unique key per case to avoid sibling races.
        let key = "OBS_OTLP_METRICS_TEST_DURATION_RESOLVE";
        let _g = EnvGuard::unset(key);
        assert_eq!(resolve_duration_ms(key, DEFAULT_INTERVAL), DEFAULT_INTERVAL);

        let _g = EnvGuard::set(key, "0");
        assert_eq!(resolve_duration_ms(key, DEFAULT_INTERVAL), DEFAULT_INTERVAL);

        let _g = EnvGuard::set(key, "not-a-number");
        assert_eq!(resolve_duration_ms(key, DEFAULT_INTERVAL), DEFAULT_INTERVAL);

        let _g = EnvGuard::set(key, "1500");
        assert_eq!(
            resolve_duration_ms(key, DEFAULT_INTERVAL),
            Duration::from_millis(1500),
        );
    }
}
