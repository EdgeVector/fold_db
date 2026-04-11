use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::operations::MutationType;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use serde_json::json;
use std::collections::HashMap;

async fn setup_db() -> FoldDB {
    let dir = tempfile::tempdir().unwrap();
    FoldDB::new(dir.path().to_str().unwrap()).await.unwrap()
}

fn typed_schema_json() -> &'static str {
    r#"{
        "name": "Person",
        "key": { "range_field": "created_at" },
        "fields": {
            "name": {},
            "age": {},
            "email": {},
            "tags": {},
            "created_at": {}
        },
        "field_types": {
            "name": "String",
            "age": "Integer",
            "email": "String",
            "tags": { "Array": "String" },
            "created_at": "String"
        }
    }"#
}

fn untyped_schema_json() -> &'static str {
    r#"{
        "name": "Freeform",
        "key": { "range_field": "id" },
        "fields": {
            "data": {},
            "id": {}
        }
    }"#
}

#[tokio::test]
async fn typed_schema_accepts_valid_mutation() {
    let db = setup_db().await;
    db.load_schema_from_json(typed_schema_json()).await.unwrap();
    db.schema_manager
        .set_schema_state("Person", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("Alice"));
    fields.insert("age".to_string(), json!(30));
    fields.insert("email".to_string(), json!("alice@example.com"));
    fields.insert("tags".to_string(), json!(["admin", "user"]));
    fields.insert("created_at".to_string(), json!("2026-01-01"));

    let mutation = Mutation::new(
        "Person".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-01".to_string())),
        "pk".to_string(),
        MutationType::Create,
    );
    let result = db
        .mutation_manager
        .write_mutations_batch_async(vec![mutation])
        .await;
    assert!(result.is_ok(), "Valid typed mutation should succeed");
}

#[tokio::test]
async fn typed_schema_rejects_wrong_type() {
    let db = setup_db().await;
    db.load_schema_from_json(typed_schema_json()).await.unwrap();
    db.schema_manager
        .set_schema_state("Person", SchemaState::Approved)
        .await
        .unwrap();

    // age should be Integer but we provide String
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("Bob"));
    fields.insert("age".to_string(), json!("thirty")); // WRONG: String instead of Integer
    fields.insert("created_at".to_string(), json!("2026-01-01"));

    let mutation = Mutation::new(
        "Person".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-01".to_string())),
        "pk".to_string(),
        MutationType::Create,
    );
    let result = db
        .mutation_manager
        .write_mutations_batch_async(vec![mutation])
        .await;
    assert!(result.is_err(), "Wrong type should be rejected");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("age"),
        "Error should mention the field name: {}",
        err
    );
    assert!(
        err.contains("Integer"),
        "Error should mention expected type: {}",
        err
    );
}

#[tokio::test]
async fn typed_schema_rejects_wrong_array_element_type() {
    let db = setup_db().await;
    db.load_schema_from_json(typed_schema_json()).await.unwrap();
    db.schema_manager
        .set_schema_state("Person", SchemaState::Approved)
        .await
        .unwrap();

    // tags should be Array<String> but we provide Array<Number>
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("Carol"));
    fields.insert("age".to_string(), json!(25));
    fields.insert("tags".to_string(), json!([1, 2, 3])); // WRONG: numbers instead of strings
    fields.insert("created_at".to_string(), json!("2026-01-01"));

    let mutation = Mutation::new(
        "Person".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-01".to_string())),
        "pk".to_string(),
        MutationType::Create,
    );
    let result = db
        .mutation_manager
        .write_mutations_batch_async(vec![mutation])
        .await;
    assert!(
        result.is_err(),
        "Wrong array element type should be rejected"
    );
    let err = result.unwrap_err().to_string();
    assert!(err.contains("tags"), "Error should mention field: {}", err);
}

#[tokio::test]
async fn untyped_schema_accepts_anything() {
    let db = setup_db().await;
    db.load_schema_from_json(untyped_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("Freeform", SchemaState::Approved)
        .await
        .unwrap();

    // No field_types declared — should accept any value
    let mut fields = HashMap::new();
    fields.insert("data".to_string(), json!({"nested": [1, "two", true]}));
    fields.insert("id".to_string(), json!("abc"));

    let mutation = Mutation::new(
        "Freeform".to_string(),
        fields,
        KeyValue::new(None, Some("abc".to_string())),
        "pk".to_string(),
        MutationType::Create,
    );
    let result = db
        .mutation_manager
        .write_mutations_batch_async(vec![mutation])
        .await;
    assert!(
        result.is_ok(),
        "Untyped schema should accept anything: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn schema_ref_type_enforced() {
    let db = setup_db().await;

    // Create a child schema
    db.load_schema_from_json(
        r#"{
            "name": "Post",
            "key": { "range_field": "id" },
            "fields": { "title": {}, "id": {} }
        }"#,
    )
    .await
    .unwrap();
    db.schema_manager
        .set_schema_state("Post", SchemaState::Approved)
        .await
        .unwrap();

    // Create a parent schema with a typed ref field
    db.load_schema_from_json(
        r#"{
            "name": "User",
            "key": { "range_field": "id" },
            "fields": { "name": {}, "posts": {}, "id": {} },
            "field_types": {
                "name": "String",
                "posts": { "SchemaRef": "Post" },
                "id": "String"
            },
            "ref_fields": { "posts": "Post" }
        }"#,
    )
    .await
    .unwrap();
    db.schema_manager
        .set_schema_state("User", SchemaState::Approved)
        .await
        .unwrap();

    // Valid ref — points to correct schema
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("Alice"));
    fields.insert(
        "posts".to_string(),
        json!([{"schema": "Post", "key": {"range": "p1"}}]),
    );
    fields.insert("id".to_string(), json!("u1"));

    let result = db
        .mutation_manager
        .write_mutations_batch_async(vec![Mutation::new(
            "User".to_string(),
            fields,
            KeyValue::new(None, Some("u1".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await;
    assert!(
        result.is_ok(),
        "Valid schema ref should succeed: {:?}",
        result.err()
    );

    // Invalid ref — points to wrong schema
    let mut bad_fields = HashMap::new();
    bad_fields.insert("name".to_string(), json!("Bob"));
    bad_fields.insert(
        "posts".to_string(),
        json!([{"schema": "WrongSchema", "key": {"range": "p1"}}]),
    );
    bad_fields.insert("id".to_string(), json!("u2"));

    let result = db
        .mutation_manager
        .write_mutations_batch_async(vec![Mutation::new(
            "User".to_string(),
            bad_fields,
            KeyValue::new(None, Some("u2".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await;
    assert!(result.is_err(), "Wrong schema ref should be rejected");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Post"),
        "Error should mention expected schema: {}",
        err
    );
}
