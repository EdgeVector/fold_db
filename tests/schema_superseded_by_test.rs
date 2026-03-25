use fold_db::db_operations::native_index::FastEmbedModel;
use fold_db::schema::{SchemaCore, SchemaState};
use std::sync::Arc;

fn schema_a_json() -> String {
    r#"{
        "name": "ContactV1",
        "key": { "range_field": "created_at" },
        "fields": {
            "name": {},
            "email": {},
            "created_at": {}
        }
    }"#
    .to_string()
}

fn schema_b_json() -> String {
    r#"{
        "name": "ContactV2",
        "key": { "range_field": "created_at" },
        "fields": {
            "name": {},
            "email": {},
            "phone": {},
            "created_at": {}
        }
    }"#
    .to_string()
}

fn schema_c_json() -> String {
    r#"{
        "name": "ContactV3",
        "key": { "range_field": "created_at" },
        "fields": {
            "name": {},
            "email": {},
            "phone": {},
            "address": {},
            "created_at": {}
        }
    }"#
    .to_string()
}

#[tokio::test]
async fn blocked_with_successor_redirects_to_new_schema() {
    let core = SchemaCore::new_for_testing().await.expect("init core");

    core.load_schema_from_json(&schema_a_json())
        .await
        .expect("load A");
    core.load_schema_from_json(&schema_b_json())
        .await
        .expect("load B");

    // Block A and record B as its successor
    core.block_and_supersede("ContactV1", "ContactV2")
        .await
        .expect("block_and_supersede");

    // get_schema("ContactV1") should redirect to ContactV2
    let schema = core
        .get_schema("ContactV1")
        .await
        .expect("get_schema")
        .expect("schema should exist");
    assert_eq!(schema.name, "ContactV2");

    // State of A should be Blocked
    let states = core.get_schema_states().expect("states");
    assert_eq!(states.get("ContactV1"), Some(&SchemaState::Blocked));
}

#[tokio::test]
async fn blocked_without_successor_returns_none() {
    let core = SchemaCore::new_for_testing().await.expect("init core");

    core.load_schema_from_json(&schema_a_json())
        .await
        .expect("load A");

    // Block A without a successor — no redirect, schema is just gone
    core.block_schema("ContactV1")
        .await
        .expect("block");

    // get_schema should still find it (Blocked doesn't hide from get_schema,
    // only from QueryExecutor)
    let schema = core
        .get_schema("ContactV1")
        .await
        .expect("get_schema");
    assert!(schema.is_some());
}

#[tokio::test]
async fn superseded_chain_redirect_a_to_b_to_c() {
    let core = SchemaCore::new_for_testing().await.expect("init core");

    core.load_schema_from_json(&schema_a_json())
        .await
        .expect("load A");
    core.load_schema_from_json(&schema_b_json())
        .await
        .expect("load B");
    core.load_schema_from_json(&schema_c_json())
        .await
        .expect("load C");

    // A → B → C
    core.block_and_supersede("ContactV1", "ContactV2")
        .await
        .expect("block A→B");
    core.block_and_supersede("ContactV2", "ContactV3")
        .await
        .expect("block B→C");

    // get_schema("ContactV1") should follow chain to ContactV3
    let schema = core
        .get_schema("ContactV1")
        .await
        .expect("get_schema")
        .expect("schema should exist");
    assert_eq!(schema.name, "ContactV3");
}

#[tokio::test]
async fn superseded_by_persists_across_schema_core_instances() {
    use fold_db::fold_db_core::infrastructure::message_bus::AsyncMessageBus;
    use std::sync::Arc;

    // Create a persistent sled DB
    let dir = tempfile::tempdir().expect("tempdir");
    let db = sled::open(dir.path()).expect("open sled");
    let db_ops = Arc::new(
        fold_db::db_operations::DbOperations::from_sled(db, Arc::new(FastEmbedModel::new()))
            .await
            .expect("db_ops"),
    );

    // First instance: load schemas and block with successor
    {
        let bus = Arc::new(AsyncMessageBus::new());
        let core = SchemaCore::new(Arc::clone(&db_ops), bus)
            .await
            .expect("core1");

        core.load_schema_from_json(&schema_a_json())
            .await
            .expect("load A");
        core.load_schema_from_json(&schema_b_json())
            .await
            .expect("load B");

        core.block_and_supersede("ContactV1", "ContactV2")
            .await
            .expect("block_and_supersede");
    }

    // Second instance: verify redirect still works
    {
        let bus = Arc::new(AsyncMessageBus::new());
        let core = SchemaCore::new(Arc::clone(&db_ops), bus)
            .await
            .expect("core2");

        let schema = core
            .get_schema("ContactV1")
            .await
            .expect("get_schema")
            .expect("schema should exist");
        assert_eq!(schema.name, "ContactV2");

        let states = core.get_schema_states().expect("states");
        assert_eq!(states.get("ContactV1"), Some(&SchemaState::Blocked));
    }
}
