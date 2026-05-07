use fold_db::atom::deterministic_molecule_uuid;
use fold_db::schema::types::field::Field;
use fold_db::schema::types::SchemaError;
use fold_db::schema::{SchemaCore, SchemaState};
use fold_db::test_helpers::TestSchemaBuilder;

fn user_schema_json() -> String {
    TestSchemaBuilder::new("User")
        .fields(&["id", "name"])
        .range_key("created_at")
        .build_json()
}

fn user_public_schema_json() -> String {
    TestSchemaBuilder::new("UserPublic")
        .fields(&["id", "display_name", "view_count", "is_featured"])
        .range_key("created_at")
        .field_mapper("id", "User.id")
        .field_mapper("display_name", "User.name")
        .build_json()
}

#[tokio::test]
async fn approving_schema_applies_field_mappers() {
    let core = SchemaCore::new_for_testing().await.expect("init core");

    core.load_schema_from_json(&user_schema_json())
        .await
        .expect("load source schema");
    core.load_schema_from_json(&user_public_schema_json())
        .await
        .expect("load target schema");

    let initial_target_schema = core
        .get_schema_metadata("UserPublic")
        .expect("fetch schema")
        .expect("schema exists");

    // Before approval, the id field has its OWN deterministic molecule UUID
    let pre_approval_id_uuid = initial_target_schema
        .runtime_fields
        .get("id")
        .and_then(|field| field.common().molecule_uuid())
        .cloned()
        .expect("id should have a deterministic molecule uuid");

    // It should be UserPublic's own deterministic UUID, not User's
    assert_eq!(
        pre_approval_id_uuid,
        deterministic_molecule_uuid("UserPublic", "id"),
        "before approval, id should have UserPublic's own deterministic UUID"
    );

    core.set_schema_state("UserPublic", SchemaState::Approved)
        .await
        .expect("approve schema");

    let approved_schema = core
        .get_schema_metadata("UserPublic")
        .expect("fetch schema")
        .expect("schema exists");

    let id_uuid = approved_schema
        .runtime_fields
        .get("id")
        .and_then(|field| field.common().molecule_uuid())
        .cloned()
        .expect("id molecule uuid");
    let display_uuid = approved_schema
        .runtime_fields
        .get("display_name")
        .and_then(|field| field.common().molecule_uuid())
        .cloned()
        .expect("display_name molecule uuid");

    // After approval, mapped fields should point to the SOURCE schema's molecule UUID
    assert_eq!(
        id_uuid,
        deterministic_molecule_uuid("User", "id"),
        "id should map to User.id's molecule"
    );
    assert_eq!(
        display_uuid,
        deterministic_molecule_uuid("User", "name"),
        "display_name should map to User.name's molecule"
    );
}

#[tokio::test]
async fn approving_schema_with_dangling_source_schema_errors() {
    // A FieldMapper that points at a source schema that does not exist
    // must surface as a hard error at approval time. Silently skipping
    // would leave the target field without its molecule UUID and turn
    // every read against the new schema into an empty result.
    let core = SchemaCore::new_for_testing().await.expect("init core");

    // Note: we never load a "User" source schema — the mapper is dangling.
    let target = TestSchemaBuilder::new("UserPublic")
        .fields(&["id", "display_name"])
        .range_key("created_at")
        .field_mapper("id", "User.id")
        .build_json();
    core.load_schema_from_json(&target)
        .await
        .expect("load target schema");

    let err = core
        .set_schema_state("UserPublic", SchemaState::Approved)
        .await
        .expect_err("approval should reject dangling source schema");

    match err {
        SchemaError::InvalidData(msg) => {
            assert!(
                msg.contains("source schema 'User'") && msg.contains("does not exist"),
                "expected dangling-source-schema error, got: {msg}",
            );
        }
        other => panic!("expected SchemaError::InvalidData, got {other:?}"),
    }
}

#[tokio::test]
async fn approving_schema_with_missing_source_field_errors() {
    // FieldMapper's source_field references a field that doesn't exist in
    // the source schema's runtime_fields. A malformed mapper like this
    // means the schema is internally inconsistent — fail loud at approval
    // rather than silently dropping the mapping.
    let core = SchemaCore::new_for_testing().await.expect("init core");

    // Source schema 'User' exists but does NOT have a 'phantom' field.
    let source = TestSchemaBuilder::new("User")
        .fields(&["id", "name"])
        .range_key("created_at")
        .build_json();
    core.load_schema_from_json(&source)
        .await
        .expect("load source");

    // Target schema points at User.phantom — the field is missing.
    let target = TestSchemaBuilder::new("UserPublic")
        .fields(&["display_name"])
        .range_key("created_at")
        .field_mapper("display_name", "User.phantom")
        .build_json();
    core.load_schema_from_json(&target)
        .await
        .expect("load target");

    let err = core
        .set_schema_state("UserPublic", SchemaState::Approved)
        .await
        .expect_err("approval should reject missing source field");

    match err {
        SchemaError::InvalidData(msg) => {
            assert!(
                msg.contains("source field 'User.phantom'")
                    && msg.contains("missing from runtime_fields"),
                "expected missing-source-field error, got: {msg}",
            );
        }
        other => panic!("expected SchemaError::InvalidData, got {other:?}"),
    }
}
