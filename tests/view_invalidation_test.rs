use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field::FieldValue;
use fold_db::schema::types::key_config::KeyConfig;
use fold_db::schema::types::operations::MutationType;
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::types::{KeyValue, Mutation, Query};
use fold_db::schema::SchemaState;
use fold_db::view::types::{FieldRef, TransformFieldDef, TransformFieldState, TransformView};
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
    let mut fields = HashMap::new();
    fields.insert(
        "out".into(),
        TransformFieldDef {
            source: FieldRef::new(source_schema, source_field),
            wasm_forward: None,
            wasm_inverse: None,
        },
    );
    TransformView::new(name, SchemaType::Range, Some(KeyConfig::new(None, Some("publish_date".to_string()))), fields)
}

#[tokio::test]
async fn mutating_source_invalidates_cached_view_field() {
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
    let query = Query::new("CV".to_string(), vec!["out".to_string()]);
    let results = db.query_executor.query(query.clone()).await.unwrap();
    let first_value = results["out"].values().next().unwrap().value.clone();
    assert_eq!(first_value, json!("original"));

    // Verify field state is Cached
    let state = db
        .db_ops
        .get_transform_field_state("CV", "out")
        .await
        .unwrap();
    assert!(
        matches!(state, TransformFieldState::Cached { .. }),
        "Field should be cached after first query"
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

    // Verify field state was invalidated to Empty
    let state_after = db
        .db_ops
        .get_transform_field_state("CV", "out")
        .await
        .unwrap();
    assert!(
        matches!(state_after, TransformFieldState::Empty),
        "Field should be invalidated after source mutation, got {:?}",
        state_after
    );

    // Re-query: should fetch fresh data (both original and updated are present since they have different range keys)
    let results2 = db.query_executor.query(query).await.unwrap();
    let all_values: Vec<_> = results2["out"].values().map(|fv| fv.value.clone()).collect();
    assert!(
        all_values.contains(&json!("updated")),
        "Re-query should contain updated value, got {:?}",
        all_values
    );
}

#[tokio::test]
async fn overridden_field_survives_source_mutation() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("content".to_string(), json!("source_val"));
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
        .register_view(identity_view("OV", "BlogPost", "content"))
        .await
        .unwrap();

    // Manually set the field state to Overridden (simulating a direct write to the view)
    let override_entries = vec![(
        KeyValue::new(None, Some("override".to_string())),
        FieldValue {
            value: json!("custom_override"),
            atom_uuid: String::new(),
            source_file_name: None,
            metadata: None,
            molecule_uuid: None,
            molecule_version: None,
        },
    )];
    db.db_ops
        .set_transform_field_state(
            "OV",
            "out",
            &TransformFieldState::Overridden {
                entries: override_entries,
            },
        )
        .await
        .unwrap();

    // Mutate the source
    let mut fields2 = HashMap::new();
    fields2.insert("content".to_string(), json!("new_source_val"));
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

    // Overridden field should NOT be invalidated
    let state = db
        .db_ops
        .get_transform_field_state("OV", "out")
        .await
        .unwrap();
    assert!(
        matches!(state, TransformFieldState::Overridden { .. }),
        "Overridden field should survive source mutation, got {:?}",
        state
    );

    // Query should return the overridden value, not the source
    let query = Query::new("OV".to_string(), vec!["out".to_string()]);
    let results = db.query_executor.query(query).await.unwrap();
    let value = results["out"].values().next().unwrap().value.clone();
    assert_eq!(value, json!("custom_override"));
}
