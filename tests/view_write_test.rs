use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::key_config::KeyConfig;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::test_helpers::TestSchemaBuilder;
use fold_db::view::transform_field_override::TransformFieldOverride;
use fold_db::view::types::{TransformView, ViewCacheState, WasmTransformSpec};
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

/// Identity view write redirects to the source schema —
/// writing to the view should land data in BlogPost.
#[tokio::test]
async fn identity_write_redirects_to_source() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let view = identity_view("WriteView", "BlogPost", "title");
    db.schema_manager().register_view(view).await.unwrap();

    // Verify the view is identity and has a valid source_field_map
    let stored_view = db.schema_manager().get_view("WriteView").unwrap().unwrap();
    assert!(stored_view.is_identity());
    let field_map = stored_view.source_field_map().unwrap();
    assert_eq!(
        field_map.get("title").unwrap(),
        &("BlogPost".to_string(), "title".to_string()),
    );

    // Write to the VIEW — should redirect to BlogPost.title
    let mut mutation_fields = HashMap::new();
    mutation_fields.insert("title".to_string(), json!("Written via view"));
    let mutation = Mutation::new(
        "WriteView".to_string(),
        mutation_fields,
        KeyValue::new(None, Some("2026-01-01".to_string())),
        "pk".to_string(),
        MutationType::Create,
    );
    db.mutation_manager()
        .write_mutations_batch_async(vec![mutation])
        .await
        .unwrap();

    // Query the SOURCE schema to verify the write landed there
    let query = Query::new("BlogPost".to_string(), vec!["title".to_string()]);
    let results = db.query_executor().query(query).await.unwrap();

    assert!(
        results.contains_key("title"),
        "BlogPost should have title field in results"
    );
    let title_values = &results["title"];
    assert!(
        !title_values.is_empty(),
        "Should have data written to BlogPost.title"
    );
    let values: Vec<_> = title_values.values().map(|fv| fv.value.clone()).collect();
    assert!(
        values.contains(&json!("Written via view")),
        "BlogPost.title should contain value written via view, got {:?}",
        values
    );
}

