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
    let results = db.query_executor().query_with_access(query, &fold_db::access::AccessContext::owner("test"), None).await.unwrap();

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

/// WASM views do not support write-back — source_field_map returns None,
/// and attempting a mutation is rejected.
#[tokio::test]
async fn wasm_view_write_is_rejected() {
    let db = setup_db().await;

    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    // Register a WASM view (not identity)
    let view = TransformView::new(
        "WasmView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["content".to_string()],
        )],
        Some(vec![0, 1, 2]), // Placeholder WASM — won't be executed
        HashMap::from([("out".to_string(), FieldValueType::String)]),
    );
    db.schema_manager().register_view(view).await.unwrap();

    // WASM view should not expose a source_field_map (no inverse)
    let stored = db.schema_manager().get_view("WasmView").unwrap().unwrap();
    assert!(!stored.is_identity());
    assert!(
        stored.source_field_map().is_none(),
        "WASM view should not have a source_field_map"
    );

    // Try to mutate the WASM view — should be rejected
    let mut mutation_fields = HashMap::new();
    mutation_fields.insert("out".to_string(), json!("should fail"));
    let mutation = Mutation::new(
        "WasmView".to_string(),
        mutation_fields,
        KeyValue::new(None, Some("2026-01-01".to_string())),
        "pk".to_string(),
        MutationType::Create,
    );
    let result = db
        .mutation_manager()
        .write_mutations_batch_async(vec![mutation])
        .await;
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("WASM view"),
        "Should mention WASM view write-back not supported"
    );
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
    db.query_executor().query_with_access(query.clone(), &fold_db::access::AccessContext::owner("test"), None).await.unwrap();

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
    let results = db.query_executor().query_with_access(query, &fold_db::access::AccessContext::owner("test"), None).await.unwrap();
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
    db.query_executor().query_with_access(query_a, &fold_db::access::AccessContext::owner("test"), None).await.unwrap();
    db.query_executor().query_with_access(query_b.clone(), &fold_db::access::AccessContext::owner("test"), None).await.unwrap();

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
    let results = db.query_executor().query_with_access(query_b, &fold_db::access::AccessContext::owner("test"), None).await.unwrap();
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
