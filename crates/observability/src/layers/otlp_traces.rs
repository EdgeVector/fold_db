//! OTLP TRACES layer — exports `tracing` spans to an OTel collector over
//! HTTP/protobuf.
//!
//! Phase 4 / T1. The layer is a no-op when the `OBS_OTLP_ENDPOINT` env var is
//! unset, so binaries can compose it unconditionally and get OTLP only when
//! the operator opts in.
//!
//! ## Non-blocking contract
//!
//! Span emission is on the hot path of every instrumented call. Saturating
//! the exporter (slow collector, high traffic burst) MUST NOT block the
//! caller. We achieve that with a bounded `tokio::sync::mpsc` channel sized
//! `MAX_QUEUE_SIZE = 2048`; `on_end` uses [`try_send`] and on full simply
//! drops the span and increments the [`obs.spans.dropped`](OtlpGuard::dropped)
//! counter. No retry, no panic, no spin.
//!
//! ## Worker thread
//!
//! A dedicated `obs-otlp-traces` OS thread runs a single-thread Tokio
//! runtime. The worker batches up to `MAX_EXPORT_BATCH_SIZE = 512` spans per
//! flush and either fills the batch or waits up to `SCHEDULED_DELAY = 5s`
//! before exporting. The wall-clock for a slow OTLP request is absorbed by
//! the worker, never by the application thread.
//!
//! Running the runtime on its own thread (rather than spawning into an
//! ambient one) means:
//! - the layer composes the same in CLI, Tauri, and Lambda binaries that may
//!   not have a Tokio runtime running at subscriber-install time;
//! - shutting down the FoldNode runtime never starves the OTLP flush.
//!
//! ## Drop counter
//!
//! [`OtlpGuard::dropped`] returns a snapshot of the lifetime drop count.
//! Phase 4 / T3 (OTLP METRICS) will publish this as an OTel counter named
//! `obs.spans.dropped`; for now it is queryable in-process and asserted
//! against by the integration test.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opentelemetry::trace::{TraceResult, TracerProvider as _};
use opentelemetry::{Context, KeyValue};
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::export::trace::{SpanData, SpanExporter as SpanExporterTrait};
use opentelemetry_sdk::trace::{Span as SdkSpan, SpanProcessor, Tracer, TracerProvider};
use opentelemetry_sdk::Resource;
use tracing::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::registry::LookupSpan;

/// Env var that gates the OTLP exporter. When unset (or empty) the layer is
/// a no-op and `build_otlp_traces_layer` returns `None`.
pub const OBS_OTLP_ENDPOINT_ENV: &str = "OBS_OTLP_ENDPOINT";

/// Hard upper bound on spans buffered between the application threads and
/// the OTLP exporter worker. Once full, new spans are dropped.
pub const MAX_QUEUE_SIZE: usize = 2048;

/// Largest batch the worker hands to a single `SpanExporter::export` call.
pub const MAX_EXPORT_BATCH_SIZE: usize = 512;

/// Maximum delay between scheduled flushes when the batch is not yet full.
pub const SCHEDULED_DELAY: Duration = Duration::from_secs(5);

/// RAII handle returned alongside the OTLP layer. Holds a clone of the
/// dropped-span counter and the [`TracerProvider`] so its `Drop` can issue an
/// orderly shutdown that drains any in-flight spans.
#[must_use = "OtlpGuard must be held for the lifetime of the binary or trailing spans are lost"]
pub struct OtlpGuard {
    dropped: Arc<AtomicU64>,
    provider: Option<TracerProvider>,
}

impl OtlpGuard {
    /// Lifetime count of spans dropped because the worker channel was full.
    /// Surfaced as the `obs.spans.dropped` self-monitoring metric.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

impl std::fmt::Debug for OtlpGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OtlpGuard")
            .field("dropped", &self.dropped())
            .field("provider", &self.provider.is_some())
            .finish()
    }
}

