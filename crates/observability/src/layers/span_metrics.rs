//! SPAN METRICS layer — translate `tracing` span lifetimes into OTLP-bound
//! latency histograms keyed by span name.
//!
//! Phase 4 / T2. The trace export pipeline (T1) already ships span data to a
//! collector, but per-span latency histograms are far cheaper to query than
//! aggregating spans on the read side. This layer pre-registers a fixed list
//! of span names, watches for any span whose name matches, and on `on_close`
//! records `(closed_at − entered_at)` in milliseconds against a low-cardinality
//! attribute set.
//!
//! ## What "pre-registered" means
//!
//! Adding a new metric is a two-line change: append the span name to
//! [`PRE_REGISTERED_SPANS`] and wrap the corresponding code path in a
//! `tracing::span!` with that name. Spans whose name is *not* in the list are
//! ignored — that keeps unbounded ad-hoc spans from accidentally creating
//! unbounded metric series.
//!
//! ## Cardinality
//!
//! Histogram attributes are limited to an allowlist:
//! `service.name`, `http.method`, `http.route`. Per-tenant or per-user fields
//! (e.g. `user.hash`) are *deliberately* dropped — they would explode the
//! time-series index in the collector and the downstream TSDB. Adding a new
//! allowed label requires editing [`ALLOWED_LABEL_KEYS`].
//!
//! ## Timing source
//!
//! The layer captures `Instant::now()` at `on_new_span` rather than `on_enter`.
//! `on_enter` fires every time the span is entered (which can happen multiple
//! times for a span across `.in_scope()` blocks or async polls), but the
//! semantically interesting duration is creation → close. `Instant` is
//! monotonic, so it's safe across clock jumps; we deliberately don't use
//! `SystemTime::now()` here.

use std::collections::HashMap;
use std::time::Instant;

use opentelemetry::metrics::{Histogram, Meter};
use opentelemetry::KeyValue;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

/// Span names that the layer records into pre-registered histograms. Spans
/// whose name is not in this list are ignored entirely — they do not create
/// new metric series.
pub const PRE_REGISTERED_SPANS: &[&str] = &[
    "http.server.request",
    "db.sled.put",
    "db.sled.get",
    "db.sled.scan",
    "wasm.transform.execute",
    "lambda.handler.invoke",
    "schema_service.register",
];

/// Attribute keys (canonical OTel-semantic dot-form) that the layer copies
/// from span fields onto histogram observations. Anything else is dropped to
/// keep the time-series cardinality bounded.
pub const ALLOWED_LABEL_KEYS: &[&str] = &["service.name", "http.method", "http.route"];

/// `tracing` field names cannot contain dots when written via the macro
/// shorthand (`tracing::info_span!("...", http.method = ...)` parses
/// `http.method` as a path expression on the `http` module). Instrumentation
/// sites therefore use underscore-form names; this map translates them to the
/// canonical dot-form before recording. Both forms are accepted: a caller who
/// uses the literal string syntax (`r#"http.method"# = ...`) still works.
const LABEL_ALIASES: &[(&str, &str)] = &[
    ("service_name", "service.name"),
    ("service.name", "service.name"),
    ("http_method", "http.method"),
    ("http.method", "http.method"),
    ("http_route", "http.route"),
    ("http.route", "http.route"),
];

/// Per-span timing extension. Captured at `on_new_span`, read at `on_close`.
/// `Instant` is monotonic so the elapsed math is safe across wall-clock jumps.
#[derive(Debug)]
struct Timing {
    entered_at: Instant,
}

/// Per-span allowlisted labels extension. Populated incrementally — fields
/// can be supplied at span construction (`Attributes`) or later via
/// `Span::record` (`Record`).
#[derive(Debug, Default)]
struct LabelBag {
    labels: HashMap<&'static str, String>,
}

impl LabelBag {
    fn record_visited(&mut self, mut visitor: LabelVisitor) {
        for (canonical, value) in visitor.drain() {
            self.labels.insert(canonical, value);
        }
    }

    fn into_keyvalues(self) -> Vec<KeyValue> {
        self.labels
            .into_iter()
            .map(|(k, v)| KeyValue::new(k, v))
            .collect()
    }
}

