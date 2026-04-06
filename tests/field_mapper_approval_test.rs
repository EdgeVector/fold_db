use fold_db::atom::deterministic_molecule_uuid;
use fold_db::schema::types::field::Field;
use fold_db::schema::{SchemaCore, SchemaState};

fn user_schema_json() -> String {
    r#"{
        "name": "User",
        "key": { "range_field": "created_at" },
        "fields": {
            "id": {},
            "name": {},
            "created_at": {}
        }
    }"#
    .to_string()
}

fn user_public_schema_json() -> String {
    r#"{
        "name": "UserPublic",
        "key": { "range_field": "created_at" },
        "fields": {
            "id": {},
            "display_name": {},
            "view_count": {},
            "is_featured": {}
        },
        "field_mappers": {
            "id": "User.id",
            "display_name": "User.name"
        }
    }"#
    .to_string()
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
