//! Integration tests for WASM transform views through the full stack.
//! Only run when the `transform-wasm` feature is enabled.
#![cfg(feature = "transform-wasm")]

use fold_db::db_operations::native_index::FastEmbedModel;
use fold_db::fold_db_core::FoldDB;
use std::sync::Arc;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::view::types::TransformView;
use serde_json::json;
use std::collections::HashMap;

fn wat_to_wasm(wat: &str) -> Vec<u8> {
    wat::parse_str(wat).expect("valid WAT")
}

/// WASM module that returns a hardcoded output regardless of input.
/// Output: {"fields": {"summary": {"k1": "hardcoded"}}}
fn hardcoded_wasm() -> Vec<u8> {
    let output = r#"{"fields":{"summary":{"k1":"hardcoded"}}}"#;
    let output_bytes = output.as_bytes();
    let len = output_bytes.len();
    let escaped = output_bytes
        .iter()
        .map(|b| format!("\\{:02x}", b))
        .collect::<String>();

    let wat = format!(
        r#"(module
            (memory (export "memory") 1)
            (data (i32.const 1024) "{escaped}")
            (global $bump (mut i32) (i32.const 2048))
            (func (export "alloc") (param $size i32) (result i32)
                (local $ptr i32)
                (local.set $ptr (global.get $bump))
                (global.set $bump (i32.add (global.get $bump) (local.get $size)))
                (local.get $ptr)
            )
            (func (export "transform") (param $ptr i32) (param $len i32) (result i64)
                (i64.or
                    (i64.shl (i64.extend_i32_u (i32.const 1024)) (i64.const 32))
                    (i64.extend_i32_u (i32.const {len}))
                )
            )
        )"#,
    );
    wat_to_wasm(&wat)
}

async fn setup_db() -> FoldDB {
    let dir = tempfile::tempdir().unwrap();
    FoldDB::new(dir.path().to_str().unwrap(), Arc::new(FastEmbedModel::new())).await.unwrap()
}

fn blogpost_schema_json() -> &'static str {
    r#"{
        "name": "BlogPost",
        "key": { "range_field": "publish_date" },
        "fields": {
            "title": {},
            "content": {},
            "publish_date": {}
        }
    }"#
}

#[tokio::test]
async fn wasm_view_query_returns_transformed_output() {
    let mut db = setup_db().await;

    // Setup schema with data
    db.load_schema_from_json(blogpost_schema_json()).await.unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!("Hello World"));
    fields.insert("content".to_string(), json!("Test content"));
    fields.insert("publish_date".to_string(), json!("2026-01-01"));
    db.mutation_manager
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            fields,
            KeyValue::new(None, Some("2026-01-01".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();

    // Register a WASM view with hardcoded output
    let view = TransformView::new(
        "SummaryView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["title".to_string(), "content".to_string()],
        )],
        Some(hardcoded_wasm()),
        HashMap::from([("summary".to_string(), FieldValueType::String)]),
    );
    db.schema_manager.register_view(view).await.unwrap();

    // Query the view
    let query = Query::new("SummaryView".to_string(), vec!["summary".to_string()]);
    let results = db.query_executor.query(query).await.unwrap();

    assert!(results.contains_key("summary"));
    let summary_values = &results["summary"];
    assert!(!summary_values.is_empty());
    let value = summary_values.values().next().unwrap();
    assert_eq!(value.value, json!("hardcoded"));
}

#[tokio::test]
async fn wasm_view_output_type_validation_works() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json()).await.unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!("Hello"));
    fields.insert("publish_date".to_string(), json!("2026-01-01"));
    db.mutation_manager
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            fields,
            KeyValue::new(None, Some("2026-01-01".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();

    // Register a WASM view that outputs a string, but declare it as Integer
    let view = TransformView::new(
        "BadTypeView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["title".to_string()],
        )],
        Some(hardcoded_wasm()), // Returns {"summary": {"k1": "hardcoded"}} — a String
        HashMap::from([("summary".to_string(), FieldValueType::Integer)]), // Declared as Integer
    );
    db.schema_manager.register_view(view).await.unwrap();

    // Query should fail with type validation error
    let query = Query::new("BadTypeView".to_string(), vec!["summary".to_string()]);
    let result = db.query_executor.query(query).await;
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("type validation"),
        "Should fail with type validation error"
    );
}

#[tokio::test]
async fn wasm_view_cache_invalidation_works() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json()).await.unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!("Original"));
    fields.insert("publish_date".to_string(), json!("2026-01-01"));
    db.mutation_manager
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            fields,
            KeyValue::new(None, Some("2026-01-01".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();

    // Register WASM view
    let view = TransformView::new(
        "WasmCacheView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["title".to_string()],
        )],
        Some(hardcoded_wasm()),
        HashMap::from([("summary".to_string(), FieldValueType::String)]),
    );
    db.schema_manager.register_view(view).await.unwrap();

    // First query: populates cache
    let query = Query::new("WasmCacheView".to_string(), vec!["summary".to_string()]);
    db.query_executor.query(query.clone()).await.unwrap();

    // Verify cached
    let state = db.db_ops.get_view_cache_state("WasmCacheView").await.unwrap();
    assert!(
        matches!(state, fold_db::view::ViewCacheState::Cached { .. }),
        "View should be cached"
    );

    // Mutate source
    let mut fields2 = HashMap::new();
    fields2.insert("title".to_string(), json!("Updated"));
    fields2.insert("publish_date".to_string(), json!("2026-01-02"));
    db.mutation_manager
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            fields2,
            KeyValue::new(None, Some("2026-01-02".to_string())),
            "pk".to_string(),
            MutationType::Update,
        )])
        .await
        .unwrap();

    // Cache should be invalidated
    let state2 = db.db_ops.get_view_cache_state("WasmCacheView").await.unwrap();
    assert!(
        matches!(state2, fold_db::view::ViewCacheState::Empty),
        "View cache should be invalidated after source mutation"
    );
}
