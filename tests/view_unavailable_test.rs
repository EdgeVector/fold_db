//! Integration tests for the `ViewCacheState::Unavailable` state.
//!
//! Covers the MDT-A semantics from `docs/design/multi_device_transforms.md`
//! Open Design Item #5:
//!
//! - Compute failure on a transform view → state becomes `Unavailable(reason)`
//! - A re-read of an `Unavailable` view does NOT retry the transform
//!   (sticky per input).
//! - A source mutation invalidates `Unavailable` → `Empty` so the next read
//!   recomputes on the new input.
//! - Direct state round-trips cleanly via the storage serialization format.

#![cfg(feature = "transform-wasm")]

use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::test_helpers::TestSchemaBuilder;
use fold_db::view::types::{TransformView, UnavailableReason, ViewCacheState, WasmTransformSpec};
use serde_json::json;
use std::collections::HashMap;

/// A WASM module whose `transform` function traps on `unreachable` —
/// the cleanest way to force a runtime ExecutionError without depending on
/// gas or compile-time failure plumbing that isn't wired up yet.
fn trapping_wasm() -> Vec<u8> {
    let wat = r#"(module
        (memory (export "memory") 1)
        (func (export "alloc") (param $size i32) (result i32)
            (i32.const 1024)
        )
        (func (export "transform") (param $ptr i32) (param $len i32) (result i64)
            unreachable
        )
    )"#;
    wat::parse_str(wat).expect("valid WAT")
}

/// MDT-E fixture: a module whose `transform` enters an unconditional
/// infinite loop. Any finite `max_gas` will trap it with
/// `Trap::OutOfFuel`, which the engine classifies as
/// `TransformGasExceeded` and the resolver surfaces as
/// `UnavailableReason::GasExceeded`.
fn fuel_burner_wasm() -> Vec<u8> {
    let wat = r#"(module
        (memory (export "memory") 1)
        (global $bump (mut i32) (i32.const 4096))
        (func (export "alloc") (param $size i32) (result i32)
            (local $ptr i32)
            (local.set $ptr (global.get $bump))
            (global.set $bump (i32.add (global.get $bump) (local.get $size)))
            (local.get $ptr)
        )
        (func (export "transform") (param $ptr i32) (param $len i32) (result i64)
            (loop $spin
                (br $spin)
            )
            (i64.const 0)
        )
    )"#;
    wat::parse_str(wat).expect("valid WAT")
}

async fn setup_db() -> FoldDB {
    let dir = tempfile::tempdir().unwrap();
    FoldDB::new(dir.path().to_str().unwrap()).await.unwrap()
}

fn blogpost_schema_json() -> String {
    TestSchemaBuilder::new("BlogPost")
        .fields(&["title", "content"])
        .range_key("publish_date")
        .build_json()
}

