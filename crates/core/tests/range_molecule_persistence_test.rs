//! Tests verifying that Range schema molecules persist correctly across
//! multiple mutation batches and schema reloads.
//!
//! These tests verify that:
//! 1. sync_molecule_uuids() copies molecule UUIDs from runtime_fields into
//!    field_molecule_uuids so they are persisted when the schema is saved.
//! 2. populate_runtime_fields() restores persisted UUIDs from field_molecule_uuids
//!    before falling back to deterministic derivation.

use fold_db::atom::deterministic_molecule_uuid;
use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field::Field;
use fold_db::schema::types::key_value::KeyValue;
use fold_db::schema::types::mutation::Mutation;
use fold_db::schema::SchemaState;
use fold_db::test_helpers::TestSchemaBuilder;
use fold_db::MutationType;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;

fn file_records_schema_json() -> String {
    TestSchemaBuilder::new("FileRecords")
        .fields(&["content", "file_type"])
        .range_key("source_file")
        .build_json()
}

fn make_mutation(source_file: &str, content: &str, file_type: &str) -> Mutation {
    let mut fields = HashMap::new();
    fields.insert("source_file".to_string(), json!(source_file));
    fields.insert("content".to_string(), json!(content));
    fields.insert("file_type".to_string(), json!(file_type));

    Mutation::new(
        "FileRecords".to_string(),
        fields,
        KeyValue::new(None, Some(source_file.to_string())),
        "test_user".to_string(),
        MutationType::Create,
    )
}

/// After a mutation, sync_molecule_uuids should populate field_molecule_uuids
/// so the schema round-trips through the database with molecule UUIDs intact.
/// Then populate_runtime_fields (called on deserialization) should restore those
/// UUIDs into runtime_fields rather than always deriving fresh ones.
///
/// This simulates a server restart: write data, save schema to DB, reload schema
/// from DB, and verify the molecule UUIDs are still present.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mutations_work_after_simulated_restart() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().to_str().expect("path");
    let fold_db = FoldDB::new(db_path).await.expect("create FoldDB");

    fold_db
        .schema_manager()
        .load_schema_from_json(&file_records_schema_json())
        .await
        .expect("load schema");
    fold_db
        .schema_manager()
        .set_schema_state("FileRecords", SchemaState::Approved)
        .await
        .expect("approve");

    // Write first mutation
    let mutation1 = make_mutation("original.txt", "Original content", "text");
    fold_db
        .mutation_manager()
        .write_mutations_batch_async(vec![mutation1])
        .await
        .expect("write mutation 1");

    // Verify field_molecule_uuids was populated by sync_molecule_uuids
    {
        let schemas = fold_db.schema_manager().get_schemas().expect("get schemas");
        let schema = schemas.get("FileRecords").expect("schema exists");
        assert!(
            schema
                .field_molecule_uuids
                .as_ref()
                .is_some_and(|m| !m.is_empty()),
            "should have field_molecule_uuids after mutation"
        );
    }

    // Simulate restart: reload schema from DB (which calls populate_runtime_fields)
    let reloaded = fold_db
        .db_ops()
        .get_schema("FileRecords")
        .await
        .unwrap()
        .expect("schema in DB");

    // Verify molecule_uuid is restored on runtime fields from persisted field_molecule_uuids
    let field = reloaded.runtime_fields.get("source_file").expect("field");
    assert!(
        field.common().molecule_uuid().is_some(),
        "molecule_uuid should be restored from field_molecule_uuids"
    );

    // Force the schema manager cache to use the reloaded schema (no in-memory molecule state)
    fold_db
        .schema_manager()
        .update_schema(&reloaded)
        .await
        .expect("update schema with reloaded version");

    // Write second mutation after simulated restart
    let mutation2 = make_mutation("after_restart.txt", "Post-restart content", "text");
    fold_db
        .mutation_manager()
        .write_mutations_batch_async(vec![mutation2])
        .await
        .expect("write mutation 2 after simulated restart");

    // Verify: both files should be in the molecule
    let schema = fold_db
        .schema_manager()
        .get_schema_metadata("FileRecords")
        .expect("get metadata")
        .expect("schema exists");

    let source_field = schema
        .runtime_fields
        .get("source_file")
        .expect("source_file field");
    let keys = source_field.get_all_keys();
    let mut range_keys: Vec<String> = keys.iter().filter_map(|kv| kv.range.clone()).collect();
    range_keys.sort();

    assert_eq!(
        range_keys,
        vec!["after_restart.txt", "original.txt"],
        "Both files should be in the molecule after simulated restart"
    );

    // FoldDB flushes via shutdown() or Drop — no explicit close needed
}

/// Test that the molecule UUID stays consistent across multiple batches
/// (mutations append to the same molecule, not create new ones).
/// Specifically tests that sync_molecule_uuids persists UUIDs to DB
/// and that they can be read back from the stored schema.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn molecule_uuid_stays_consistent_across_batches() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().to_str().expect("path");
    let fold_db = FoldDB::new(db_path).await.expect("create FoldDB");

    fold_db
        .schema_manager()
        .load_schema_from_json(&file_records_schema_json())
        .await
        .expect("load schema");
    fold_db
        .schema_manager()
        .set_schema_state("FileRecords", SchemaState::Approved)
        .await
        .expect("approve");

    // Write first batch
    let mutation1 = make_mutation("a.txt", "Content A", "text");
    fold_db
        .mutation_manager()
        .write_mutations_batch_async(vec![mutation1])
        .await
        .expect("write mutation 1");

    let mol_uuid_first = {
        let schema = fold_db
            .db_ops()
            .get_schema("FileRecords")
            .await
            .unwrap()
            .expect("schema");
        schema
            .field_molecule_uuids
            .as_ref()
            .expect("mol uuids after first")
            .get("source_file")
            .expect("source_file mol uuid")
            .clone()
    };

    // Write second batch
    let mutation2 = make_mutation("b.txt", "Content B", "text");
    fold_db
        .mutation_manager()
        .write_mutations_batch_async(vec![mutation2])
        .await
        .expect("write mutation 2");

    let mol_uuid_second = {
        let schema = fold_db
            .db_ops()
            .get_schema("FileRecords")
            .await
            .unwrap()
            .expect("schema");
        schema
            .field_molecule_uuids
            .as_ref()
            .expect("mol uuids after second")
            .get("source_file")
            .expect("source_file mol uuid")
            .clone()
    };

    // Molecule UUID must be the same across batches
    assert_eq!(
        mol_uuid_first, mol_uuid_second,
        "Molecule UUID should stay the same across batches (append, not replace)"
    );

    // Also verify it matches the deterministic derivation
    let expected = deterministic_molecule_uuid("FileRecords", "source_file");
    assert_eq!(
        mol_uuid_first, expected,
        "Molecule UUID should match deterministic derivation"
    );

    // FoldDB flushes via shutdown() or Drop — no explicit close needed
}
