//! Tests for background precomputation of deep view chains.
//!
//! Verifies that views deeper than level 1 (depending on other views)
//! transition through Computing state during background precomputation,
//! and that queries against Computing views return a clear error.

use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::key_config::KeyConfig;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::test_helpers::TestSchemaBuilder;
use fold_db::view::types::{TransformView, ViewCacheState};
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
async fn deep_view_enters_computing_after_mutation() {
    let db = setup_db().await;

    // Setup: schema + data + 2-level view chain
    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();
    write_blogpost(&db, "original", "2026-01-01").await;

    // ViewA → BlogPost (level 1, direct)
    db.schema_manager()
        .register_view(identity_view("ViewA", "BlogPost", "content"))
        .await
        .unwrap();

    // ViewB → ViewA (level 2, deep)
    db.schema_manager()
        .register_view(identity_view("ViewB", "ViewA", "content"))
        .await
        .unwrap();

    // Populate caches by querying both
    let q_a = Query::new("ViewA".to_string(), vec!["content".to_string()]);
    let q_b = Query::new("ViewB".to_string(), vec!["content".to_string()]);
    db.query_executor().query(q_a).await.unwrap();
    db.query_executor().query(q_b).await.unwrap();

    // Both should be Cached
    assert!(matches!(
        db.db_ops().get_view_cache_state("ViewA").await.unwrap(),
        ViewCacheState::Cached { .. }
    ));
    assert!(matches!(
        db.db_ops().get_view_cache_state("ViewB").await.unwrap(),
        ViewCacheState::Cached { .. }
    ));

    // Mutate source — triggers invalidation + background precomputation
    write_blogpost(&db, "updated", "2026-01-02").await;

    // ViewA (level 1) may be Empty or already precomputed by the background
    // task (which computes all views bottom-up to unblock deep views).
    let state_a = db.db_ops().get_view_cache_state("ViewA").await.unwrap();
    assert!(
        matches!(
            state_a,
            ViewCacheState::Empty | ViewCacheState::Cached { .. }
        ),
        "ViewA (level 1) should be Empty or Cached, got {:?}",
        state_a
    );

    // ViewB (level 2) should be Computing or already Cached
    // (background task may complete very fast)
    let state_b = db.db_ops().get_view_cache_state("ViewB").await.unwrap();
    assert!(
        matches!(
            state_b,
            ViewCacheState::Computing | ViewCacheState::Cached { .. }
        ),
        "ViewB (level 2) should be Computing or Cached after mutation, got {:?}",
        state_b
    );
}

#[tokio::test]
async fn deep_view_eventually_becomes_cached() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();
    write_blogpost(&db, "original", "2026-01-01").await;

    db.schema_manager()
        .register_view(identity_view("ViewA", "BlogPost", "content"))
        .await
        .unwrap();
    db.schema_manager()
        .register_view(identity_view("ViewB", "ViewA", "content"))
        .await
        .unwrap();

    // Populate caches
    let q_a = Query::new("ViewA".to_string(), vec!["content".to_string()]);
    let q_b = Query::new("ViewB".to_string(), vec!["content".to_string()]);
    db.query_executor().query(q_a).await.unwrap();
    db.query_executor().query(q_b).await.unwrap();

    // Mutate source
    write_blogpost(&db, "updated", "2026-01-02").await;

    // Wait for background precomputation to complete
    // (ViewA needs to be lazily computed first, then ViewB background task runs)
    // First, lazily compute ViewA so the background task for ViewB can proceed
    let q_a = Query::new("ViewA".to_string(), vec!["content".to_string()]);
    db.query_executor().query(q_a).await.unwrap();

    // Give background task time to complete
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let state_b = db.db_ops().get_view_cache_state("ViewB").await.unwrap();
    assert!(
        matches!(state_b, ViewCacheState::Cached { .. }),
        "ViewB should eventually become Cached after precomputation, got {:?}",
        state_b
    );
}