impl Drop for OtlpGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.provider.take() {
            // `shutdown` on a TracerProvider fans out to every registered
            // SpanProcessor, which drains the worker channel and waits for
            // the runtime thread to join. Errors here are non-actionable —
            // the binary is exiting anyway.
            let _ = provider.shutdown();
        }
    }
}

/// Build the OTLP traces layer + its [`OtlpGuard`].
///
/// Returns `None` when `OBS_OTLP_ENDPOINT` is unset or empty — that's the
/// "OTLP off" state, not a failure.
///
/// Returns `None` on exporter construction error too: at startup we'd rather
/// run without remote tracing than crash the binary because a collector URL
/// was malformed. The error is logged via `tracing::error!` so the operator
/// has a thread to pull on.
pub fn build_otlp_traces_layer<S>(
    service_name: &str,
) -> Option<(OpenTelemetryLayer<S, Tracer>, OtlpGuard)>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let endpoint = std::env::var(OBS_OTLP_ENDPOINT_ENV).ok()?;
    if endpoint.trim().is_empty() {
        return None;
    }

    let exporter = match SpanExporter::builder()
        .with_http()
        .with_endpoint(&endpoint)
        .with_protocol(Protocol::HttpBinary)
        .build()
    {
        Ok(e) => e,
        Err(err) => {
            tracing::error!(
                target: "observability::otlp_traces",
                error = %err,
                endpoint = %endpoint,
                "failed to construct OTLP span exporter; OTLP traces disabled",
            );
            return None;
        }
    };

    let dropped = Arc::new(AtomicU64::new(0));
    let processor = BoundedDropProcessor::spawn(exporter, dropped.clone());

    let provider = TracerProvider::builder()
        .with_span_processor(processor)
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            service_name.to_string(),
        )]))
        .build();

    let tracer = provider.tracer(service_name.to_string());
    let layer = tracing_opentelemetry::layer().with_tracer(tracer);

    Some((
        layer,
        OtlpGuard {
            dropped,
            provider: Some(provider),
        },
    ))
}

// ---------------------------------------------------------------------------
// BoundedDropProcessor — bounded, non-blocking SpanProcessor that drops on
// saturation and counts the drops. Forwards surviving spans to the supplied
// `SpanExporter` in batches from a dedicated worker thread.
//
// Span traffic and control signals (flush / shutdown) ride on **separate**
// channels: shutdown must work even when the span queue is fully saturated,
// or a wedged collector would deadlock the binary on exit.
// ---------------------------------------------------------------------------

struct BoundedDropProcessor {
    span_tx: tokio::sync::mpsc::Sender<SpanData>,
    ctrl_tx: tokio::sync::mpsc::Sender<CtrlMsg>,
    dropped: Arc<AtomicU64>,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}

enum CtrlMsg {
    Flush(tokio::sync::oneshot::Sender<()>),
    Shutdown(tokio::sync::oneshot::Sender<()>),
}

/// Upper bound on a single OTLP export call. A wedged collector must not
/// keep a shutdown waiting longer than this — span drops are preferable to a
/// hung process exit.
const EXPORT_TIMEOUT: Duration = Duration::from_secs(2);

/// Hard cap on shutdown wall-clock. If the worker can't drain + ack within
/// this budget we abandon it and let the OS reap the thread on exit.
const SHUTDOWN_BUDGET: Duration = Duration::from_secs(3);

impl std::fmt::Debug for BoundedDropProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundedDropProcessor")
            .field("dropped", &self.dropped.load(Ordering::Relaxed))
            .finish()
    }
}

