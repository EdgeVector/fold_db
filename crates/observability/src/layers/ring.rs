//! RING layer — bounded in-memory `VecDeque<LogEntry>` queryable in-process.
//!
//! Phase 1 / T5. The RING layer captures every event the registry emits into
//! a fixed-capacity ring buffer that another part of the process can drain on
//! demand. In Phase 3 it replaces `LoggingSystem::query_logs` and powers the
//! `/api/logs` HTTP endpoint that the dashboard polls.
//!
//! ## LogEntry shape
//!
//! The on-the-wire JSON shape is identical to the existing
//! `fold_db::logging::core::LogEntry` so the dashboard parser does not have
//! to change when Phase 3 rewires the endpoint:
//!
//! ```json
//! {
//!   "id": "<uuid v4>",
//!   "timestamp": 1714060800123,
//!   "level": "INFO",
//!   "event_type": "module::path",
//!   "message": "the formatted event message",
//!   "user_id": null,
//!   "metadata": { "trace_id": "...", "span_id": "...", "field.name": "..." }
//! }
//! ```
//!
//! `user_id` is left as `None` here on purpose: task-local user context lives
//! in `fold_db_core` and the observability crate has zero deps on it. Phase 3
//! will bridge that when it consolidates logging — until then RING is the
//! plumbing, not the policy.
//!
//! ## Trace correlation
//!
//! When a `tracing-opentelemetry` layer is also installed in the registry,
//! the current span carries a real W3C span context. The RING layer reads
//! that context and writes `trace_id` (32 hex chars) and `span_id` (16 hex
//! chars) into `LogEntry.metadata` so individual log lines can be joined
//! against distributed traces at query time.
//!
//! ## Concurrency
//!
//! `on_event` is synchronous and called on the event-emitting thread. The
//! buffer is guarded by `std::sync::RwLock`, which is cheap on the hot write
//! path because writes hold the lock for the duration of one `push_back` (+
//! one `pop_front` at capacity). Queries take a read lock and clone out the
//! slice they need.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_opentelemetry::OtelData;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

/// Default capacity for the RING buffer when `init_*` does not specify one.
pub const OBS_RING_CAPACITY: usize = 5000;

/// In-memory log entry. Wire-compatible with `fold_db::logging::core::LogEntry`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: i64,
    pub level: LogLevel,
    pub event_type: String,
    pub message: String,
    pub user_id: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Log level. Wire-compatible with `fold_db::logging::core::LogLevel` —
/// serializes to UPPERCASE strings (`"TRACE"`, `"DEBUG"`, ...).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub(super) fn from_tracing(level: &tracing::Level) -> Self {
        match *level {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        }
    }
}

/// Handle to a RING buffer that lets other parts of the process query the
/// recently-emitted entries. Cheap to clone — internally an `Arc`.
#[derive(Clone)]
pub struct RingHandle {
    buffer: Arc<RwLock<VecDeque<LogEntry>>>,
    capacity: usize,
}

impl RingHandle {
    /// Return up to `limit` most-recent entries, optionally filtered to those
    /// with `timestamp >= from_timestamp`. Results are ordered oldest → newest
    /// to match the existing `WebOutput::query` contract that the dashboard
    /// already consumes.
    pub fn query(&self, limit: Option<usize>, from_timestamp: Option<i64>) -> Vec<LogEntry> {
        let buf = self.buffer.read().unwrap_or_else(|p| p.into_inner());
        let from_ts = from_timestamp.unwrap_or(i64::MIN);

        // Walk newest → oldest so an explicit `limit` keeps the *most recent*
        // N entries; reverse at the end to restore chronological order.
        let mut picked: Vec<LogEntry> = buf
            .iter()
            .rev()
            .filter(|e| e.timestamp >= from_ts)
            .take(limit.unwrap_or(usize::MAX))
            .cloned()
            .collect();
        picked.reverse();
        picked
    }

    /// Buffer capacity (the bound enforced on each push).
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Current entry count. Primarily for tests; not part of the stable API
    /// that `/api/logs` will rely on.
    pub fn len(&self) -> usize {
        self.buffer.read().unwrap_or_else(|p| p.into_inner()).len()
    }