/// Subscriber layer that records each pre-registered span's lifetime into a
/// pre-built histogram. Returned by [`build_span_metrics_layer`].
pub struct SpanMetricsLayer {
    histograms: HashMap<&'static str, Histogram<f64>>,
}

impl SpanMetricsLayer {
    /// Names of pre-registered histograms — primarily for tests / diagnostics.
    pub fn registered_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self.histograms.keys().copied().collect();
        names.sort();
        names
    }
}

/// Build a [`SpanMetricsLayer`] backed by `meter`. All histograms in
/// [`PRE_REGISTERED_SPANS`] are constructed up front: the layer itself is
/// allocation-free on the hot path and never falls back to creating new
/// instruments at runtime, which avoids surprise cardinality explosions if a
/// span name typo were ever to slip past code review.
pub fn build_span_metrics_layer(meter: &Meter) -> SpanMetricsLayer {
    let mut histograms = HashMap::with_capacity(PRE_REGISTERED_SPANS.len());
    for &name in PRE_REGISTERED_SPANS {
        let hist = meter
            .f64_histogram(name)
            .with_unit("ms")
            .with_description("Span duration in milliseconds (fold_db span_metrics layer)")
            .build();
        histograms.insert(name, hist);
    }
    SpanMetricsLayer { histograms }
}

impl<S> Layer<S> for SpanMetricsLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        if !self.histograms.contains_key(span.name()) {
            return;
        }

        let mut exts = span.extensions_mut();
        // `ExtensionsMut::insert` panics on duplicate; `replace` doesn't and
        // returns the previous value. We only initialize Timing once — subsequent
        // span re-creations through follows-from links would be a logic bug
        // upstream, but we tolerate it gracefully.
        if exts.get_mut::<Timing>().is_none() {
            let _ = exts.replace(Timing {
                entered_at: Instant::now(),
            });
        }

        let mut visitor = LabelVisitor::default();
        attrs.record(&mut visitor);
        if !visitor.is_empty() {
            if let Some(existing) = exts.get_mut::<LabelBag>() {
                existing.record_visited(visitor);
            } else {
                let mut new_bag = LabelBag::default();
                new_bag.record_visited(visitor);
                let _ = exts.replace(new_bag);
            }
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        if !self.histograms.contains_key(span.name()) {
            return;
        }

        let mut visitor = LabelVisitor::default();
        values.record(&mut visitor);
        if visitor.is_empty() {
            return;
        }

        let mut exts = span.extensions_mut();
        if let Some(existing) = exts.get_mut::<LabelBag>() {
            existing.record_visited(visitor);
        } else {
            let mut bag = LabelBag::default();
            bag.record_visited(visitor);
            let _ = exts.replace(bag);
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else { return };
        let Some(hist) = self.histograms.get(span.name()) else {
            return;
        };

        let mut exts = span.extensions_mut();
        // No `Timing` means we never saw `on_new_span` for this span — that
        // only happens if the layer is wired up *after* spans already exist.
        // Skip rather than record a bogus zero-duration measurement.
        let timing = match exts.remove::<Timing>() {
            Some(t) => t,
            None => return,
        };
        let elapsed_ms = timing.entered_at.elapsed().as_secs_f64() * 1_000.0;
        let labels = exts
            .remove::<LabelBag>()
            .map(|bag| bag.into_keyvalues())
            .unwrap_or_default();
        drop(exts);

        hist.record(elapsed_ms, &labels);
    }
}

/// Visitor that collects only allowlisted span fields, normalizing each to
/// the canonical OTel dot-form key.
#[derive(Default)]
struct LabelVisitor {
    captured: Vec<(&'static str, String)>,
}

impl LabelVisitor {
    fn is_empty(&self) -> bool {
        self.captured.is_empty()
    }

