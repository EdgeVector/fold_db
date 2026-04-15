//! Regression test for schema lookup by descriptive_name.
//!
//! `get_schema_by_name` must accept both:
//!   - the content-addressed identity_hash (the stable primary key), and
//!   - the human-readable descriptive_name (resolved via the
//!     descriptive_name_index that's populated at load time without
//!     needing the embedding model).
//!
//! fold_db_node always fetches Phase 1 built-in schemas by descriptive
//! name at startup, so this fallback path is load-bearing.

use std::collections::HashMap;

use fold_db::schema::types::data_classification::DataClassification;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::schema::DeclarativeSchemaType;
use fold_db::schema::types::Schema;
use fold_db::schema_service::state::SchemaServiceState;
use fold_db::schema_service::types::SchemaAddOutcome;
use tempfile::tempdir;

fn make_state() -> SchemaServiceState {
    let temp_dir = tempdir().expect("failed to create temp directory");
    let db_path = temp_dir
        .path()
        .join("test_lookup_db")
        .to_string_lossy()
        .to_string();
    std::mem::forget(temp_dir);
    SchemaServiceState::new(db_path).expect("failed to create state")
}

async fn add_schema(state: &SchemaServiceState, descriptive_name: &str) -> String {
    let mut schema = Schema::new(
        descriptive_name.to_string(),
        DeclarativeSchemaType::Single,
        None,
        Some(vec!["id".to_string(), "title".to_string()]),
        None,
        None,
    );
    schema.descriptive_name = Some(descriptive_name.to_string());
    for f in ["id", "title"] {
        schema
            .field_descriptions
            .insert(f.to_string(), format!("{} field", f));
        schema
            .field_types
            .insert(f.to_string(), FieldValueType::String);
        schema
            .field_classifications
            .insert(f.to_string(), vec!["word".to_string()]);
        schema
            .field_data_classifications
            .insert(f.to_string(), DataClassification::low());
    }
    match state
        .add_schema(schema, HashMap::new())
        .await
        .expect("failed to add schema")
    {
        SchemaAddOutcome::Added(s, _)
        | SchemaAddOutcome::AlreadyExists(s, _)
        | SchemaAddOutcome::Expanded(_, s, _) => s.name,
    }
}

#[tokio::test]
async fn lookup_by_identity_hash_returns_schema() {
    let state = make_state();
    let hash = add_schema(&state, "Test Schema").await;

    let fetched = state
        .get_schema_by_name(&hash)
        .expect("get_schema_by_name failed")
        .expect("schema should be found by identity hash");
    assert_eq!(fetched.descriptive_name.as_deref(), Some("Test Schema"));
}

#[tokio::test]
async fn lookup_by_descriptive_name_falls_through_to_index() {
    let state = make_state();
    let hash = add_schema(&state, "Test Schema").await;

    // This is the path fold_db_node uses on boot — descriptive name
    // → identity_hash via descriptive_name_index. Must work even when
    // the embedding model is unavailable (e.g. fastembed can't reach
    // HuggingFace from inside a Lambda), because the index is
    // populated without running the embedder.
    let fetched = state
        .get_schema_by_name("Test Schema")
        .expect("get_schema_by_name failed")
        .expect("schema should be found by descriptive name");
    assert_eq!(fetched.name, hash);
    assert_eq!(fetched.descriptive_name.as_deref(), Some("Test Schema"));
}

#[tokio::test]
async fn lookup_by_unknown_name_returns_none() {
    let state = make_state();
    let _ = add_schema(&state, "Real Schema").await;

    let fetched = state
        .get_schema_by_name("NonexistentSchema")
        .expect("get_schema_by_name failed");
    assert!(fetched.is_none());
}
