use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::key_config::KeyConfig;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::view::types::{TransformView, ViewCacheState};
use serde_json::json;
use std::collections::HashMap;

async fn setup_db() -> FoldDB {
    let dir = tempfile::tempdir().unwrap();
    FoldDB::new(dir.path().to_str().unwrap()).await.unwrap()
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

#[tokio::test]
async fn mutating_source_invalidates_view_cache() {
    let mut db = setup_db().await;

    // Setup: schema + data + view
    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!("original"));
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

    db.schema_manager
        .register_view(identity_view("CV", "BlogPost", "content"))
        .await
        .unwrap();

    // First query: populates cache
    let query = Query::new("CV".to_string(), vec!["content".to_string()]);
    let results = db.query_executor.query(query.clone()).await.unwrap();
    let first_value = results["content"].values().next().unwrap().value.clone();
    assert_eq!(first_value, json!("original"));

    // Verify cache state is Cached
    let state = db
        .db_ops
        .get_view_cache_state("CV")
        .await
        .unwrap();
    assert!(
        matches!(state, ViewCacheState::Cached { .. }),
        "View should be cached after first query"
    );

    // Mutate the source
    let mut fields2 = HashMap::new();
    fields2.insert("content".to_string(), json!("updated"));
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

    // Verify cache state was invalidated to Empty
    let state_after = db
        .db_ops
        .get_view_cache_state("CV")
        .await
        .unwrap();
    assert!(
        matches!(state_after, ViewCacheState::Empty),
        "View cache should be invalidated after source mutation, got {:?}",
        state_after
    );

    // Re-query: should fetch fresh data
    let results2 = db.query_executor.query(query).await.unwrap();
    let all_values: Vec<_> = results2["content"].values().map(|fv| fv.value.clone()).collect();
    assert!(
        all_values.contains(&json!("updated")),
        "Re-query should contain updated value, got {:?}",
        all_values
    );
}

#[tokio::test]
async fn re_query_after_invalidation_re_caches() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
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

    db.schema_manager
        .register_view(identity_view("TV", "BlogPost", "title"))
        .await
        .unwrap();

    // First query: caches
    let query = Query::new("TV".to_string(), vec!["title".to_string()]);
    db.query_executor.query(query.clone()).await.unwrap();

    // Invalidate
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

    assert!(matches!(
        db.db_ops.get_view_cache_state("TV").await.unwrap(),
        ViewCacheState::Empty
    ));

    // Re-query: should re-cache
    db.query_executor.query(query).await.unwrap();

    assert!(matches!(
        db.db_ops.get_view_cache_state("TV").await.unwrap(),
        ViewCacheState::Cached { .. }
    ));
}

#[tokio::test]
async fn cascading_invalidation_through_view_chain() {
    let mut db = setup_db().await;

    // Setup: schema + data
    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!("original"));
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

    // ViewA reads from BlogPost.content
    db.schema_manager
        .register_view(identity_view("ViewA", "BlogPost", "content"))
        .await
        .unwrap();

    // ViewB reads from ViewA.content (view chain)
    let view_b = TransformView::new(
        "ViewB",
        SchemaType::Range,
        Some(KeyConfig::new(None, Some("publish_date".to_string()))),
        vec![Query::new("ViewA".to_string(), vec!["content".to_string()])],
        None,
        HashMap::from([("content".to_string(), FieldValueType::Any)]),
    );
    db.schema_manager.register_view(view_b).await.unwrap();

    // Query both views to populate caches
    let query_a = Query::new("ViewA".to_string(), vec!["content".to_string()]);
    let query_b = Query::new("ViewB".to_string(), vec!["content".to_string()]);
    db.query_executor.query(query_a).await.unwrap();
    db.query_executor.query(query_b).await.unwrap();

    // Both should be cached
    assert!(matches!(
        db.db_ops.get_view_cache_state("ViewA").await.unwrap(),
        ViewCacheState::Cached { .. }
    ));
    assert!(matches!(
        db.db_ops.get_view_cache_state("ViewB").await.unwrap(),
        ViewCacheState::Cached { .. }
    ));

    // Mutate the source schema
    let mut fields2 = HashMap::new();
    fields2.insert("content".to_string(), json!("updated"));
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

    // ViewA should be invalidated (direct dependency)
    assert!(
        matches!(
            db.db_ops.get_view_cache_state("ViewA").await.unwrap(),
            ViewCacheState::Empty
        ),
        "ViewA cache should be invalidated"
    );

    // ViewB should ALSO be invalidated (cascade: ViewB depends on ViewA)
    assert!(
        matches!(
            db.db_ops.get_view_cache_state("ViewB").await.unwrap(),
            ViewCacheState::Empty
        ),
        "ViewB cache should be invalidated via cascade"
    );
}