    fn drain(&mut self) -> impl Iterator<Item = (&'static str, String)> + '_ {
        self.captured.drain(..)
    }

    fn maybe_record(&mut self, field: &Field, value: String) {
        let name = field.name();
        for &(alias, canonical) in LABEL_ALIASES {
            if alias == name {
                self.captured.push((canonical, value));
                return;
            }
        }
    }
}

impl Visit for LabelVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.maybe_record(field, value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.maybe_record(field, format!("{:?}", value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.maybe_record(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.maybe_record(field, value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.maybe_record(field, value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.maybe_record(field, value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::metrics::MeterProvider;
    use opentelemetry_sdk::metrics::data::{Histogram as HistogramData, ResourceMetrics};
    use opentelemetry_sdk::metrics::reader::MetricReader;
    use opentelemetry_sdk::metrics::{ManualReader, SdkMeterProvider};
    use opentelemetry_sdk::Resource;
    use std::sync::Arc;
    use tracing::subscriber::with_default;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    /// Build a `SdkMeterProvider` paired with a `ManualReader` so the test
    /// can synchronously snapshot what the histograms recorded. The provider
    /// holds one `Arc<ManualReader>` (wrapped in a delegating `MetricReader`)
    /// and the caller holds a sibling `Arc` for `.collect()`.
    fn meter_with_reader() -> (SdkMeterProvider, Arc<ManualReader>) {
        let reader = Arc::new(ManualReader::builder().build());
        let provider = SdkMeterProvider::builder()
            .with_reader(SharedManualReader(reader.clone()))
            .build();
        (provider, reader)
    }

    /// `MetricReader` implementation that delegates to a shared
    /// `Arc<ManualReader>`. Lets the test keep a reader handle for
    /// `.collect()` while the provider also owns one.
    struct SharedManualReader(Arc<ManualReader>);

    impl std::fmt::Debug for SharedManualReader {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SharedManualReader").finish()
        }
    }

    impl opentelemetry_sdk::metrics::reader::MetricReader for SharedManualReader {
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

    fn empty_resource_metrics() -> ResourceMetrics {
        ResourceMetrics {
            resource: Resource::empty(),
            scope_metrics: Vec::new(),
        }
    }

    fn collect_histogram(rm: &ResourceMetrics, name: &str) -> Option<HistogramData<f64>> {
        for scope in &rm.scope_metrics {
            for metric in &scope.metrics {
                if metric.name == name {
                    if let Some(hist) = metric.data.as_any().downcast_ref::<HistogramData<f64>>() {
                        // Clone manually because `HistogramData` is not Clone.
                        return Some(HistogramData {
                            data_points: hist.data_points.clone(),
                            temporality: hist.temporality,
                        });
                    }
                }
            }
        }
        None
    }

    #[test]
    fn build_span_metrics_layer_registers_all_seven_spans() {
        let (provider, _reader) = meter_with_reader();
        let meter = provider.meter("test");
        let layer = build_span_metrics_layer(&meter);

        let mut expected: Vec<&'static str> = PRE_REGISTERED_SPANS.to_vec();
        expected.sort();
        assert_eq!(
            layer.registered_names(),
            expected,
            "all seven pre-registered span histograms must be created up front"
        );
        assert_eq!(layer.registered_names().len(), 7);
    }

    #[test]
    fn layer_composes_with_registry_with_no_attached_readers() {
        // `SdkMeterProvider::default()` has no readers wired; histograms
        // discard everything they record. The layer must still build and
        // compose into a `Registry` without panicking.
        let provider = SdkMeterProvider::default();
        let meter = provider.meter("noop");
        let layer = build_span_metrics_layer(&meter);

        let subscriber = Registry::default().with(layer);
        with_default(subscriber, || {
            let span = tracing::info_span!("http.server.request");
            let _g = span.enter();
        });
    }

    #[test]
    fn http_server_request_span_records_one_observation_with_expected_duration() {
        let (provider, reader) = meter_with_reader();
        let meter = provider.meter("test");
        let layer = build_span_metrics_layer(&meter);

        let subscriber = Registry::default().with(layer);
        with_default(subscriber, || {
            let span = tracing::info_span!(
                "http.server.request",
                http_method = "GET",
                http_route = "/api/logs",
                service_name = "fold_db_node",
            );
            let _enter = span.enter();
            std::thread::sleep(std::time::Duration::from_millis(5));
            drop(_enter);
            drop(span);
        });

        let mut rm = empty_resource_metrics();
        reader
            .collect(&mut rm)
            .expect("manual reader should collect");

        let hist = collect_histogram(&rm, "http.server.request")
            .expect("http.server.request histogram must be present");
        assert_eq!(
            hist.data_points.len(),
            1,
            "exactly one histogram time-series for the single observation"
        );
        let dp = &hist.data_points[0];
        assert_eq!(dp.count, 1, "exactly one recorded measurement");
        assert!(
            dp.sum >= 5.0 && dp.sum < 5_000.0,
            "duration ~5ms, got {}ms (sum should be at least 5ms but under 5s)",
            dp.sum
        );
    }

    #[test]
    fn cardinality_allowlist_drops_high_cardinality_labels() {
        let (provider, reader) = meter_with_reader();
        let meter = provider.meter("test");
        let layer = build_span_metrics_layer(&meter);

        let subscriber = Registry::default().with(layer);
        with_default(subscriber, || {
            // Mix one allowed label with several high-cardinality fields
            // (uuid, user hash, schema name). Only `http_method` should
            // survive onto the histogram observation.
            let user_uuid = uuid::Uuid::new_v4().to_string();
            let span = tracing::info_span!(
                "db.sled.put",
                http_method = "PUT",
                user_hash = user_uuid.as_str(),
                schema_name = "Notes",
                fold_node_id = "node-abc-123",
            );
            let _enter = span.enter();
        });

        let mut rm = empty_resource_metrics();
        reader.collect(&mut rm).expect("collect");

        let hist =
            collect_histogram(&rm, "db.sled.put").expect("db.sled.put histogram must be present");
        assert_eq!(hist.data_points.len(), 1);
        let dp = &hist.data_points[0];
        let keys: Vec<&str> = dp.attributes.iter().map(|kv| kv.key.as_str()).collect();
        assert_eq!(
            keys,
            vec!["http.method"],
            "only allowlisted labels may flow through; got {:?}",
            keys
        );
        // Belt-and-suspenders: explicitly assert each forbidden key is absent.
        for forbidden in ["user.hash", "user_hash", "schema.name", "fold.node_id"] {
            assert!(
                !dp.attributes.iter().any(|kv| kv.key.as_str() == forbidden),
                "high-cardinality label {forbidden} must not propagate"
            );
        }
    }

    #[test]
    fn span_name_not_in_allowlist_is_ignored() {
        let (provider, reader) = meter_with_reader();
        let meter = provider.meter("test");
        let layer = build_span_metrics_layer(&meter);

        let subscriber = Registry::default().with(layer);
        with_default(subscriber, || {
            let span = tracing::info_span!("some.unregistered.span", http_method = "GET");
            let _enter = span.enter();
        });

        let mut rm = empty_resource_metrics();
        reader.collect(&mut rm).expect("collect");
        // Unregistered span name must produce no metric series at all — there
        // should be no histogram named after it.
        assert!(
            collect_histogram(&rm, "some.unregistered.span").is_none(),
            "unregistered span names must not create metrics"
        );
    }

    #[test]
    fn label_alias_canonicalizes_to_dot_form() {
        let (provider, reader) = meter_with_reader();
        let meter = provider.meter("test");
        let layer = build_span_metrics_layer(&meter);

        let subscriber = Registry::default().with(layer);
        with_default(subscriber, || {
            // Use the underscore form (the only form tracing macros accept
            // without raw-string trickery).
            let span = tracing::info_span!(
                "db.sled.get",
                http_method = "GET",
                http_route = "/api/v1/foo",
                service_name = "fold_db_node",
            );
            let _e = span.enter();
        });

        let mut rm = empty_resource_metrics();
        reader.collect(&mut rm).expect("collect");
        let hist = collect_histogram(&rm, "db.sled.get").expect("histogram present");
        let mut keys: Vec<String> = hist.data_points[0]
            .attributes
            .iter()
            .map(|kv| kv.key.to_string())
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "http.method".to_string(),
                "http.route".to_string(),
                "service.name".to_string(),
            ],
            "underscore field names must be normalized to OTel dot-form"
        );
    }
}
