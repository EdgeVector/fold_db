//! End-to-end integration: boot a real FoldDB, register a view with an
//! `OnWrite` trigger, mutate the source schema, and assert that a
//! corresponding row landed in the internal `TriggerFiring` schema.
//!
//! Guards against regressing the entire dispatch chain at once — any
//! break in MutationManager → TriggerDispatcher →
//! ViewOrchestratorFireHandler → MutationManagerFiringWriter lands here.

use std::collections::HashMap;

use fold_db::fold_db_core::fold_db::FoldDB;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::key_config::KeyConfig;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::test_helpers::TestSchemaBuilder;
use fold_db::triggers::types::Trigger;
use fold_db::triggers::{fields, status, TRIGGER_FIRING_SCHEMA_NAME};
use fold_db::view::types::TransformView;
use serde_json::json;

async fn setup_db() -> (tempfile::TempDir, FoldDB) {
    let dir = tempfile::tempdir().unwrap();
    let db = FoldDB::new(dir.path().to_str().unwrap()).await.unwrap();
    (dir, db)
}

fn blogpost_schema_json() -> String {
    TestSchemaBuilder::new("BlogPost")
        .fields(&["title", "content"])
        .range_key("publish_date")
        .build_json()
}

fn identity_view_with_triggers(
    name: &str,
    source_schema: &str,
    source_field: &str,
    triggers: Vec<Trigger>,
) -> TransformView {
    let mut view = TransformView::new(
        name,
        SchemaType::Range,
        Some(KeyConfig::new(None, Some("publish_date".to_string()))),
        vec![Query::new(
            source_schema.to_string(),
            vec![source_field.to_string()],
        )],
        None,
        HashMap::from([(source_field.to_string(), FieldValueType::Any)]),
    );
    view.triggers = triggers;
    view
}

async fn scan_trigger_firings(db: &FoldDB) -> Vec<HashMap<String, serde_json::Value>> {
    let q = Query::new(
        TRIGGER_FIRING_SCHEMA_NAME.to_string(),
        vec![
            fields::TRIGGER_ID.to_string(),
            fields::VIEW_NAME.to_string(),
            fields::FIRED_AT.to_string(),
            fields::DURATION_MS.to_string(),
            fields::STATUS.to_string(),
            fields::INPUT_ROW_COUNT.to_string(),
            fields::OUTPUT_ROW_COUNT.to_string(),
            fields::ERROR_MESSAGE.to_string(),
        ],
    );
    let results = db
        .query_executor()
        .query(q)
        .await
        .expect("query TriggerFiring");

    // Reshape: HashMap<field, HashMap<KeyValue, FieldValue>> →
    // Vec<HashMap<field, Value>> keyed by (hash, range).
    let mut rows: HashMap<(Option<String>, Option<String>), HashMap<String, serde_json::Value>> =
        HashMap::new();
    for (field_name, entries) in results {
        for (kv, fv) in entries {
            let key = (kv.hash.clone(), kv.range.clone());
            rows.entry(key)
                .or_default()
                .insert(field_name.clone(), fv.value.clone());
        }
    }
    rows.into_values().collect()
}

#[tokio::test]
async fn on_write_trigger_writes_trigger_firing_row() {
    let (_tmp, db) = setup_db().await;

    // Register a normal user schema and mark it approved.
    db.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .unwrap();

    // Register a view with an explicit OnWrite trigger. The runner
    // should see this and fire on every BlogPost mutation.
    db.schema_manager()
        .register_view(identity_view_with_triggers(
            "BP_Content",
            "BlogPost",
            "content",
            vec![Trigger::OnWrite],
        ))
        .await
        .unwrap();

    // Baseline: no firings yet.
    let baseline = scan_trigger_firings(&db).await;
    assert!(
        baseline.is_empty(),
        "expected no TriggerFiring rows before mutation, got {:?}",
        baseline
    );

    // Mutate the source schema.
    let mut fields_map = HashMap::new();
    fields_map.insert("content".to_string(), json!("hello world"));
    fields_map.insert("publish_date".to_string(), json!("2026-04-21"));
    db.mutation_manager()
        .write_mutations_batch_async(vec![Mutation::new(
            "BlogPost".to_string(),
            fields_map,
            KeyValue::new(None, Some("2026-04-21".to_string())),
            "pk".to_string(),
            MutationType::Create,
        )])
        .await
        .expect("mutate BlogPost");

    // TriggerFiring is itself a mutation through MutationManager, so the
    // row is visible by the time write_mutations_batch_async (for the
    // BlogPost mutation) returns — the OnWrite path is inline.
    let rows = scan_trigger_firings(&db).await;
    assert_eq!(
        rows.len(),
        1,
        "expected exactly one TriggerFiring row, got {:?}",
        rows
    );

    let row = &rows[0];
    assert_eq!(
        row.get(fields::VIEW_NAME),
        Some(&json!("BP_Content")),
        "view_name"
    );
    assert_eq!(
        row.get(fields::TRIGGER_ID),
        Some(&json!("BP_Content:0")),
        "trigger_id should be '{{view_name}}:{{index}}'"
    );
    assert_eq!(
        row.get(fields::STATUS),
        Some(&json!(status::SUCCESS)),
        "status"
    );
    assert!(
        matches!(row.get(fields::FIRED_AT), Some(serde_json::Value::Number(n)) if n.as_i64().map(|v| v > 0).unwrap_or(false)),
        "fired_at should be a positive epoch ms"
    );
    assert!(
        matches!(
            row.get(fields::ERROR_MESSAGE),
            Some(serde_json::Value::Null)
        ),
        "successful fire should have null error_message"
    );
}
