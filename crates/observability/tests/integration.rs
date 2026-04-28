//! End-to-end integration test for the observability crate.
//!
//! Phase 1 / T7 — exit criterion. A single test exercises the full
//! init → emit → capture → reload path that each binary will use at startup:
//!
//! 1. Set `OBS_FILE_PATH` to a tempfile and call `init_node`.
//! 2. Emit a structured event inside a span.
//! 3. Verify the FMT layer wrote a JSON line carrying `service.name`,
//!    `severity_text=INFO`, and the call-site fields.
//! 4. Verify the RING layer captured the event with `trace_id` populated
//!    (proving the OTel layer wired by `init_node` produces real W3C ids).
//! 5. Flip the RELOAD handle to `warn`, re-emit at info, and assert the new
//!    event is suppressed in both ring and file.
//! 6. Confirm a second `init_node` returns `AlreadyInitialized`.
//!
//! `init_node` installs a process-global `tracing` subscriber, which can
//! happen exactly once per process. Cargo runs each `tests/<name>.rs` in its
//! own binary, but `#[test]` functions inside the same file share a process —
//! so the happy path is structured as one test, with the empty-`service_name`
//! panic case in a separate test that bails out before touching the global.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use observability::{init_node, ObsError};
use serde_json::Value;
use tempfile::TempDir;

/// Pin a `TempDir` for the lifetime of the test process so the log path the
/// global subscriber holds open does not get unlinked while the worker thread
/// is still draining its queue.
fn shared_tempdir() -> &'static TempDir {
    static DIR: OnceLock<TempDir> = OnceLock::new();
    DIR.get_or_init(|| tempfile::tempdir().expect("create tempdir"))
}