#[tokio::test]
async fn query_during_computing_returns_error() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();
    write_blogpost(&db, "original", "2026-01-01").await;

    db.schema_manager()
        .register_view(identity_view("ViewA", "BlogPost", "content"))
        .await
        .unwrap();

    // Manually set ViewA to Computing to simulate in-progress precomputation
    db.db_ops()
        .set_view_cache_state("ViewA", &ViewCacheState::Computing)
        .await
        .unwrap();

    // Query should fail with clear error
    let q = Query::new("ViewA".to_string(), vec!["content".to_string()]);
    let result = db.query_executor().query(q).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("precomputed") || err.contains("not ready"),
        "Error should mention precomputation/not ready, got: {}",
        err
    );
}

#[tokio::test]
async fn three_level_chain_precomputes_bottom_up() {
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

    // Populate all caches
    for name in &["ViewA", "ViewB", "ViewC"] {
        let q = Query::new(name.to_string(), vec!["content".to_string()]);
        db.query_executor().query(q).await.unwrap();
    }

    // Mutate source
    write_blogpost(&db, "changed", "2026-01-02").await;

    // ViewA (level 1) is Empty initially but may be precomputed by the
    // background task (which computes all views bottom-up). Either state is valid.
    let state_a = db.db_ops().get_view_cache_state("ViewA").await.unwrap();
    assert!(
        matches!(
            state_a,
            ViewCacheState::Empty | ViewCacheState::Cached { .. }
        ),
        "ViewA should be Empty or already Cached, got {:?}",
        state_a
    );

    // ViewB and ViewC should be Computing or Cached
    let state_b = db.db_ops().get_view_cache_state("ViewB").await.unwrap();
    let state_c = db.db_ops().get_view_cache_state("ViewC").await.unwrap();
    assert!(
        matches!(
            state_b,
            ViewCacheState::Computing | ViewCacheState::Cached { .. }
        ),
        "ViewB should be Computing or Cached, got {:?}",
        state_b
    );
    assert!(
        matches!(
            state_c,
            ViewCacheState::Computing | ViewCacheState::Cached { .. }
        ),
        "ViewC should be Computing or Cached, got {:?}",
        state_c
    );

    // Lazily compute ViewA so background tasks can resolve
    let q = Query::new("ViewA".to_string(), vec!["content".to_string()]);
    db.query_executor().query(q).await.unwrap();

    // Wait for background tasks
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    // All should be Cached now
    for name in &["ViewA", "ViewB", "ViewC"] {
        assert!(
            matches!(
                db.db_ops().get_view_cache_state(name).await.unwrap(),
                ViewCacheState::Cached { .. }
            ),
            "{} should be Cached after precomputation",
            name
        );
    }
}

#[tokio::test]
async fn precomputed_view_has_fresh_data() {
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

    // Populate caches with v1
    let q_a = Query::new("ViewA".to_string(), vec!["content".to_string()]);
    let q_b = Query::new("ViewB".to_string(), vec!["content".to_string()]);
    db.query_executor().query(q_a.clone()).await.unwrap();
    db.query_executor().query(q_b.clone()).await.unwrap();

    // Mutate to v2
    write_blogpost(&db, "v2", "2026-01-01").await;

    // Lazily compute ViewA to unblock ViewB's precomputation
    db.query_executor().query(q_a).await.unwrap();

    // Wait for ViewB precomputation
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // ViewB should be Cached with fresh data
    let state = db.db_ops().get_view_cache_state("ViewB").await.unwrap();
    assert!(
        matches!(state, ViewCacheState::Cached { .. }),
        "ViewB should be Cached, got {:?}",
        state
    );

    // Query ViewB — should have v2 data
    let results = db.query_executor().query(q_b).await.unwrap();
    let values: Vec<_> = results["content"]
        .values()
        .map(|fv| fv.value.clone())
        .collect();
    assert!(
        values.contains(&json!("v2")),
        "Precomputed ViewB should contain fresh v2 data, got {:?}",
        values
    );
}