#[tokio::test]
async fn view_chain_query_returns_source_data() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!("Chain Test"));
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

    // ViewA reads from BlogPost.title
    db.schema_manager
        .register_view(identity_view("ViewA", "BlogPost", "title"))
        .await
        .unwrap();

    // ViewB reads from ViewA.title
    let view_b = TransformView::new(
        "ViewB",
        SchemaType::Range,
        Some(KeyConfig::new(None, Some("publish_date".to_string()))),
        vec![Query::new("ViewA".to_string(), vec!["title".to_string()])],
        None,
        HashMap::from([("title".to_string(), FieldValueType::Any)]),
    );
    db.schema_manager.register_view(view_b).await.unwrap();

    // Query ViewB — should recursively resolve through ViewA to BlogPost
    let query = Query::new("ViewB".to_string(), vec!["title".to_string()]);
    let results = db.query_executor.query(query).await.unwrap();

    assert!(results.contains_key("title"));
    let values: Vec<_> = results["title"].values().map(|fv| fv.value.clone()).collect();
    assert!(
        values.contains(&json!("Chain Test")),
        "ViewB should return BlogPost data through ViewA chain, got {:?}",
        values
    );
}

#[tokio::test]
async fn three_level_chain_resolves_to_source() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!("deep chain"));
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

    // ViewA → BlogPost
    db.schema_manager
        .register_view(identity_view("ViewA", "BlogPost", "content"))
        .await
        .unwrap();

    // ViewB → ViewA
    db.schema_manager
        .register_view(identity_view("ViewB", "ViewA", "content"))
        .await
        .unwrap();

    // ViewC → ViewB
    db.schema_manager
        .register_view(identity_view("ViewC", "ViewB", "content"))
        .await
        .unwrap();

    // Query ViewC — resolves through ViewB → ViewA → BlogPost
    let query = Query::new("ViewC".to_string(), vec!["content".to_string()]);
    let results = db.query_executor.query(query).await.unwrap();

    let values: Vec<_> = results["content"]
        .values()
        .map(|fv| fv.value.clone())
        .collect();
    assert!(
        values.contains(&json!("deep chain")),
        "ViewC should resolve through 3-level chain, got {:?}",
        values
    );
}

#[tokio::test]
async fn chain_re_query_after_cascade_invalidation_gets_fresh_data() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!("v1"));
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

    // ViewA → BlogPost, ViewB → ViewA
    db.schema_manager
        .register_view(identity_view("ViewA", "BlogPost", "content"))
        .await
        .unwrap();
    db.schema_manager
        .register_view(identity_view("ViewB", "ViewA", "content"))
        .await
        .unwrap();

    // Populate caches
    let query_b = Query::new("ViewB".to_string(), vec!["content".to_string()]);
    let results = db.query_executor.query(query_b.clone()).await.unwrap();
    let val = results["content"].values().next().unwrap().value.clone();
    assert_eq!(val, json!("v1"));

    // Mutate source — both caches cascade-invalidated
    let mut fields2 = HashMap::new();
    fields2.insert("content".to_string(), json!("v2"));
    fields2.insert("publish_date".to_string(), json!("2026-01-01"));
    db.mutation_manager
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            fields2,
            KeyValue::new(None, Some("2026-01-01".to_string())),
            "pk".to_string(),
            MutationType::Update,
        )])
        .await
        .unwrap();

    // Re-query ViewB — should get fresh "v2" through the entire chain
    let results2 = db.query_executor.query(query_b).await.unwrap();
    let values: Vec<_> = results2["content"]
        .values()
        .map(|fv| fv.value.clone())
        .collect();
    assert!(
        values.contains(&json!("v2")),
        "ViewB should return fresh data after cascade invalidation, got {:?}",
        values
    );
}