    /// `true` when no entries have been recorded yet.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Subscriber layer that records each event into a bounded ring buffer.
pub struct RingLayer {
    handle: RingHandle,
}

/// Build a RING layer + the handle that lets the rest of the process query
/// it. Capacity is clamped to a minimum of 1 — a zero-capacity ring is a
/// foot-gun that silently drops every event.
pub fn build_ring_layer(capacity: usize) -> (RingLayer, RingHandle) {
    let cap = capacity.max(1);
    let handle = RingHandle {
        buffer: Arc::new(RwLock::new(VecDeque::with_capacity(cap))),
        capacity: cap,
    };
    (
        RingLayer {
            handle: handle.clone(),
        },
        handle,
    )
}

impl<S> Layer<S> for RingLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let event_meta = event.metadata();
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let mut metadata_map = visitor.fields;

        // Lift trace_id / span_id directly off the parent span's `OtelData`
        // extension. `OtelData` is attached by `tracing-opentelemetry`'s
        // layer during `on_new_span`. Reading it through `ctx.event_span`
        // is the canonical layer-to-layer interop pattern; calling
        // `Span::current().context()` from inside `on_event` is unreliable
        // because the dispatcher's notion of "current span" depends on
        // entry/exit ordering relative to the event hook. When no OTel
        // layer is installed (or the event has no parent span), the
        // extension is absent and we just skip these fields.
        if let Some(span_ref) = ctx.event_span(event) {
            let exts = span_ref.extensions();
            if let Some(otel_data) = exts.get::<OtelData>() {
                if let Some(trace_id) = otel_data.builder.trace_id {
                    metadata_map.insert("trace_id".to_string(), format!("{:032x}", trace_id));
                }
                if let Some(span_id) = otel_data.builder.span_id {
                    metadata_map.insert("span_id".to_string(), format!("{:016x}", span_id));
                }
            }
        }

        let entry = LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
            level: LogLevel::from_tracing(event_meta.level()),
            event_type: event_meta.target().to_string(),
            message: visitor.message,
            user_id: None,
            metadata: if metadata_map.is_empty() {
                None
            } else {
                Some(metadata_map)
            },
        };

        let mut buf = self
            .handle
            .buffer
            .write()
            .unwrap_or_else(|p| p.into_inner());
        if buf.len() == self.handle.capacity {
            buf.pop_front();
        }
        buf.push_back(entry);
    }
}

/// `tracing::field::Visit` impl that pulls the `message` field out separately
/// (it's how `tracing::info!("hello")` is recorded — as a debug field named
/// `message`) and stuffs everything else into a `String → String` map.
#[derive(Default)]
pub(super) struct FieldVisitor {
    pub(super) message: String,
    pub(super) fields: HashMap<String, String>,
}

impl FieldVisitor {
    fn store(&mut self, field: &Field, value: String) {
        if field.name() == "message" {
            self.message = value;
        } else {
            self.fields.insert(field.name().to_string(), value);
        }
    }
}

