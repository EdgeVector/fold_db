//! WEB layer — fans out each event as a JSON `LogEntry` over a
//! [`tokio::sync::broadcast`] channel for the dashboard's
//! `/api/logs/stream` SSE consumer.
//!
//! Phase 3 / T4. The pre-existing `WebOutput` in `fold_db::logging::outputs`
//! already serializes a [`LogEntry`] to JSON and broadcasts it on a
//! `broadcast::Sender<String>`. This layer is the tracing-native replacement
//! that Phase 3 / T5 will subscribe `/api/logs/stream` to. Until that
//! rewiring lands the layer is registered nowhere.
//!
//! ## Wire shape
//!
//! Identical to [`crate::layers::ring::LogEntry`] (which itself mirrors
//! `fold_db::logging::core::LogEntry`) so the dashboard parser does not
//! change between RING/poll and WEB/stream:
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
//! `trace_id` and `span_id` are added to `metadata` when a parent span
//! carries an `OtelData` extension — purely additive, the dashboard's
//! existing parser ignores unknown metadata keys.
//!
//! ## Backpressure
//!
//! [`broadcast::Sender::send`] is non-blocking and returns `Err` only when
//! there are no live receivers — it does not block the tracing pipeline
//! when subscribers fall behind. Slow consumers see `RecvError::Lagged`
//! on their next `recv()` and skip ahead; we accept that trade-off for
//! the SSE endpoint where dropping is preferable to back-pressuring the
//! whole process.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;
use tracing::{Event, Subscriber};
use tracing_opentelemetry::OtelData;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

use crate::layers::ring::{FieldVisitor, LogEntry, LogLevel};

/// Default capacity for the WEB broadcast channel when `init_*` does not
/// specify one. Sized to absorb a brief burst without forcing slow SSE
/// consumers into `Lagged` immediately.
pub const OBS_WEB_CAPACITY: usize = 1024;

/// Cheap-to-clone handle that hands out broadcast receivers for the WEB
/// layer. Phase 3 / T5 will hold one of these inside the HTTP server state
/// so each `/api/logs/stream` connection can call [`WebHandle::subscribe`].
#[derive(Clone)]
pub struct WebHandle {
    sender: Arc<broadcast::Sender<String>>,
}

impl WebHandle {
    /// New SSE-style receiver of serialized [`LogEntry`] JSON strings.
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }

    /// Current number of live subscribers — primarily for tests.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// Subscriber layer that serializes each event as a [`LogEntry`] JSON
/// string and fans it out over the broadcast channel held by
/// [`WebHandle`].
pub struct WebLayer {
    sender: Arc<broadcast::Sender<String>>,
}

/// Build a WEB layer + the handle that lets HTTP handlers subscribe to it.
/// Capacity is clamped to a minimum of 1 — `broadcast::channel(0)` panics.
pub fn build_web_layer(capacity: usize) -> (WebLayer, WebHandle) {
    let cap = capacity.max(1);
    let (sender, _) = broadcast::channel(cap);
    let sender = Arc::new(sender);
    (
        WebLayer {
            sender: sender.clone(),
        },
        WebHandle { sender },
    )
}