#[tokio::test]
async fn multi_source_view_from_two_views() {
    let mut db = setup_db().await;

    // Create two source schemas
    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    db.load_schema_from_json(
        r#"{
            "name": "Author",
            "key": { "range_field": "publish_date" },
            "fields": { "name": {}, "publish_date": {} }
        }"#,
    )
    .await
    .unwrap();
    db.schema_manager
        .set_schema_state("Author", SchemaState::Approved)
        .await
        .unwrap();

    // Write data
    let mut bp_fields = HashMap::new();
    bp_fields.insert("title".to_string(), json!("Hello"));
    bp_fields.insert("publish_date".to_string(), json!("2026-01-01"));
    db.mutation_manager
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            bp_fields,
            KeyValue::new(None, Some("2026-01-01".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();

    let mut author_fields = HashMap::new();
    author_fields.insert("name".to_string(), json!("Tom"));
    author_fields.insert("publish_date".to_string(), json!("2026-01-01"));
    db.mutation_manager
        .write_mutations_batch_async(vec![Mutation::new(
            "Author".to_string(),
            author_fields,
            KeyValue::new(None, Some("2026-01-01".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();

    // ViewA → BlogPost.title, ViewB → Author.name
    db.schema_manager
        .register_view(identity_view("ViewA", "BlogPost", "title"))
        .await
        .unwrap();

    let view_b = TransformView::new(
        "ViewB",
        SchemaType::Range,
        Some(KeyConfig::new(None, Some("publish_date".to_string()))),
        vec![Query::new("Author".to_string(), vec!["name".to_string()])],
        None,
        HashMap::from([("name".to_string(), FieldValueType::Any)]),
    );
    db.schema_manager.register_view(view_b).await.unwrap();

    // ViewC reads from both ViewA and ViewB
    let view_c = TransformView::new(
        "ViewC",
        SchemaType::Range,
        Some(KeyConfig::new(None, Some("publish_date".to_string()))),
        vec![
            Query::new("ViewA".to_string(), vec!["title".to_string()]),
            Query::new("ViewB".to_string(), vec!["name".to_string()]),
        ],
        None,
        HashMap::from([
            ("title".to_string(), FieldValueType::Any),
            ("name".to_string(), FieldValueType::Any),
        ]),
    );
    db.schema_manager.register_view(view_c).await.unwrap();

    // Query ViewC — should merge data from both source views
    let query = Query::new("ViewC".to_string(), vec![]);
    let results = db.query_executor.query(query).await.unwrap();

    assert!(
        results.contains_key("title"),
        "ViewC should have 'title' from ViewA"
    );
    assert!(
        results.contains_key("name"),
        "ViewC should have 'name' from ViewB"
    );

    let title = results["title"].values().next().unwrap().value.clone();
    let name = results["name"].values().next().unwrap().value.clone();
    assert_eq!(title, json!("Hello"));
    assert_eq!(name, json!("Tom"));
}

#[tokio::test]
async fn three_level_cascade_invalidation() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!("original"));
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

    // ViewA → BlogPost, ViewB → ViewA, ViewC → ViewB
    db.schema_manager
        .register_view(identity_view("ViewA", "BlogPost", "content"))
        .await
        .unwrap();
    db.schema_manager
        .register_view(identity_view("ViewB", "ViewA", "content"))
        .await
        .unwrap();
    db.schema_manager
        .register_view(identity_view("ViewC", "ViewB", "content"))
        .await
        .unwrap();

    // Populate all caches
    for name in &["ViewA", "ViewB", "ViewC"] {
        let q = Query::new(name.to_string(), vec!["content".to_string()]);
        db.query_executor.query(q).await.unwrap();
    }

    // All should be cached
    for name in &["ViewA", "ViewB", "ViewC"] {
        assert!(
            matches!(
                db.db_ops.get_view_cache_state(name).await.unwrap(),
                ViewCacheState::Cached { .. }
            ),
            "{} should be cached",
            name
        );
    }

    // Mutate source
    let mut fields2 = HashMap::new();
    fields2.insert("content".to_string(), json!("changed"));
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

    // All three views should be invalidated via cascade
    for name in &["ViewA", "ViewB", "ViewC"] {
        assert!(
            matches!(
                db.db_ops.get_view_cache_state(name).await.unwrap(),
                ViewCacheState::Empty
            ),
            "{} should be invalidated via cascade",
            name
        );
    }
}
