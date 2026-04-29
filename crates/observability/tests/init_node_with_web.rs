//! End-to-end integration test for [`observability::init_node_with_web`].
//!
//! Mirrors `tests/integration.rs` (which exercises [`init_node`]) but covers
//! the WEB-broadcast variant. Each `tests/<name>.rs` runs in its own binary,
//! so this file owns the `INIT_ONCE` slot and the global tracing subscriber
//! for the duration of its test process.
//!
//! Asserts:
//! 1. `init_node_with_web` returns a guard exposing `RingHandle`,
//!    `Arc<ReloadHandle>`, and `WebHandle` accessors that match the existing
//!    `NodeObsGuard` API in `fold_db_node`.
//! 2. Subscribing to the `WebHandle` BEFORE emitting an event delivers the
//!    serialized `LogEntry` JSON to the receiver.
//! 3. The captured event carries a real W3C `trace_id` in `metadata`,
//!    proving the OTel layer is composed and the subscriber is installed.
//! 4. A second `init_node_with_web` call surfaces as `AlreadyInitialized`.

use std::sync::OnceLock;
use std::time::Duration;

use observability::{init_node_with_web, ObsError};
use serde_json::Value;
use tempfile::TempDir;

/// Pin a `TempDir` for the lifetime of the test process so the FMT log path
/// the global subscriber holds open does not get unlinked while the worker
/// thread is still draining its queue.
fn shared_tempdir() -> &'static TempDir {
    static DIR: OnceLock<TempDir> = OnceLock::new();
    DIR.get_or_init(|| tempfile::tempdir().expect("create tempdir"))
}

#[test]
fn init_compose_emit_then_already_initialized() {
    let dir = shared_tempdir();
    let log_path = dir.path().join("observability.jsonl");
    // SAFETY: this binary's only test owns these env vars for the duration
    // of the process. Cargo runs each `tests/<name>.rs` in its own binary.
    std::env::set_var("OBS_FILE_PATH", &log_path);
    std::env::set_var("RUST_LOG", "info");

    let guard =
        init_node_with_web("init_node_with_web_it").expect("init_node_with_web should succeed");

    // Subscribe BEFORE emitting — `broadcast::Sender::send` returns Err with
    // no receivers, and the WEB layer deliberately swallows that error.
    let mut rx = guard.web().subscribe();
    assert_eq!(
        guard.web().receiver_count(),
        1,
        "subscribe() must be observable through the guard's WebHandle",
    );
    assert!(
        guard.ring().capacity() > 0,
        "RingHandle exposed via guard must have capacity",
    );

    {
        let span = tracing::info_span!("init_node_with_web_test");
        let _enter = span.enter();
        tracing::info!(
            user.hash = %"abc123",
            schema.name = %"PhotoMetadata",
            "schema registered",
        );
    }

    // The broadcast send is synchronous on the emitter side, so the receiver
    // sees the JSON string immediately after the macro returns. A small
    // backstop timeout keeps a regression from hanging the test runner.
    let json_str = recv_with_timeout(&mut rx, Duration::from_secs(2))
        .expect("event must reach the WEB broadcast channel");
    let value: Value = serde_json::from_str(&json_str).expect("WEB payload must be JSON");

    assert_eq!(value["level"], "INFO");
    assert_eq!(value["message"], "schema registered");
    let metadata = value["metadata"]
        .as_object()
        .expect("metadata object on WEB payload");
    assert_eq!(
        metadata.get("user.hash").and_then(Value::as_str),
        Some("abc123"),
    );
    assert_eq!(
        metadata.get("schema.name").and_then(Value::as_str),
        Some("PhotoMetadata"),
    );
    let trace_id = metadata
        .get("trace_id")
        .and_then(Value::as_str)
        .expect("OTel layer must populate trace_id on WEB payload");
    assert_eq!(trace_id.len(), 32, "trace_id should be 32-char hex");
    assert!(
        trace_id.chars().all(|c| c.is_ascii_hexdigit()),
        "trace_id should be hex, got {trace_id:?}",
    );
    assert_ne!(
        trace_id,
        &"0".repeat(32),
        "trace_id should not be the all-zero invalid context",
    );

    // RING captured the same event — both layers run on every emission
    // regardless of `with()` order.
    let ring_entries = guard.ring().query(None, None);
    assert_eq!(
        ring_entries.len(),
        1,
        "RING handle exposed via guard must capture the same event",
    );

    // Second init must surface as AlreadyInitialized — same single-init
    // contract as `init_node`.
    let err = init_node_with_web("init_node_with_web_it").expect_err("second init must fail");
    assert!(
        matches!(err, ObsError::AlreadyInitialized),
        "expected AlreadyInitialized, got {err:?}",
    );

    drop(guard);
}

/// Block-receiving from a `tokio::sync::broadcast::Receiver` without dragging
/// in a tokio runtime — `try_recv` plus a small sleep loop bounded by
/// `deadline`. The WEB layer's `send` is synchronous on the producing
/// thread, so in practice this returns on the very first `try_recv`.
fn recv_with_timeout(
    rx: &mut tokio::sync::broadcast::Receiver<String>,
    deadline: Duration,
) -> Option<String> {
    let started = std::time::Instant::now();
    loop {
        match rx.try_recv() {
            Ok(s) => return Some(s),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                if started.elapsed() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
}
