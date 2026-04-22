//! Round-trip coverage for the TriggerFiring schema registered at
//! FoldDB startup. Exercises the actual boot path — no direct calls
//! into `register_trigger_firing_schema`.

use fold_db::fold_db_core::fold_db::FoldDB;
use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::SchemaState;
use fold_db::triggers::{fields, TRIGGER_FIRING_SCHEMA_NAME};

async fn boot_fresh_db() -> (tempfile::TempDir, FoldDB) {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let path = temp_dir.path().to_str().expect("utf8 path").to_string();
    let db = FoldDB::new(&path).await.expect("FoldDB::new");
    (temp_dir, db)
}

#[tokio::test]
async fn trigger_firing_schema_is_registered_at_startup() {
    let (_tmp, db) = boot_fresh_db().await;

    let schema = db
        .schema_manager()
        .get_schema(TRIGGER_FIRING_SCHEMA_NAME)
        .await
        .expect("get_schema")
        .expect("TriggerFiring schema should be present after FoldDB::new");

    assert_eq!(schema.name, TRIGGER_FIRING_SCHEMA_NAME);
    assert_eq!(schema.schema_type, SchemaType::HashRange);

    let key = schema.key.as_ref().expect("HashRange key");
    assert_eq!(key.hash_field.as_deref(), Some(fields::TRIGGER_ID));
    assert_eq!(key.range_field.as_deref(), Some(fields::FIRED_AT));

    let mut declared: Vec<&str> = schema
        .fields
        .as_ref()
        .expect("fields present")
        .iter()
        .map(String::as_str)
        .collect();
    declared.sort();
    let mut expected = vec![
        fields::TRIGGER_ID,
        fields::VIEW_NAME,
        fields::FIRED_AT,
        fields::DURATION_MS,
        fields::STATUS,
        fields::INPUT_ROW_COUNT,
        fields::OUTPUT_ROW_COUNT,
        fields::ERROR_MESSAGE,
    ];
    expected.sort();
    assert_eq!(declared, expected);

    for numeric in [
        fields::FIRED_AT,
        fields::DURATION_MS,
        fields::INPUT_ROW_COUNT,
        fields::OUTPUT_ROW_COUNT,
    ] {
        assert_eq!(
            schema.field_types.get(numeric),
            Some(&FieldValueType::Integer),
            "{numeric} should be Integer"
        );
    }
    for string_field in [fields::TRIGGER_ID, fields::VIEW_NAME, fields::STATUS] {
        assert_eq!(
            schema.field_types.get(string_field),
            Some(&FieldValueType::String),
            "{string_field} should be String"
        );
    }
    match schema.field_types.get(fields::ERROR_MESSAGE) {
        Some(FieldValueType::OneOf(variants)) => {
            assert!(variants.contains(&FieldValueType::String));
            assert!(variants.contains(&FieldValueType::Null));
        }
        other => panic!("expected OneOf(String, Null) for error_message, got {other:?}"),
    }
}

#[tokio::test]
async fn trigger_firing_schema_is_approved_at_startup() {
    let (_tmp, db) = boot_fresh_db().await;

    let states = db.schema_manager().get_schema_states().expect("states");
    assert_eq!(
        states.get(TRIGGER_FIRING_SCHEMA_NAME),
        Some(&SchemaState::Approved),
        "TriggerFiring must be Approved so the runner can write without a manual approval step",
    );
}

#[tokio::test]
async fn registration_is_idempotent_on_repeated_calls() {
    let (_tmp, db) = boot_fresh_db().await;

    // The first registration ran inside FoldDB::new. Re-running it
    // against the same SchemaCore is what the idempotency contract
    // guards — it must not error and must not clobber state.
    let schema_manager = db.schema_manager();
    fold_db::triggers::register_trigger_firing_schema(&schema_manager)
        .await
        .expect("second registration");
    fold_db::triggers::register_trigger_firing_schema(&schema_manager)
        .await
        .expect("third registration");

    let schema = schema_manager
        .get_schema(TRIGGER_FIRING_SCHEMA_NAME)
        .await
        .expect("get_schema")
        .expect("schema should still be present");
    assert_eq!(schema.name, TRIGGER_FIRING_SCHEMA_NAME);
    assert_eq!(
        schema_manager
            .get_schema_states()
            .expect("states")
            .get(TRIGGER_FIRING_SCHEMA_NAME),
        Some(&SchemaState::Approved),
    );
}
