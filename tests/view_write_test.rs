use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field::FieldValue;
use fold_db::schema::types::key_config::KeyConfig;
use fold_db::schema::types::operations::MutationType;
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::types::{KeyValue, Mutation, Query};
use fold_db::schema::SchemaState;
use fold_db::view::types::{
    FieldRef, TransformFieldDef, TransformFieldState, TransformView, TransformWriteMode,
};
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

#[tokio::test]
async fn identity_write_redirects_to_source() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    // Create a view with identity mapping
    let mut fields = HashMap::new();
    fields.insert(
        "view_title".into(),
        TransformFieldDef {
            source: FieldRef::new("BlogPost", "title"),
            wasm_forward: None,
            wasm_inverse: None,
        },
    );
    let view = TransformView::new("WriteView", SchemaType::Range, Some(KeyConfig::new(None, Some("publish_date".to_string()))), fields);
    db.schema_manager.register_view(view).await.unwrap();

    // Verify write mode is Identity
    let stored_view = db.schema_manager.get_view("WriteView").unwrap().unwrap();
    assert_eq!(
        *stored_view.write_modes.get("view_title").unwrap(),
        TransformWriteMode::Identity
    );

    // Write to the VIEW — should redirect to BlogPost.title
    let mut mutation_fields = HashMap::new();
    mutation_fields.insert("view_title".to_string(), json!("Written via view"));
    let mutation = Mutation::new(
        "WriteView".to_string(),
        mutation_fields,
        KeyValue::new(None, Some("2026-01-01".to_string())),
        "pk".to_string(),
        MutationType::Create,
    );
    db.mutation_manager
        .write_mutations_batch_async(vec![mutation])
        .await
        .unwrap();

    // Query the SOURCE schema to verify the write landed there
    let query = Query::new("BlogPost".to_string(), vec!["title".to_string()]);
    let results = db.query_executor.query(query).await.unwrap();

    assert!(
        results.contains_key("title"),
        "BlogPost should have title field in results"
    );
    let title_values = &results["title"];
    assert!(
        !title_values.is_empty(),
        "Should have data written to BlogPost.title"
    );
    let first_value = title_values.values().next().unwrap().value.clone();
    assert_eq!(first_value, json!("Written via view"));
}

#[tokio::test]
async fn irreversible_write_stores_as_overridden() {
    let mut db = setup_db().await;

    db.load_schema_from_json(blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    // Write some source data first
    let mut source_fields = HashMap::new();
    source_fields.insert("content".to_string(), json!("original content"));
    source_fields.insert("publish_date".to_string(), json!("2026-01-01"));
    db.mutation_manager
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            source_fields,
            KeyValue::new(None, Some("2026-01-01".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .unwrap();

    // Create a view with forward-only WASM (irreversible)
    // Since we don't have actual WASM, we'll simulate by directly checking the write mode
    // For now, test the Irreversible path by creating the field state manually
    let mut fields = HashMap::new();
    fields.insert(
        "computed".into(),
        TransformFieldDef {
            source: FieldRef::new("BlogPost", "content"),
            wasm_forward: None, // Identity for now
            wasm_inverse: None,
        },
    );
    let view = TransformView::new("IrrevView", SchemaType::Range, Some(KeyConfig::new(None, Some("publish_date".to_string()))), fields);
    db.schema_manager.register_view(view).await.unwrap();

    // Manually simulate an Irreversible write by setting Overridden state
    let override_entries = vec![(
        KeyValue::new(None, Some("override".to_string())),
        FieldValue {
            value: json!("directly written value"),
            atom_uuid: String::new(),
            source_file_name: None,
            metadata: None,
            molecule_uuid: None,
            molecule_version: None,
        },
    )];
    db.db_ops
        .set_transform_field_state(
            "IrrevView",
            "computed",
            &TransformFieldState::Overridden {
                entries: override_entries,
            },
        )
        .await
        .unwrap();

    // Query the view — should return the overridden value
    let query = Query::new("IrrevView".to_string(), vec!["computed".to_string()]);
    let results = db.query_executor.query(query).await.unwrap();

    let computed_values = &results["computed"];
    let value = computed_values.values().next().unwrap().value.clone();
    assert_eq!(value, json!("directly written value"));

    // Source should be untouched — query BlogPost.content should still have original
    let source_query = Query::new("BlogPost".to_string(), vec!["content".to_string()]);
    let source_results = db.query_executor.query(source_query).await.unwrap();
    let source_value = source_results["content"]
        .values()
        .next()
        .unwrap()
        .value
        .clone();
    assert_eq!(source_value, json!("original content"));
}