impl BoundedDropProcessor {
    fn spawn<E>(exporter: E, dropped: Arc<AtomicU64>) -> Self
    where
        E: SpanExporterTrait + 'static,
    {
        let (span_tx, span_rx) = tokio::sync::mpsc::channel::<SpanData>(MAX_QUEUE_SIZE);
        // Control channel sized for one in-flight flush + one shutdown — far
        // more than callers ever queue concurrently.
        let (ctrl_tx, ctrl_rx) = tokio::sync::mpsc::channel::<CtrlMsg>(4);

        let worker = std::thread::Builder::new()
            .name("obs-otlp-traces".to_string())
            .spawn(move || run_worker(exporter, span_rx, ctrl_rx))
            .expect("spawn obs-otlp-traces worker thread");

        Self {
            span_tx,
            ctrl_tx,
            dropped,
            worker: Mutex::new(Some(worker)),
        }
    }
}

impl SpanProcessor for BoundedDropProcessor {
    fn on_start(&self, _span: &mut SdkSpan, _cx: &Context) {}

    fn on_end(&self, span: SpanData) {
        // try_send is the bedrock of the non-blocking contract: it returns
        // immediately whether the channel had room or not. Both Full and
        // Closed map to "drop and count" — Closed only happens after
        // shutdown, where dropping is the only sane behavior anyway.
        if self.span_tx.try_send(span).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn force_flush(&self) -> TraceResult<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if self.ctrl_tx.try_send(CtrlMsg::Flush(tx)).is_err() {
            return Ok(());
        }
        // force_flush callers (TracerProvider shutdown, manual operator
        // flush) are explicitly happy to wait — the on_end hot path is the
        // one that must stay non-blocking.
        let _ = rx.blocking_recv();
        Ok(())
    }

    fn shutdown(&self) -> TraceResult<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        // Control channel has its own capacity, so even a fully-saturated
        // span queue cannot starve shutdown signalling.
        if self.ctrl_tx.try_send(CtrlMsg::Shutdown(tx)).is_ok() {
            // Wait up to SHUTDOWN_BUDGET for a clean drain. The worker may
            // be stuck in a hanging export on a wedged collector — in that
            // case we'd rather lose a tail of in-flight spans than hang the
            // binary exit indefinitely.
            let deadline = std::time::Instant::now() + SHUTDOWN_BUDGET;
            let mut rx = rx;
            loop {
                match rx.try_recv() {
                    Ok(()) => break,
                    Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                        if std::time::Instant::now() >= deadline {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Err(_) => break,
                }
            }
        }
        if let Some(handle) = self.worker.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = handle.join();
        }
        Ok(())
    }
}

fn run_worker<E>(
    mut exporter: E,
    mut span_rx: tokio::sync::mpsc::Receiver<SpanData>,
    mut ctrl_rx: tokio::sync::mpsc::Receiver<CtrlMsg>,
) where
    E: SpanExporterTrait + 'static,
{
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            tracing::error!(
                target: "observability::otlp_traces",
                error = %err,
                "failed to build worker runtime; spans will be dropped",
            );
            return;
        }
    };

    runtime.block_on(async move {
        let mut buf: Vec<SpanData> = Vec::with_capacity(MAX_EXPORT_BATCH_SIZE);
        let mut shutdown_ack: Option<tokio::sync::oneshot::Sender<()>> = None;

        'main: loop {
            let sleep = tokio::time::sleep(SCHEDULED_DELAY);
            tokio::pin!(sleep);

            tokio::select! {
                biased;
                ctrl = ctrl_rx.recv() => {
                    match ctrl {
                        Some(CtrlMsg::Flush(ack)) => {
                            // Drain any pending spans before flushing.
                            while buf.len() < MAX_EXPORT_BATCH_SIZE {
                                match span_rx.try_recv() {
                                    Ok(s) => buf.push(s),
                                    Err(_) => break,
                                }
                            }
                            if !buf.is_empty() {
                                flush_batch(&mut exporter, &mut buf).await;
                            }
                            let _ = ack.send(());
                        }
                        Some(CtrlMsg::Shutdown(ack)) => {
                            shutdown_ack = Some(ack);
                            break 'main;
                        }
                        None => break 'main,
                    }
                }
                _ = &mut sleep => {
                    if !buf.is_empty() {
                        flush_batch(&mut exporter, &mut buf).await;
                    }
                }
                maybe = span_rx.recv() => {
                    match maybe {
                        Some(s) => {
                            buf.push(s);
                            while buf.len() < MAX_EXPORT_BATCH_SIZE {
                                match span_rx.try_recv() {
                                    Ok(more) => buf.push(more),
                                    Err(_) => break,
                                }
                            }
                            if buf.len() >= MAX_EXPORT_BATCH_SIZE {
                                flush_batch(&mut exporter, &mut buf).await;
                            }
                        }
                        None => break 'main,
                    }
                }
            }
        }

        // Drain anything the channels still hold so we do not silently lose
        // spans the application already handed over.
        while let Ok(s) = span_rx.try_recv() {
            buf.push(s);
        }
        if !buf.is_empty() {
            flush_batch(&mut exporter, &mut buf).await;
        }
        exporter.shutdown();
        if let Some(ack) = shutdown_ack {
            let _ = ack.send(());
        }
    });
}

