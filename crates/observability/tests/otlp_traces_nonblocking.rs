//! Integration test for the OTLP TRACES layer's non-blocking contract.
//!
//! Phase 4 / T1 acceptance criterion: emitting many spans rapidly to a slow
//! collector MUST NOT block the calling thread. We point the OTLP exporter
//! at a TCP listener that accepts connections and never replies, install a
//! `Registry` with the layer, and verify that 5000 span emissions complete
//! in well under one second of wall clock — the saturation must drop into
//! the per-span `obs.spans.dropped` counter, not into the application
//! thread.

use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use observability::layers::otlp_traces::{build_otlp_traces_layer, OBS_OTLP_ENDPOINT_ENV};
use tracing::subscriber::with_default;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

/// Bind to a free port, accept inbound connections, and *never* reply. This
/// simulates a wedged OTLP collector: the worker thread's reqwest call hangs
/// indefinitely on the response, the bounded mpsc fills, and span emissions
/// past `MAX_QUEUE_SIZE` start dropping.
fn spawn_blackhole(stop: Arc<AtomicBool>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind blackhole listener");
    listener
        .set_nonblocking(true)
        .expect("set listener non-blocking");
    let port = listener.local_addr().expect("local addr").port();

    std::thread::Builder::new()
        .name("otlp-blackhole".to_string())
        .spawn(move || {
            // Hold the accepted streams so the kernel doesn't tear them down
            // before the test finishes.
            let mut held = Vec::new();
            while !stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        held.push(stream);
                    }
                    Err(_) => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
            }
            drop(held);
        })
        .expect("spawn blackhole");

    port
}

#[test]
fn emit_5000_spans_does_not_block_caller() {
    let stop = Arc::new(AtomicBool::new(false));
    let port = spawn_blackhole(stop.clone());

    // Set the env var BEFORE building the layer. We never unset it from a
    // peer test in this binary — Cargo runs each `tests/<name>.rs` in its
    // own process.
    std::env::set_var(OBS_OTLP_ENDPOINT_ENV, format!("http://127.0.0.1:{port}"));

    let (layer, guard) =
        build_otlp_traces_layer::<Registry>("nonblocking-test").expect("layer must build");
    let subscriber = Registry::default().with(layer);

    let started = Instant::now();
    with_default(subscriber, || {
        for i in 0..5000 {
            let span = tracing::info_span!("hot.path", iteration = i);
            let _entered = span.entered();
            tracing::info!(iteration = i, "synthetic event");
        }
    });
    let elapsed = started.elapsed();

    // Generous bound: the BoundedDropProcessor's `try_send` is O(1) and the
    // work per span is dominated by `tracing` machinery, not the OTLP
    // pipeline. 5000 emissions should be sub-200ms even on a contended CI
    // runner; allowing 1s gives a healthy safety margin while still failing
    // loudly if a regression makes the path block on the collector.
    assert!(
        elapsed < Duration::from_secs(1),
        "caller blocked on OTLP exporter: {elapsed:?} for 5000 spans",
    );

    // Saturation should have produced drops because the blackhole holds the
    // worker indefinitely. The exact count depends on tokio scheduling — we
    // only assert "more than zero" so the test stays robust on fast hosts
    // where a few spans squeak through before the channel fills.
    let dropped = guard.dropped();
    assert!(
        dropped > 0,
        "expected obs.spans.dropped to fire under blackhole saturation, got {dropped}",
    );

    stop.store(true, Ordering::Relaxed);
    drop(guard);
    std::env::remove_var(OBS_OTLP_ENDPOINT_ENV);
}