/// WASM (irreversible) views can't redirect to source — direct writes
/// instead persist a `TransformFieldOverride` molecule per
/// `transform_views_design.md` (Overridden state). The mutation succeeds,
/// no source data is touched, and the override is stored under the view's
/// own override namespace ready to be replayed across replicas.
#[tokio::test]
async fn wasm_view_write_persists_override() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let view = TransformView::new(
        "WasmView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["content".to_string()],
        )],
        Some(WasmTransformSpec {
            bytes: vec![0, 1, 2],
            max_gas: 1_000_000,
        }), // Placeholder WASM — never executed on the write path.
        HashMap::from([("out".to_string(), FieldValueType::String)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    let stored = db.schema_manager().get_view("WasmView").unwrap().unwrap();
    assert!(!stored.is_identity());
    assert!(stored.source_field_map().is_none());

    let mut mutation_fields = HashMap::new();
    mutation_fields.insert("out".to_string(), json!("manual override"));
    let key = KeyValue::new(None, Some("2026-01-01".to_string()));
    let mutation = Mutation::new(
        "WasmView".to_string(),
        mutation_fields,
        key.clone(),
        "writer-pk".to_string(),
        MutationType::Create,
    );
    db.mutation_manager()
        .write_mutations_batch_async(vec![mutation])
        .await
        .expect("WASM view writes should now persist as overrides");

    let stored_override = db
        .db_ops()
        .views()
        .get_transform_field_override("WasmView", "out", &key.to_string())
        .await
        .unwrap()
        .expect("override molecule must be stored");
    assert_eq!(stored_override.value, json!("manual override"));
    assert!(stored_override.source_link_stale);
    assert_eq!(stored_override.writer_pubkey, "writer-pk");
}

/// Override molecule wins over identity-view source data on the read path.
/// Once the override exists, subsequent source mutations do NOT recover the
/// source value — the `Overridden` state is sticky per the 3-state machine.
///
/// This is the MDT-D "source-vs-override" scenario, expressed against an
/// identity view so the read path doesn't depend on the optional
/// `transform-wasm` feature for execution. The override consultation logic
/// in `ViewResolver::resolve_with_overrides` is the same code that runs for
/// WASM views; the source-stickiness behavior is identical.
#[tokio::test]
async fn override_supersedes_source_and_sticks_across_source_mutations() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    // Identity view over BlogPost.content keyed by publish_date.
    let view = identity_view("OverrideStickyView", "BlogPost", "content");
    db.schema_manager().register_view(view).await.unwrap();

    let key = KeyValue::new(None, Some("2026-04-23".to_string()));

    // Source write at "earlier" — this lands in BlogPost.content.
    let mut source_fields = HashMap::new();
    source_fields.insert("content".to_string(), json!("from source"));
    source_fields.insert("publish_date".to_string(), json!("2026-04-23"));
    db.mutation_manager()
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            source_fields,
            key.clone(),
            "device_a".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();

    // Override at "later" — emulating a deliberate user pin from device B
    // that wins LWW. We persist directly through the store rather than via
    // a mutation so identity write-redirection doesn't bypass the override.
    let override_mol = TransformFieldOverride::with_timestamp(
        json!("from override"),
        "device_b",
        2_000_000_000_000_000_000,
    );
    db.db_ops()
        .views()
        .put_transform_field_override(
            "OverrideStickyView",
            "content",
            &key.to_string(),
            &override_mol,
        )
        .await
        .unwrap();

    // Read should return the override, not the source value.
    let query = Query::new(
        "OverrideStickyView".to_string(),
        vec!["content".to_string()],
    );
    let results = db.query_executor().query(query.clone()).await.unwrap();
    let values: Vec<_> = results["content"]
        .values()
        .map(|fv| fv.value.clone())
        .collect();
    assert!(
        values.contains(&json!("from override")),
        "Override must beat source on read; got {:?}",
        values
    );
    assert!(
        !values.contains(&json!("from source")),
        "Source value must not appear once the override is in place; got {:?}",
        values
    );

    // A subsequent source mutation arriving "after" the override must not
    // unstick it — the override stays.
    let mut updated_source = HashMap::new();
    updated_source.insert("content".to_string(), json!("source updated again"));
    updated_source.insert("publish_date".to_string(), json!("2026-04-23"));
    db.mutation_manager()
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            updated_source,
            key.clone(),
            "device_a".to_string(),
            MutationType::Update,
        )])
        .await
        .unwrap();

    let results = db.query_executor().query(query).await.unwrap();
    let values: Vec<_> = results["content"]
        .values()
        .map(|fv| fv.value.clone())
        .collect();
    assert!(
        values.contains(&json!("from override")),
        "Override must remain after source mutation (sticky); got {:?}",
        values
    );
    assert!(
        !values.contains(&json!("source updated again")),
        "Refreshed source must not appear; got {:?}",
        values
    );
}

/// Two-device concurrent override scenario from the design doc:
/// device A writes override at t=100, device B writes override at t=101 —
/// both devices converge on B's value once the log replays. Order of
/// `write_mutations_batch_async` calls must not affect the outcome; we run
/// the same scenario both ways to prove convergence.
#[tokio::test]
async fn concurrent_overrides_converge_via_lww() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let view = TransformView::new(
        "ConcurrentView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["content".to_string()],
        )],
        Some(WasmTransformSpec {
            bytes: vec![0, 1, 2],
            max_gas: 1_000_000,
        }),
        HashMap::from([("out".to_string(), FieldValueType::String)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    let key = KeyValue::new(None, Some("k".to_string()));
    let key_str = key.to_string();
    let store = db.db_ops().views();

    // Device A at t=100 (older). Device B at t=101 (newer).
    let device_a = TransformFieldOverride::with_timestamp(json!("A"), "pk_a", 100);
    let device_b = TransformFieldOverride::with_timestamp(json!("B"), "pk_b", 101);

    // Order 1: A then B → B wins.
    store
        .put_transform_field_override("ConcurrentView", "out", &key_str, &device_a)
        .await
        .unwrap();
    store
        .put_transform_field_override("ConcurrentView", "out", &key_str, &device_b)
        .await
        .unwrap();
    let got = store
        .get_transform_field_override("ConcurrentView", "out", &key_str)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.value, json!("B"));

    // Order 2: clear, then B then A → still B (replaying older log entry
    // is a no-op).
    store
        .clear_transform_field_overrides("ConcurrentView")
        .await
        .unwrap();
    store
        .put_transform_field_override("ConcurrentView", "out", &key_str, &device_b)
        .await
        .unwrap();
    store
        .put_transform_field_override("ConcurrentView", "out", &key_str, &device_a)
        .await
        .unwrap();
    let got = store
        .get_transform_field_override("ConcurrentView", "out", &key_str)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.value, json!("B"));
    assert_eq!(got.writer_pubkey, "pk_b");
    assert_eq!(got.written_at, 101);
}

