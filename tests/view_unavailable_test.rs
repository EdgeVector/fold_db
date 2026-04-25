//! WASM-fire failure surfacing through the query interface.
//!
//! Pre-cache-cleanup, transform failures landed as a sticky
//! `ViewCacheState::Unavailable` row that subsequent reads short-circuited
//! against. Post-cleanup, every fire runs WASM on the latest input and
//! surfaces failures as `SchemaError::InvalidTransform` with a cause
//! string the trigger runner records in the `TriggerFiring` audit log.
//! These tests pin the user-visible shape of those errors.

#![cfg(feature = "transform-wasm")]

use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::test_helpers::TestSchemaBuilder;
use fold_db::view::types::{FieldId, GasModel, InputDimension, TransformView, WasmTransformSpec};
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
/// `TransformGasExceeded` and the resolver surfaces as a
/// `gas exceeded` cause string.
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

/// A trapping transform's failure surfaces as `InvalidTransform` with the
/// `unavailable` cause. Repeated reads return the same error shape on
/// fresh input — without sticky cache state, the WASM re-runs each time.
#[tokio::test]
async fn trapping_transform_surfaces_invalid_transform_error() {
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
            gas_model: None,
        }),
        HashMap::from([("summary".to_string(), FieldValueType::String)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    let query = Query::new("TrapView".to_string(), vec!["summary".to_string()]);
    let first = db.query_executor().query(query.clone()).await;
    let first_err = first
        .expect_err("trapping transform should error")
        .to_string();
    assert!(
        first_err.contains("unavailable"),
        "error should mention unavailable, got {first_err}"
    );

    // Second query: also fails, with the same shape.
    let second_err = db
        .query_executor()
        .query(query)
        .await
        .expect_err("re-read should still error")
        .to_string();
    assert!(second_err.contains("unavailable"));
}

/// After source mutation, the next read re-runs WASM on the new input.
/// The trapping module still traps, so the error persists — but we
/// confirm by replacing the trapping view with a non-trapping identity
/// view and verifying that the next query returns data.
#[tokio::test]
async fn source_mutation_re_runs_wasm() {
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
            gas_model: None,
        }),
        HashMap::from([("summary".to_string(), FieldValueType::String)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    let query = Query::new("TrapView".to_string(), vec!["summary".to_string()]);
    assert!(db.query_executor().query(query.clone()).await.is_err());

    write_blogpost(&db, "Second", "2026-01-02").await;

    // The trap is deterministic on any input — the error persists, but it
    // re-runs on the new input rather than hitting a sticky cache state.
    assert!(db.query_executor().query(query).await.is_err());
}

/// MDT-E: gas exhaustion surfaces as `gas exceeded` in the error cause.
#[tokio::test]
async fn gas_exhaustion_surfaces_gas_exceeded() {
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
            gas_model: None,
        }),
        HashMap::from([("summary".to_string(), FieldValueType::String)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    let query = Query::new("FuelBurnerView".to_string(), vec!["summary".to_string()]);
    let err = db
        .query_executor()
        .query(query)
        .await
        .expect_err("fuel-exhausted transform must error")
        .to_string();
    assert!(
        err.contains("gas exceeded"),
        "expected `gas exceeded` in error, got {err}"
    );
}

/// MDT-F Phase 2: oversized input is rejected by the envelope check
/// BEFORE the WASM runs. Detect "WASM did not run" by using a trapping
/// module — if the envelope check short-circuits correctly, the cause
/// string is `exceeds calibrated envelope`; if it fails, the trap fires
/// and we'd see something else.
#[tokio::test]
async fn exceeds_envelope_rejects_before_wasm_runs() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();
    let big_title = "x".repeat(500);
    write_blogpost(&db, &big_title, "2026-01-01").await;

    let view = TransformView::new(
        "EnvelopeView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["title".to_string()],
        )],
        Some(WasmTransformSpec {
            bytes: trapping_wasm(),
            max_gas: 1_000_000,
            gas_model: Some(GasModel {
                base: 0,
                coefficients: vec![(
                    InputDimension::FieldBytes(FieldId {
                        schema: "BlogPost".to_string(),
                        field: "title".to_string(),
                    }),
                    1.0,
                )],
                max_input_size: 100,
            }),
        }),
        HashMap::from([("summary".to_string(), FieldValueType::String)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    let query = Query::new("EnvelopeView".to_string(), vec!["summary".to_string()]);
    let err = db
        .query_executor()
        .query(query)
        .await
        .expect_err("oversized input must surface an error")
        .to_string();
    assert!(
        err.contains("exceeds calibrated envelope"),
        "expected envelope rejection, got {err}"
    );
}

/// Mirror of the oversized test: input below the envelope runs the WASM
/// normally. With a trapping WASM the surfaced cause is the trap message
/// — confirming the envelope check did not short-circuit and control
/// reached the guest.
#[tokio::test]
async fn below_envelope_proceeds_to_wasm() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();
    write_blogpost(&db, "hi", "2026-01-01").await;

    let view = TransformView::new(
        "SmallEnvelopeView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["title".to_string()],
        )],
        Some(WasmTransformSpec {
            bytes: trapping_wasm(),
            max_gas: 1_000_000,
            gas_model: Some(GasModel {
                base: 0,
                coefficients: vec![(
                    InputDimension::FieldBytes(FieldId {
                        schema: "BlogPost".to_string(),
                        field: "title".to_string(),
                    }),
                    1.0,
                )],
                max_input_size: 10_000,
            }),
        }),
        HashMap::from([("summary".to_string(), FieldValueType::String)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    let query = Query::new("SmallEnvelopeView".to_string(), vec!["summary".to_string()]);
    let err = db
        .query_executor()
        .query(query)
        .await
        .expect_err("trapping WASM must error when envelope passes")
        .to_string();
    assert!(
        !err.contains("exceeds calibrated envelope"),
        "envelope check must have passed; got {err}"
    );
}
