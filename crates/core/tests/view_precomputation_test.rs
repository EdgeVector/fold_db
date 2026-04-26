//! View chain freshness through trigger-driven cascade fires.
//!
//! Pre-cache-cleanup this file tested the `ViewCacheState::Computing`
//! background-precompute state machine. After the cleanup, fires are
//! synchronous and cascades flow through the trigger system (each level
//! fires its dependents when its derived atoms land). The remaining
//! externally-observable contract is: after a source mutation, queries
//! on every level of the chain reflect the new value.

use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::key_config::KeyConfig;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::test_helpers::TestSchemaBuilder;
use fold_db::view::types::TransformView;
use serde_json::json;
use std::collections::HashMap;

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

fn identity_view(name: &str, source_schema: &str, source_field: &str) -> TransformView {
    TransformView::new(
        name,
        SchemaType::Range,
        Some(KeyConfig::new(None, Some("publish_date".to_string()))),
        vec![Query::new(
            source_schema.to_string(),
            vec![source_field.to_string()],
        )],
        None,
        HashMap::from([(source_field.to_string(), FieldValueType::Any)]),
    )
}

async fn write_blogpost(db: &FoldDB, content: &str, date: &str) {
    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!(content));
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

#[tokio::test]
async fn three_level_chain_query_returns_source_data() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();
    write_blogpost(&db, "deep", "2026-01-01").await;

    // ViewA → BlogPost, ViewB → ViewA, ViewC → ViewB
    db.schema_manager()
        .register_view(identity_view("ViewA", "BlogPost", "content"))
        .await
        .unwrap();
    db.schema_manager()
        .register_view(identity_view("ViewB", "ViewA", "content"))
        .await
        .unwrap();
    db.schema_manager()
        .register_view(identity_view("ViewC", "ViewB", "content"))
        .await
        .unwrap();

    // Allow trigger cascade to settle (each level dual-writes derived
    // atoms; the trigger dispatcher fires the next level when those
    // atoms land).
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Each level returns the source value through chain resolution
    // (cold paths fall back to fire_view via the orchestrator).
    for name in &["ViewA", "ViewB", "ViewC"] {
        let q = Query::new(name.to_string(), vec!["content".to_string()]);
        let res = db.query_executor().query(q).await.unwrap();
        let values: Vec<_> = res["content"].values().map(|fv| fv.value.clone()).collect();
        assert!(
            values.contains(&json!("deep")),
            "{} should resolve to source value, got {:?}",
            name,
            values
        );
    }
}

#[tokio::test]
async fn deep_view_reflects_source_after_mutation() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();
    write_blogpost(&db, "v1", "2026-01-01").await;

    db.schema_manager()
        .register_view(identity_view("ViewA", "BlogPost", "content"))
        .await
        .unwrap();
    db.schema_manager()
        .register_view(identity_view("ViewB", "ViewA", "content"))
        .await
        .unwrap();

    // Prime
    let q_a = Query::new("ViewA".to_string(), vec!["content".to_string()]);
    let q_b = Query::new("ViewB".to_string(), vec!["content".to_string()]);
    db.query_executor().query(q_a.clone()).await.unwrap();
    db.query_executor().query(q_b.clone()).await.unwrap();

    // Mutate source
    write_blogpost(&db, "v2", "2026-01-01").await;

    // Allow trigger cascade to settle.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // ViewB must surface the updated value.
    let res = db.query_executor().query(q_b).await.unwrap();
    let values: Vec<_> = res["content"].values().map(|fv| fv.value.clone()).collect();
    assert!(
        values.contains(&json!("v2")),
        "ViewB should reflect updated source, got {:?}",
        values
    );
}
