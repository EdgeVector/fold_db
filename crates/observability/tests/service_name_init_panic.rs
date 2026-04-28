//! Phase 5 / T5 — `service.name` is load-bearing.
//!
//! The OTel `service.name` Resource attribute is the primary dimension
//! Honeycomb / Sentry / Loki group spans and events under. Landing in
//! production with `service.name=""` (or a whitespace blob) silently fans
//! every span out under `unknown_service`, which on a busy dashboard masks
//! whichever service has the bug. We chose to fail loudly at boot rather
//! than discover the misconfiguration in the morning.
//!
//! These tests pin two things:
//! 1. `init_node("", _)` panics with a message containing the literal
//!    `service.name`. Operators reading the panic line should see the OTel
//!    attribute key they're missing.
//! 2. `init_node("phase5-t5-success", _)` succeeds AND
//!    [`observability::installed_service_name`] returns the same value —
//!    proof that the post-init Resource verification ran (and so the global
//!    TracerProvider's Resource carries the expected `service.name`).
//!
//! Each `tests/<name>.rs` is its own binary, so the `INIT_ONCE` and
//! `SERVICE_NAME` `OnceCell`s are fresh here and don't contend with the
//! happy-path test in `tests/integration.rs`.

use std::sync::OnceLock;

use observability::{init_node, installed_service_name};
use tempfile::TempDir;

/// Pin a `TempDir` for the lifetime of the test process so the FMT writer's
/// non-blocking worker thread (which keeps `OBS_FILE_PATH` open) does not
/// see the file unlinked while it is still draining its queue. Same pattern
/// as `tests/integration.rs`.
fn shared_tempdir() -> &'static TempDir {
    static DIR: OnceLock<TempDir> = OnceLock::new();
    DIR.get_or_init(|| tempfile::tempdir().expect("create tempdir"))
}

#[test]
#[should_panic(expected = "service.name")]
fn init_node_panics_on_empty_service_name() {
    // `assert_service_name` runs BEFORE `try_claim_init`, so this panic does
    // not consume the process-global INIT_ONCE slot. The success-path test
    // running in parallel in the same binary is therefore safe.
    let _ = init_node("", "0.0.0");
}

#[test]
#[should_panic(expected = "service.name")]
fn init_node_panics_on_whitespace_service_name() {
    // Whitespace-only is treated identically to empty: it would surface as
    // a blank `service.name` on the Resource, which is the failure mode we
    // are trying to prevent.
    let _ = init_node("   \t\n", "0.0.0");
}

#[test]
fn init_node_success_records_service_name_on_global_provider() {
    let dir = shared_tempdir();
    let log_path = dir.path().join("observability.jsonl");

    // SAFETY: this test owns `OBS_FILE_PATH` for this binary. The two panic
    // tests above bail in `assert_service_name` before reading any env, so
    // there is no race even when cargo runs the `#[test]`s in parallel.
    std::env::set_var("OBS_FILE_PATH", &log_path);

    // Use a literal that's distinct from any production service name so a
    // grep across logs makes it obvious where this came from.
    let service: &'static str = "phase5-t5-success";
    let guard = init_node(service, "0.0.0").expect("init_node should succeed");

    // The post-init contract: the same value passed in is what the global
    // TracerProvider's Resource ended up carrying. `installed_service_name`
    // is set in `record_service_name` only after the probe-backed Resource
    // verification in `build_traces_layer_or_noop` has passed, so this
    // equality assertion is also evidence that the verification ran.
    assert_eq!(
        installed_service_name(),
        Some(service),
        "installed_service_name() must reflect the value passed to init_node",
    );

    // Drop the guard so the FMT non-blocking worker drains. Holding it past
    // this point is unnecessary in this test — we are asserting on the
    // post-init invariant, not on log output shape (covered by
    // `tests/integration.rs`).
    drop(guard);
}