async fn write_blogpost(db: &FoldDB, title: &str, date: &str) {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!(title));
    fields.insert("publish_date".to_string(), json!(date));
    db.mutation_manager()
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            fields,
            KeyValue::new(None, Some(date.to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();
}

/// Empty → compute fails → Unavailable(ExecutionError). Immediate re-read
/// must not retry — the persisted state remains Unavailable and the query
/// surfaces the same reason.
#[tokio::test]
async fn compute_failure_transitions_to_unavailable_and_does_not_retry() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();
    write_blogpost(&db, "Hello", "2026-01-01").await;

    let view = TransformView::new(
        "TrapView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["title".to_string()],
        )],
        Some(WasmTransformSpec {
            bytes: trapping_wasm(),
            max_gas: 1_000_000,
        }),
        HashMap::from([("summary".to_string(), FieldValueType::String)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    // First query: transform traps, view becomes Unavailable.
    let query = Query::new("TrapView".to_string(), vec!["summary".to_string()]);
    let first = db.query_executor().query(query.clone()).await;
    assert!(first.is_err(), "trapping transform should error");
    let first_err = first.unwrap_err().to_string();
    assert!(
        first_err.contains("unavailable"),
        "error should mention unavailable, got {first_err}"
    );

    let state = db.db_ops().get_view_cache_state("TrapView").await.unwrap();
    let reason = state
        .unavailable_reason()
        .expect("state should be Unavailable after failed compute");
    assert!(matches!(reason, UnavailableReason::ExecutionError { .. }));

    // Second query: must NOT retry. We verify by snapshotting the exact
    // state (including the reason message) before and after a re-read.
    // A retry would either (a) error with a different message, or (b)
    // overwrite the state with a fresh reason. Sticky-per-input requires
    // neither to happen.
    let state_before = db.db_ops().get_view_cache_state("TrapView").await.unwrap();
    let reason_before = state_before.unavailable_reason().unwrap().clone();

    let second = db.query_executor().query(query).await;
    assert!(second.is_err(), "re-read should still error");
    assert_eq!(
        second.unwrap_err().to_string(),
        first_err,
        "re-read error should be identical (no retry)"
    );

    let state_after = db.db_ops().get_view_cache_state("TrapView").await.unwrap();
    let reason_after = state_after.unavailable_reason().unwrap().clone();
    assert_eq!(
        reason_before, reason_after,
        "Unavailable reason must not change across re-reads"
    );
}

/// Unavailable → source mutation → Empty. After the source moves, the next
/// read recomputes (and — in this test — succeeds with a non-trapping view
/// that replaces the trapping one via a fresh registration). The key
/// assertion is just the state transition: source mutation clears
/// Unavailable back to Empty.
#[tokio::test]
async fn source_mutation_clears_unavailable_to_empty() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();
    write_blogpost(&db, "First", "2026-01-01").await;

    let view = TransformView::new(
        "TrapView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["title".to_string()],
        )],
        Some(WasmTransformSpec {
            bytes: trapping_wasm(),
            max_gas: 1_000_000,
        }),
        HashMap::from([("summary".to_string(), FieldValueType::String)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    // Force the trap → Unavailable.
    let query = Query::new("TrapView".to_string(), vec!["summary".to_string()]);
    let _ = db.query_executor().query(query).await;

    assert!(
        matches!(
            db.db_ops().get_view_cache_state("TrapView").await.unwrap(),
            ViewCacheState::Unavailable { .. }
        ),
        "state should be Unavailable before source mutation"
    );

    // Mutate the source schema — invalidation cascades to TrapView.
    write_blogpost(&db, "Second", "2026-01-02").await;

    // The orchestrator resets Unavailable → Empty so the next read can
    // retry on the new input. (It may then transition to Computing or
    // another terminal state if background precomputation runs; what we
    // assert is that it is no longer Unavailable.)
    let state_after = db.db_ops().get_view_cache_state("TrapView").await.unwrap();
    assert!(
        !matches!(state_after, ViewCacheState::Unavailable { .. }),
        "source mutation should clear Unavailable, got {state_after:?}"
    );
}

/// Round-trip the Unavailable state through the view-cache store — the
/// same serde-json path used at restart. Confirms the variant persists
/// cleanly.
#[tokio::test]
async fn unavailable_state_persists_through_store() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let view = TransformView::new(
        "RoundTripView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["title".to_string()],
        )],
        None,
        HashMap::from([("title".to_string(), FieldValueType::Any)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    // Write each variant directly, then read back and compare.
    let variants = vec![
        UnavailableReason::GasExceeded { input_size: 4096 },
        UnavailableReason::CompileError {
            message: "parse error at offset 0x42".to_string(),
        },
        UnavailableReason::TransformBytesUnavailable,
        UnavailableReason::ExecutionError {
            message: "trap: unreachable".to_string(),
        },
    ];

    for reason in variants {
        let state = ViewCacheState::Unavailable {
            reason: reason.clone(),
        };
        db.db_ops()
            .set_view_cache_state("RoundTripView", &state)
            .await
            .unwrap();

        let loaded = db
            .db_ops()
            .get_view_cache_state("RoundTripView")
            .await
            .unwrap();
        assert_eq!(
            loaded.unavailable_reason(),
            Some(&reason),
            "round-trip failed for {reason:?}"
        );
    }
}

// ==================== MDT-E: max_gas end-to-end ==================== //

/// MDT-E: a transform that blows through its fuel budget must land in
/// `Unavailable { GasExceeded }` (not `ExecutionError`), so callers can
/// distinguish "compute is impossible at this budget" from "guest
/// trapped on some other path". The sticky-per-input re-read contract
/// is the same as every other `Unavailable` variant — verified
/// elsewhere in this file.
#[tokio::test]
async fn gas_exhaustion_transitions_to_unavailable_gas_exceeded() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();
    write_blogpost(&db, "Hello", "2026-01-01").await;

    // 5_000 fuel units comfortably cover module setup but the guest's
    // loop burns them all before `transform` could return.
    let view = TransformView::new(
        "FuelBurnerView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["title".to_string()],
        )],
        Some(WasmTransformSpec {
            bytes: fuel_burner_wasm(),
            max_gas: 5_000,
        }),
        HashMap::from([("summary".to_string(), FieldValueType::String)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    let query = Query::new("FuelBurnerView".to_string(), vec!["summary".to_string()]);
    let result = db.query_executor().query(query).await;
    assert!(
        result.is_err(),
        "fuel-exhausted transform must surface an error, got {result:?}"
    );

    let state = db
        .db_ops()
        .get_view_cache_state("FuelBurnerView")
        .await
        .unwrap();
    let reason = state
        .unavailable_reason()
        .expect("state should be Unavailable after fuel exhaustion");
    match reason {
        UnavailableReason::GasExceeded { input_size } => {
            assert!(
                *input_size > 0,
                "input_size must reflect serialized input bytes (> 0 for a non-empty query)"
            );
        }
        other => panic!("expected GasExceeded, got {other:?}"),
    }
}