impl Visit for FieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.store(field, value.to_string());
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.store(field, format!("{:?}", value));
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.store(field, value.to_string());
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.store(field, value.to_string());
    }
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.store(field, value.to_string());
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.store(field, value.to_string());
    }
    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.store(field, value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::trace::{TraceContextExt, TracerProvider};
    use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
    use tracing::subscriber::with_default;
    use tracing_opentelemetry::OpenTelemetrySpanExt;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    /// Smallest registry that drives the RING layer for tests.
    fn registry_with(layer: RingLayer) -> impl tracing::Subscriber {
        Registry::default().with(layer)
    }

    #[test]
    fn captures_event_into_logentry_shape() {
        let (layer, handle) = build_ring_layer(16);
        let subscriber = registry_with(layer);

        with_default(subscriber, || {
            tracing::info!(target: "ring_test", user_hash = "abcd", "hello world");
        });

        let logs = handle.query(None, None);
        assert_eq!(logs.len(), 1);
        let entry = &logs[0];
        assert_eq!(entry.event_type, "ring_test");
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.message, "hello world");
        assert!(entry.user_id.is_none());

        // The custom field should have been folded into metadata.
        let meta = entry.metadata.as_ref().expect("metadata should be present");
        assert_eq!(meta.get("user_hash").map(String::as_str), Some("abcd"));

        // Wire shape: serializing must produce the dashboard's expected JSON keys
        // including `level: "INFO"` (UPPERCASE) and a present `user_id` key (null).
        let json = serde_json::to_value(entry).unwrap();
        assert_eq!(json["level"], serde_json::json!("INFO"));
        assert!(json.get("user_id").is_some(), "user_id key must be present");
        assert_eq!(json["user_id"], serde_json::Value::Null);
        for key in ["id", "timestamp", "event_type", "message", "metadata"] {
            assert!(json.get(key).is_some(), "missing key: {key}");
        }
    }

    #[test]
    fn ring_fills_to_capacity_and_evicts_oldest() {
        let (layer, handle) = build_ring_layer(3);
        let subscriber = registry_with(layer);

        with_default(subscriber, || {
            tracing::info!(seq = 1u64, "one");
            tracing::info!(seq = 2u64, "two");
            tracing::info!(seq = 3u64, "three");
            // At-capacity assertions.
            tracing::info!(seq = 4u64, "four");
            tracing::info!(seq = 5u64, "five");
        });

        let logs = handle.query(None, None);
        assert_eq!(logs.len(), 3, "ring must stay bounded by capacity");
        assert_eq!(handle.capacity(), 3);

        // Oldest two were evicted — we should see messages 3, 4, 5 in order.
        let messages: Vec<&str> = logs.iter().map(|e| e.message.as_str()).collect();
        assert_eq!(messages, vec!["three", "four", "five"]);
    }

    #[test]
    fn query_returns_most_recent_n_in_chronological_order() {
        let (layer, handle) = build_ring_layer(10);
        let subscriber = registry_with(layer);

        with_default(subscriber, || {
            for i in 0..5 {
                tracing::info!(i = i as u64, "msg {i}");
            }
        });

        // Limit = 3 → keep the latest 3, returned oldest → newest.
        let logs = handle.query(Some(3), None);
        let messages: Vec<String> = logs.iter().map(|e| e.message.clone()).collect();
        assert_eq!(messages, vec!["msg 2", "msg 3", "msg 4"]);
    }

    #[test]
    fn query_filters_by_from_timestamp() {
        let (layer, handle) = build_ring_layer(10);
        let subscriber = registry_with(layer);

        with_default(subscriber, || {
            tracing::info!("first");
            // Sleep enough for `SystemTime::now` to advance even on hosts
            // with coarse clocks (Windows ~16ms).
            std::thread::sleep(std::time::Duration::from_millis(20));
            let cutoff = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            std::thread::sleep(std::time::Duration::from_millis(20));
            tracing::info!("second");

            let after_cutoff = handle.query(None, Some(cutoff));
            let messages: Vec<String> = after_cutoff.iter().map(|e| e.message.clone()).collect();
            assert_eq!(messages, vec!["second"]);
        });
    }

    #[test]
    fn trace_id_and_span_id_propagate_into_metadata() {
        // Wire a tracing-opentelemetry layer alongside the RING layer so the
        // parent span carries a real W3C `OtelData` extension that the RING
        // layer can lift into `metadata`.
        let provider = SdkTracerProvider::builder().build();
        let tracer = provider.tracer("ring-test");
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        let (ring_layer, handle) = build_ring_layer(16);
        let subscriber = Registry::default().with(otel_layer).with(ring_layer);

        let trace_id_hex = with_default(subscriber, || {
            let span = tracing::info_span!("unit");
            let _enter = span.enter();
            tracing::info!("inside span");

            // Pull the trace id off the same span via OtelSpanExt so the test
            // asserts the *exact* id that should appear on the LogEntry.
            let span_ctx = span.context().span().span_context().clone();
            assert!(
                span_ctx.is_valid(),
                "OtelLayer must seed a valid span context"
            );
            format!("{:032x}", span_ctx.trace_id())
        });

        let logs = handle.query(None, None);
        assert_eq!(logs.len(), 1);
        let meta = logs[0]
            .metadata
            .as_ref()
            .expect("trace ids should populate metadata");
        assert_eq!(meta.get("trace_id"), Some(&trace_id_hex));
        assert!(
            meta.get("span_id").is_some_and(|s| s.len() == 16),
            "span_id must be 16-char hex, got {:?}",
            meta.get("span_id")
        );
    }

    #[test]
    fn no_otel_layer_means_no_trace_metadata() {
        // Without a tracing-opentelemetry layer the parent span carries no
        // OtelData; metadata should simply lack trace_id / span_id rather
        // than fail or panic.
        let (layer, handle) = build_ring_layer(4);
        let subscriber = registry_with(layer);

        with_default(subscriber, || {
            let span = tracing::info_span!("plain");
            let _enter = span.enter();
            tracing::info!("event without OTel");
        });

        let logs = handle.query(None, None);
        assert_eq!(logs.len(), 1);
        if let Some(meta) = &logs[0].metadata {
            assert!(!meta.contains_key("trace_id"));
            assert!(!meta.contains_key("span_id"));
        }
    }

    #[test]
    fn capacity_zero_is_clamped_to_one() {
        let (_layer, handle) = build_ring_layer(0);
        assert_eq!(
            handle.capacity(),
            1,
            "zero-capacity ring would silently drop everything"
        );
    }
}
