use datafold::schema::{Field, SchemaCore, SchemaState};

fn user_schema_json() -> String {
    r#"{
        "name": "User",
        "key": { "range_field": "created_at" },
        "fields": {
            "id": {},
            "name": {},
            "created_at": {}
        },
        "field_molecule_uuids": {
            "id": "uuid-user-id",
            "name": "uuid-user-name"
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
        .get_schema("UserPublic")
        .expect("fetch schema")
        .expect("schema exists");

    assert!(
        initial_target_schema
            .runtime_fields
            .get("id")
            .and_then(|field| field.common().molecule_uuid())
            .is_none(),
        "target id field should not have molecule before approval"
    );

    core.set_schema_state("UserPublic", SchemaState::Approved)
        .await
        .expect("approve schema");

    let approved_schema = core
        .get_schema("UserPublic")
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

    assert_eq!(id_uuid, "uuid-user-id");
    assert_eq!(display_uuid, "uuid-user-name");
}