impl<S> Layer<S> for WebLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let event_meta = event.metadata();
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let mut metadata_map = visitor.fields;

        // See the long-form rationale on `RingLayer::on_event` — same
        // dance: lift trace_id / span_id from the parent span's
        // `OtelData` extension when an OpenTelemetry layer is also
        // installed. Absent that layer the keys simply don't appear,
        // which the dashboard parser tolerates.
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

        // Serialize once, fan out to every subscriber. `send` is
        // non-blocking and returns Err only when no receivers exist —
        // that's expected before the dashboard connects, so we
        // deliberately swallow the error rather than dropping
        // observability noise into stderr on every event.
        if let Ok(json) = serde_json::to_string(&entry) {
            let _ = self.sender.send(json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::trace::{TraceContextExt, TracerProvider};
    use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
    use serde_json::Value;
    use tracing::subscriber::with_default;
    use tracing_opentelemetry::OpenTelemetrySpanExt;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    /// Snapshot test: drive the WEB layer with a single event under an
    /// OTel-instrumented span, recv the JSON off the broadcast channel,
    /// and assert the parsed value matches the dashboard's expected
    /// `LogEntry` shape exactly — every key present, every type correct,
    /// `trace_id` / `span_id` propagated additively into `metadata`.
    #[test]
    fn snapshot_matches_logentry_shape_with_trace_ids() {
        let provider = SdkTracerProvider::builder().build();
        let tracer = provider.tracer("web-test");
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        let (web_layer, handle) = build_web_layer(16);
        // Subscribe BEFORE emitting — `broadcast::Sender::send` returns
        // Err with no receivers and the event would be silently dropped.
        let mut rx = handle.subscribe();
        assert_eq!(handle.receiver_count(), 1);

        let subscriber = Registry::default().with(otel_layer).with(web_layer);

        let expected_trace_id = with_default(subscriber, || {
            let span = tracing::info_span!("unit");
            let _enter = span.enter();
            tracing::info!(target: "web_test", user_hash = "abcd", "hello world");
            let span_ctx = span.context().span().span_context().clone();
            assert!(
                span_ctx.is_valid(),
                "OtelLayer must seed a valid span context"
            );
            format!("{:032x}", span_ctx.trace_id())
        });

        let json_str = rx.try_recv().expect("event must be on the channel");
        let value: Value = serde_json::from_str(&json_str).expect("must be valid JSON");

        // Field-by-field shape assertions — this is the contract the
        // dashboard SSE parser depends on. Adding fields is fine,
        // changing types or removing fields breaks the wire.
        assert!(value["id"].is_string(), "id must be a string");
        assert_eq!(
            value["id"].as_str().unwrap().len(),
            36,
            "id must be a uuid-v4 hyphenated string"
        );
        assert!(value["timestamp"].is_i64(), "timestamp must be i64 millis");
        assert!(value["timestamp"].as_i64().unwrap() > 0);
        assert_eq!(value["level"], Value::String("INFO".to_string()));
        assert_eq!(value["event_type"], Value::String("web_test".to_string()));
        assert_eq!(value["message"], Value::String("hello world".to_string()));
        assert!(
            value.get("user_id").is_some(),
            "user_id key must be present"
        );
        assert!(value["user_id"].is_null(), "user_id must serialize as null");

        let meta = value["metadata"]
            .as_object()
            .expect("metadata must be an object");
        assert_eq!(
            meta.get("user_hash").and_then(Value::as_str),
            Some("abcd"),
            "custom event field must land in metadata"
        );
        assert_eq!(
            meta.get("trace_id").and_then(Value::as_str),
            Some(expected_trace_id.as_str()),
            "trace_id must be the parent span's W3C trace id (32 hex chars)"
        );
        let span_id = meta
            .get("span_id")
            .and_then(Value::as_str)
            .expect("span_id must be present under an OTel-instrumented span");
        assert_eq!(span_id.len(), 16, "span_id must be 16-char hex");
        assert!(
            span_id.chars().all(|c| c.is_ascii_hexdigit()),
            "span_id must be hex"
        );

        // The wire shape must contain *exactly* these top-level keys —
        // catches accidental new fields landing in front of the
        // dashboard parser.
        let mut keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "event_type",
                "id",
                "level",
                "message",
                "metadata",
                "timestamp",
                "user_id",
            ]
        );
    }

    #[test]
    fn send_with_no_subscribers_does_not_panic() {
        // Pre-Phase-3-T5 the dashboard hasn't connected yet, so the
        // sender's receiver_count is zero. The layer must swallow the
        // resulting `SendError` instead of poisoning the tracing path.
        let (web_layer, handle) = build_web_layer(4);
        assert_eq!(handle.receiver_count(), 0);

        let subscriber = Registry::default().with(web_layer);
        with_default(subscriber, || {
            tracing::info!("nobody is listening");
        });
    }

    #[test]
    fn capacity_zero_is_clamped_to_one() {
        // `broadcast::channel(0)` panics — `build_web_layer(0)` must not.
        let (_layer, handle) = build_web_layer(0);
        let _rx = handle.subscribe();
    }
}
