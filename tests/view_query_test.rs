use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::key_config::KeyConfig;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::view::types::TransformView;
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
async fn query_identity_view_returns_source_data() {
    let mut db = setup_db().await;

    // Load and approve a schema
    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    // Write data to the schema
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!("Hello World"));
    fields.insert("content".to_string(), json!("Test content"));
    fields.insert("publish_date".to_string(), json!("2026-01-01"));
    let mutation = Mutation::new(
        "BlogPost".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-01".to_string())),
        "test_pub_key".to_string(),
        MutationType::Create,
    );
    db.mutation_manager
        .write_mutations_batch_async(vec![mutation])
        .await
        .unwrap();

    // Register a view over BlogPost.content
    let view = identity_view("ContentView", "BlogPost", "content");
    db.schema_manager.register_view(view).await.unwrap();

    // Query the view — should return the source data
    let query = Query::new("ContentView".to_string(), vec!["content".to_string()]);
    let results = db.query_executor.query(query).await.unwrap();

    assert!(results.contains_key("content"), "View field 'content' should be in results");
    let content_values = &results["content"];
    assert!(!content_values.is_empty(), "Should have at least one value");

    // The value should be the content from BlogPost
    let first_value = content_values.values().next().unwrap();
    assert_eq!(first_value.value, json!("Test content"));
}

#[tokio::test]
async fn query_nonexistent_name_errors() {
    let db = setup_db().await;

    let query = Query::new("DoesNotExist".to_string(), vec![]);
    let result = db.query_executor.query(query).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn query_blocked_view_errors() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();

    let view = identity_view("BlockedView", "BlogPost", "title");
    db.schema_manager.register_view(view).await.unwrap();
    db.schema_manager.block_view("BlockedView").await.unwrap();

    let query = Query::new("BlockedView".to_string(), vec!["title".to_string()]);
    let result = db.query_executor.query(query).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("blocked"));
}

#[tokio::test]
async fn query_view_with_empty_fields_returns_all() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!("Title"));
    fields.insert("content".to_string(), json!("Body"));
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

    // View with two output fields from one input query
    let view = TransformView::new(
        "FullView",
        SchemaType::Range,
        Some(KeyConfig::new(None, Some("publish_date".to_string()))),
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["title".to_string(), "content".to_string()],
        )],
        None,
        HashMap::from([
            ("title".to_string(), FieldValueType::Any),
            ("content".to_string(), FieldValueType::Any),
        ]),
    );
    db.schema_manager.register_view(view).await.unwrap();

    // Query with empty fields — should return all view output fields
    let query = Query::new("FullView".to_string(), vec![]);
    let results = db.query_executor.query(query).await.unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.contains_key("title"));
    assert!(results.contains_key("content"));
}

#[tokio::test]
async fn mutation_targeting_view_is_rejected() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let view = identity_view("MyView", "BlogPost", "content");
    db.schema_manager.register_view(view).await.unwrap();

    // Try to mutate the view directly — should be rejected
    let mut mutation_fields = HashMap::new();
    mutation_fields.insert("content".to_string(), json!("should fail"));
    let mutation = Mutation::new(
        "MyView".to_string(),
        mutation_fields,
        KeyValue::new(None, Some("2026-01-01".to_string())),
        "pk".to_string(),
        MutationType::Create,
    );
    let result = db
        .mutation_manager
        .write_mutations_batch_async(vec![mutation])
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Cannot mutate view"));
}
