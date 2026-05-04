//! End-to-end integration test for `Mutation.provenance` — the additive
//! `Option<Provenance>` field added in `projects/molecule-provenance-dag`.
//!
//! PR 4 added the field on `Mutation`. PR 5 wires it through the write
//! path so that `MutationEvent` records propagate the originating
//! mutation's provenance.
//!
//! As of the cross-node share-replay fix (face-discovery-3node), a
//! mutation that arrives with `Some(Provenance::User { pubkey, .. })`
//! also propagates that pubkey onto the `AtomEntry.writer_pubkey` of the
//! per-key molecule entry — so a query response on the receiving node can
//! attribute the record to its original author rather than to the local
//! signer. See `mutation_provenance_user_propagates_to_atom_entry_writer_pubkey`
//! below for the load-bearing assertion.

use fold_db::atom::deterministic_molecule_uuid;
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

/// PR 5 — the originating mutation's `Provenance::User` must appear on the
/// resulting `MutationEvent` record. This is the end-to-end propagation
/// guarantee: a submitter who signs once has that signature recorded at
/// every durable layer, not only on the in-flight `Mutation` struct.
#[tokio::test]
async fn mutation_provenance_propagates_to_mutation_event() {
    let db = setup_db().await;
    db.load_schema_from_json(&person_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("Person", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("Carol"));
    fields.insert("created_at".to_string(), json!("2026-01-03"));

    let provenance = Provenance::user("submitter-pubkey".to_string(), "submitter-sig".to_string());
    let mutation = Mutation::new(
        "Person".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-03".to_string())),
        "pk".to_string(),
        MutationType::Create,
    )
    .with_provenance(provenance.clone());

    db.mutation_manager()
        .write_mutations_batch_async(vec![mutation])
        .await
        .expect("write should succeed");

    let mol_uuid = deterministic_molecule_uuid("Person", "name");
    let events = db
        .db_ops()
        .get_mutation_events(&mol_uuid, None)
        .await
        .expect("read events");
    assert!(
        !events.is_empty(),
        "expected at least one event for Person.name"
    );
    assert!(
        events
            .iter()
            .all(|e| e.provenance.as_ref() == Some(&provenance)),
        "every MutationEvent for Person.name must carry the submitter's \
         provenance; got {:?}",
        events.iter().map(|e| &e.provenance).collect::<Vec<_>>()
    );
}

/// Cross-node share replay: a mutation submitted with
/// `Provenance::User { pubkey, .. }` MUST land on the per-key AtomEntry
/// with `writer_pubkey == pubkey` (the original author), not with the
/// receiving node's local signer pubkey. The query response surfaces this
/// via `FieldValue.writer_pubkey`. This is the load-bearing property for
/// `bob.shared_record_count[Photography] >= 1` in the e2e
/// face-discovery-3node scenario.
#[tokio::test]
async fn mutation_provenance_user_propagates_to_atom_entry_writer_pubkey() {
    let db = setup_db().await;
    db.load_schema_from_json(&person_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("Person", SchemaState::Approved)
        .await
        .unwrap();

    let alice_pubkey = "alice-pubkey-base64".to_string();
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("Dave"));
    fields.insert("created_at".to_string(), json!("2026-01-04"));

    // signature_version=0 means "imported without a verifiable signature"
    // — the path inbound `data_share` takes when the receiving node has
    // the sender's pubkey but not a signature over the receiver's local
    // canonical bytes.
    let provenance = Provenance::User {
        pubkey: alice_pubkey.clone(),
        signature: String::new(),
        signature_version: 0,
    };
    let mutation = Mutation::new(
        "Person".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-04".to_string())),
        alice_pubkey.clone(),
        MutationType::Create,
    )
    .with_provenance(provenance);

    db.mutation_manager()
        .write_mutations_batch_async(vec![mutation])
        .await
        .expect("write should succeed");

    let results = db
        .query_executor()
        .query(Query::new("Person".to_string(), vec!["name".to_string()]))
        .await
        .unwrap();
    let name_values = results
        .get("name")
        .expect("Person.name should be queryable");
    let dave = name_values
        .iter()
        .find(|(_, fv)| fv.value == json!("Dave"))
        .expect("Dave should appear in result set");
    assert_eq!(
        dave.1.writer_pubkey.as_deref(),
        Some(alice_pubkey.as_str()),
        "FieldValue.writer_pubkey must be Alice's pubkey (the submitter), \
         not the local signer; got {:?}",
        dave.1.writer_pubkey
    );
}

/// Sibling negative control: a mutation with `provenance = None` (the
/// normal first-party write path) must NOT pick up the local signer's
/// pubkey as `writer_pubkey` for Range fields. The local signer is
/// recorded on the AtomEntry, but it bubbles up to `FieldValue.writer_pubkey`
/// on Hash/Range/HashRange variants — confirming Gap 2 wiring works for
/// the locally-signed path too.
#[tokio::test]
async fn local_mutation_surfaces_local_signer_writer_pubkey_on_range_field() {
    let db = setup_db().await;
    db.load_schema_from_json(&person_schema_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state("Person", SchemaState::Approved)
        .await
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("Eve"));
    fields.insert("created_at".to_string(), json!("2026-01-05"));

    // No provenance — local sign path.
    let mutation = Mutation::new(
        "Person".to_string(),
        fields,
        KeyValue::new(None, Some("2026-01-05".to_string())),
        "ignored-pub-key".to_string(),
        MutationType::Create,
    );

    db.mutation_manager()
        .write_mutations_batch_async(vec![mutation])
        .await
        .expect("write should succeed");

    let results = db
        .query_executor()
        .query(Query::new("Person".to_string(), vec!["name".to_string()]))
        .await
        .unwrap();
    let name_values = results
        .get("name")
        .expect("Person.name should be queryable");
    let eve = name_values
        .iter()
        .find(|(_, fv)| fv.value == json!("Eve"))
        .expect("Eve should appear in result set");
    let pk = eve
        .1
        .writer_pubkey
        .as_deref()
        .expect("locally-signed Range entry must surface a writer_pubkey");
    assert!(!pk.is_empty(), "local signer pubkey must be non-empty");
    // Must NOT be the unrelated `mutation.pub_key` — that's just an
    // identity hint, not the signer.
    assert_ne!(
        pk, "ignored-pub-key",
        "writer_pubkey must reflect the local signing keypair, not mutation.pub_key"
    );
}