/// Writing through a view should invalidate that view's cache so
/// subsequent queries return fresh data.
#[tokio::test]
async fn write_through_view_invalidates_cache() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    // Seed initial data
    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!("original"));
    fields.insert("publish_date".to_string(), json!("2026-01-01"));
    db.mutation_manager()
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            fields,
            KeyValue::new(None, Some("2026-01-01".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();

    let view = identity_view("CacheWrite", "BlogPost", "content");
    db.schema_manager().register_view(view).await.unwrap();

    // Query the view to populate cache
    let query = Query::new("CacheWrite".to_string(), vec!["content".to_string()]);
    db.query_executor().query(query.clone()).await.unwrap();

    // Cache should be populated
    assert!(matches!(
        db.db_ops()
            .get_view_cache_state("CacheWrite")
            .await
            .unwrap(),
        ViewCacheState::Cached { .. }
    ));

    // Write through the view — should invalidate cache
    let mut mutation_fields = HashMap::new();
    mutation_fields.insert("content".to_string(), json!("updated via view"));
    db.mutation_manager()
        .write_mutations_batch_async(vec![Mutation::new(
            "CacheWrite".to_string(),
            mutation_fields,
            KeyValue::new(None, Some("2026-01-02".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();

    // Re-query — should return fresh data including the new write
    let results = db.query_executor().query(query).await.unwrap();
    let values: Vec<_> = results["content"]
        .values()
        .map(|fv| fv.value.clone())
        .collect();
    assert!(
        values.contains(&json!("updated via view")),
        "View should return fresh data after write-through, got {:?}",
        values
    );
}

/// Writing through a view that has downstream dependents should cascade-
/// invalidate the entire chain (ViewB depends on ViewA which wraps BlogPost).
#[tokio::test]
async fn write_through_view_cascades_invalidation() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    // Seed data
    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!("v1"));
    fields.insert("publish_date".to_string(), json!("2026-01-01"));
    db.mutation_manager()
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            fields,
            KeyValue::new(None, Some("2026-01-01".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();

    // ViewA → BlogPost.content
    db.schema_manager()
        .register_view(identity_view("ViewA", "BlogPost", "content"))
        .await
        .unwrap();

    // ViewB → ViewA.content (chained)
    let view_b = TransformView::new(
        "ViewB",
        SchemaType::Range,
        Some(KeyConfig::new(None, Some("publish_date".to_string()))),
        vec![Query::new("ViewA".to_string(), vec!["content".to_string()])],
        None,
        HashMap::from([("content".to_string(), FieldValueType::Any)]),
    );
    db.schema_manager().register_view(view_b).await.unwrap();

    // Populate caches for both views
    let query_a = Query::new("ViewA".to_string(), vec!["content".to_string()]);
    let query_b = Query::new("ViewB".to_string(), vec!["content".to_string()]);
    db.query_executor().query(query_a).await.unwrap();
    db.query_executor().query(query_b.clone()).await.unwrap();

    // Both should be cached
    assert!(matches!(
        db.db_ops().get_view_cache_state("ViewA").await.unwrap(),
        ViewCacheState::Cached { .. }
    ));
    assert!(matches!(
        db.db_ops().get_view_cache_state("ViewB").await.unwrap(),
        ViewCacheState::Cached { .. }
    ));

    // Write through ViewA (identity → BlogPost.content)
    let mut mutation_fields = HashMap::new();
    mutation_fields.insert("content".to_string(), json!("v2"));
    db.mutation_manager()
        .write_mutations_batch_async(vec![Mutation::new(
            "ViewA".to_string(),
            mutation_fields,
            KeyValue::new(None, Some("2026-01-02".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();

    // Wait for background precomputation to complete
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // ViewB (downstream) should also see the new data after cascade invalidation
    let results = db.query_executor().query(query_b).await.unwrap();
    let values: Vec<_> = results["content"]
        .values()
        .map(|fv| fv.value.clone())
        .collect();
    assert!(
        values.contains(&json!("v2")),
        "ViewB should return fresh data after cascading write-through invalidation, got {:?}",
        values
    );
}
