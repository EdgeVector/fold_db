//! End-to-end integration test for `Mutation.provenance` — the additive
//! `Option<Provenance>` field added in `projects/molecule-provenance-dag`
//! PR 4. The field is not yet persisted to atoms/molecules (PR 5), so
//! "round-trip" here means: a `Mutation` carrying `Provenance::User`
//! survives serde, feeds cleanly into `MutationManager::write_mutations_batch_async`,
//! and the underlying data lands on the target schema just as it would for
//! a pre-PR-4 mutation.

use fold_db::atom::provenance::Provenance;
use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::test_helpers::TestSchemaBuilder;
use serde_json::json;
use std::collections::HashMap;

async fn setup_db() -> FoldDB {
    let dir = tempfile::tempdir().unwrap();
    FoldDB::new(dir.path().to_str().unwrap()).await.unwrap()
}

fn person_schema_json() -> String {
    TestSchemaBuilder::new("Person")
        .fields(&["name"])
        .range_key("created_at")
        .build_json()
}

#[tokio::test]
async fn mutation_with_provenance_user_writes_and_serde_round_trips() {
    let db = setup_db().await;
    db.load_schema_from_json(&person_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("Person", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("Alice"));
    fields.insert("created_at".to_string(), json!("2026-01-01"));

    let provenance = Provenance::user("pubkey-b64".to_string(), "signature-b64".to_string());
    let mutation = Mutation::new(
        "Person".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-01".to_string())),
        "pk".to_string(),
        MutationType::Create,
    )
    .with_provenance(provenance.clone());

    // Serde round-trip (simulating a sync-log entry) before handing to the
    // manager. The deserialized Mutation must still carry the provenance.
    let json_wire = serde_json::to_string(&mutation).expect("serialize");
    let replayed: Mutation = serde_json::from_str(&json_wire).expect("deserialize");
    assert_eq!(replayed.provenance, Some(provenance));

    // The write path must succeed with the new field present.
    db.mutation_manager()
        .write_mutations_batch_async(vec![replayed])
        .await
        .expect("write should succeed with provenance set");

    // The underlying data landed on Person as expected.
    let results = db
        .query_executor()
        .query(Query::new("Person".to_string(), vec!["name".to_string()]))
        .await
        .unwrap();
    assert!(
        results.contains_key("name"),
        "Person.name should be queryable after write"
    );
    let name_values = &results["name"];
    assert!(
        name_values.iter().any(|(_, fv)| fv.value == json!("Alice")),
        "expected Alice in result set, got {:?}",
        name_values
    );
}

#[tokio::test]
async fn mutation_without_provenance_still_writes() {
    let db = setup_db().await;
    db.load_schema_from_json(&person_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("Person", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("Bob"));
    fields.insert("created_at".to_string(), json!("2026-01-02"));

    // Mutation::new leaves provenance as None — existing behavior.
    let mutation = Mutation::new(
        "Person".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-02".to_string())),
        "pk".to_string(),
        MutationType::Create,
    );
    assert_eq!(mutation.provenance, None);

    db.mutation_manager()
        .write_mutations_batch_async(vec![mutation])
        .await
        .expect("write should succeed with provenance = None");
}