/// Wait for at least one non-empty JSON line to appear in `path`. The FMT
/// layer's `tracing_appender::non_blocking` worker flushes asynchronously, so
/// even after dropping the guard we may need a beat for the queue to drain on
/// CI runners. 2 s is far above the worst case observed locally and bounds
/// the test against an indefinite hang if the writer ever stops flushing.
fn wait_for_log_lines(path: &std::path::Path, deadline: Duration) -> String {
    let started = Instant::now();
    loop {
        if let Ok(contents) = std::fs::read_to_string(path) {
            if contents.lines().any(|l| !l.is_empty()) {
                return contents;
            }
        }
        if started.elapsed() >= deadline {
            return std::fs::read_to_string(path).unwrap_or_default();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn init_emit_capture_reload_then_already_initialized() {
    let dir = shared_tempdir();
    let log_path = dir.path().join("observability.jsonl");
    // SAFETY: this test owns `OBS_FILE_PATH` for the binary. The other
    // `#[test]` in this file (`empty_service_name_panics`) panics in
    // `assert_service_name` before reading any env, so there is no race.
    std::env::set_var("OBS_FILE_PATH", &log_path);
    // A bare `info` baseline — the RELOAD step below changes it to `warn`.
    std::env::set_var("RUST_LOG", "info");

    let guard = init_node("test_node", "0.0.0").expect("init_node should succeed");

    let ring = guard
        .ring()
        .expect("init_node installs a RING handle")
        .clone();
    let reload = guard.reload().expect("init_node installs a RELOAD handle");

    // ---------------------------------------------------------------------
    // Step 1 — emit one structured event inside a span. The span is the
    // mechanism that gives the RING layer an `OtelData` extension to read,
    // which is how `trace_id` ends up on the entry.
    // ---------------------------------------------------------------------
    {
        let span = tracing::info_span!("integration_test");
        let _enter = span.enter();
        tracing::info!(
            user.hash = %"abc123",
            schema.name = %"PhotoMetadata",
            "schema registered",
        );
    }

    // RING capture is synchronous, so we can assert immediately.
    let entries_before_reload = ring.query(None, None);
    assert_eq!(
        entries_before_reload.len(),
        1,
        "expected exactly one captured event, got {entries_before_reload:?}",
    );
    let entry = &entries_before_reload[0];
    assert_eq!(entry.message, "schema registered");
    assert_eq!(
        entry.level,
        observability::layers::LogLevel::Info,
        "level should round-trip as INFO",
    );

    let metadata = entry
        .metadata
        .as_ref()
        .expect("event metadata should be present");
    assert_eq!(
        metadata.get("user.hash").map(String::as_str),
        Some("abc123")
    );
    assert_eq!(
        metadata.get("schema.name").map(String::as_str),
        Some("PhotoMetadata"),
    );
    let trace_id = metadata
        .get("trace_id")
        .expect("OTel layer must populate trace_id on RING entries");
    assert_eq!(
        trace_id.len(),
        32,
        "trace_id should be 32-char hex, got {trace_id:?}",
    );
    assert!(
        trace_id.chars().all(|c| c.is_ascii_hexdigit()),
        "trace_id should be hex, got {trace_id:?}",
    );
    assert_ne!(
        trace_id,
        &"0".repeat(32),
        "trace_id should not be the all-zero invalid context",
    );

    // ---------------------------------------------------------------------
    // Step 2 — flip the filter to `warn` and emit at `info`. The new event
    // must NOT show up in either sink.
    // ---------------------------------------------------------------------
    reload
        .update("warn")
        .expect("reload to `warn` directive should succeed");
    {
        let span = tracing::info_span!("post_reload");
        let _enter = span.enter();
        tracing::info!("dropped after reload");
    }

    let entries_after_reload = ring.query(None, None);
    assert_eq!(
        entries_after_reload.len(),
        1,
        "info event after RELOAD update(warn) must be filtered out of RING; saw {entries_after_reload:?}",
    );
    assert_eq!(
        entries_after_reload[0].message, "schema registered",
        "RING contents should be unchanged after the suppressed event",
    );

    // ---------------------------------------------------------------------
    // Step 3 — drop the guard so the FMT non-blocking worker flushes its
    // queue, then read the file. We assert positively on the first event's
    // shape and negatively on the suppressed one.
    // ---------------------------------------------------------------------
    drop(guard);

    let contents = wait_for_log_lines(&log_path, Duration::from_secs(2));
    assert!(
        !contents.is_empty(),
        "FMT writer produced no output to {log_path:?}",
    );
    assert!(
        !contents.contains("dropped after reload"),
        "RELOAD update(warn) should have suppressed the post-reload event in the file too; got: {contents}",
    );

    let json_lines: Vec<Value> = contents
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str::<Value>(l).expect("each FMT line should be valid JSON"))
        .collect();
    assert_eq!(
        json_lines.len(),
        1,
        "expected exactly one logged line, got {json_lines:?}",
    );
    let line = &json_lines[0];
    assert_eq!(line["body"], "schema registered");
    assert_eq!(line["severity_text"], "INFO");
    assert_eq!(
        line["service.name"], "test_node",
        "FMT must stamp the service name passed to init_node",
    );
    let attrs = line["attributes"]
        .as_object()
        .expect("attributes object on the formatted line");
    assert_eq!(attrs["user.hash"], "abc123");
    assert_eq!(attrs["schema.name"], "PhotoMetadata");

    // ---------------------------------------------------------------------
    // Step 4 — second init must surface as AlreadyInitialized rather than
    // silently re-installing the subscriber.
    // ---------------------------------------------------------------------
    let err = init_node("test_node", "0.0.0").expect_err("second init_node call must fail");
    assert!(
        matches!(err, ObsError::AlreadyInitialized),
        "expected AlreadyInitialized, got {err:?}",
    );
}

#[test]
#[should_panic(expected = "service.name")]
fn empty_service_name_panics() {
    // `init_node` asserts `service_name` non-empty BEFORE claiming the
    // process-global `INIT_ONCE`, so this test does not interfere with the
    // happy-path test running in parallel in the same binary. Phase 5 / T5
    // strengthened the message to include `service.name` (the OTel attribute
    // key) so the panic surface lines up with what dashboards expect.
    let _ = init_node("", "0.0.0");
}