async fn flush_batch<E: SpanExporterTrait>(exporter: &mut E, buf: &mut Vec<SpanData>) {
    if buf.is_empty() {
        return;
    }
    let drained = std::mem::replace(buf, Vec::with_capacity(MAX_EXPORT_BATCH_SIZE));
    // Bound each export so a wedged collector never holds the worker for
    // longer than EXPORT_TIMEOUT. Without this, force_flush + shutdown
    // can be held hostage by the network.
    match tokio::time::timeout(EXPORT_TIMEOUT, exporter.export(drained)).await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            tracing::warn!(
                target: "observability::otlp_traces",
                error = %err,
                "OTLP span export failed",
            );
        }
        Err(_) => {
            tracing::warn!(
                target: "observability::otlp_traces",
                "OTLP span export timed out (collector unresponsive)",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use tracing::subscriber::with_default;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    /// Cargo runs unit tests in parallel within a single process, so any test
    /// that mutates `OBS_OTLP_ENDPOINT` races with siblings. Acquire this
    /// lock for the duration of the env-var setup and the
    /// `build_otlp_traces_layer` call, then release after the assertion.
    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    /// The env-var guard ensures tests that mutate `OBS_OTLP_ENDPOINT` do not
    /// leak that state to siblings running in the same process.
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
    fn returns_none_when_endpoint_unset() {
        let _serial = env_lock();
        let _guard = EnvGuard::unset(OBS_OTLP_ENDPOINT_ENV);
        let layer = build_otlp_traces_layer::<Registry>("svc");
        assert!(layer.is_none(), "must be a no-op when env var is missing");
    }

    #[test]
    fn returns_none_when_endpoint_is_empty() {
        let _serial = env_lock();
        let _guard = EnvGuard::set(OBS_OTLP_ENDPOINT_ENV, "   ");
        let layer = build_otlp_traces_layer::<Registry>("svc");
        assert!(layer.is_none(), "whitespace-only endpoint must be no-op");
    }

    #[test]
    fn returns_some_and_composes_into_registry() {
        // Point at an unused TCP port. The exporter constructs lazily; a
        // failed connect later is the worker thread's problem, not the
        // build path's.
        let _serial = env_lock();
        let _guard = EnvGuard::set(OBS_OTLP_ENDPOINT_ENV, "http://127.0.0.1:1");

        let (layer, otlp_guard) =
            build_otlp_traces_layer::<Registry>("svc").expect("layer must build");

        let subscriber = Registry::default().with(layer);
        with_default(subscriber, || {
            let _span = tracing::info_span!("compose_test").entered();
            tracing::info!("payload");
        });

        // Drop count is best-effort: we don't expect drops on a single span
        // emission. The point of the assert is that calling `.dropped()`
        // works through the public surface.
        let _ = otlp_guard.dropped();
    }
}
