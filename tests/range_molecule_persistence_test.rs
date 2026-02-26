//! Tests verifying that Range schema molecules persist correctly across
//! multiple mutation batches and schema reloads.
//!
//! These tests exercise two bugs that were fixed:
//! 1. Schema cache overwrite: re-loading a schema from JSON (as ingestion does
//!    for each file) was replacing the cached schema and losing molecule state.
//! 2. Missing molecule refresh: after deserialization from DB (e.g., server
//!    restart), fields had molecule_uuid but molecule=None, causing
//!    write_mutation to create a new molecule instead of appending.

use fold_db::fold_db_core::FoldDB;
use fold_db::fold_node::config::NodeConfig;
use fold_db::fold_node::FoldNode;
use fold_db::fold_node::OperationProcessor;
use fold_db::schema::SchemaState;
use fold_db::schema::types::field::Field;
use fold_db::schema::types::key_value::KeyValue;
use fold_db::schema::types::operations::Query;
use fold_db::MutationType;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;

mod common;

fn file_records_schema_json() -> serde_json::Value {
    json!({
        "name": "FileRecords",
        "key": { "range_field": "source_file" },
        "fields": ["source_file", "content", "file_type"]
    })
}

/// Helper: create a FoldNode backed by the given path.
async fn create_node(db_path: &str) -> FoldNode {
    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let config = NodeConfig::new(db_path.into())
        .with_schema_service_url("test://mock")
        .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());
    FoldNode::new(config).await.expect("create FoldNode")
}

/// Helper: load the FileRecords schema and approve it.
async fn setup_schema(node: &FoldNode) {
    let schema_str = serde_json::to_string(&file_records_schema_json()).unwrap();
    let fold_db = node.get_fold_db().await.expect("get fold_db");
    fold_db
        .schema_manager()
        .load_schema_from_json(&schema_str)
        .await
        .expect("load schema");
    fold_db
        .schema_manager()
        .set_schema_state("FileRecords", SchemaState::Approved)
        .await
        .expect("approve schema");
}

/// Helper: execute a single Range mutation for one file.
async fn write_file_mutation(processor: &OperationProcessor, source_file: &str, content: &str, file_type: &str) {
    let mut fields = HashMap::new();
    fields.insert("source_file".to_string(), json!(source_file));
    fields.insert("content".to_string(), json!(content));
    fields.insert("file_type".to_string(), json!(file_type));

    processor
        .execute_mutation(
            "FileRecords".to_string(),
            fields,
            KeyValue::new(None, Some(source_file.to_string())),
            MutationType::Create,
        )
        .await
        .expect("execute mutation");
}

/// Helper: query all FileRecords and return the source_file values.
async fn query_source_files(processor: &OperationProcessor) -> Vec<String> {
    let query = Query::new(
        "FileRecords".to_string(),
        vec!["source_file".to_string()],
    );
    let result = processor
        .execute_query_map(query)
        .await
        .expect("execute query");

    let field_results = result.get("source_file").expect("source_file in results");
    let mut files: Vec<String> = field_results
        .values()
        .map(|fv| fv.value.as_str().unwrap().to_string())
        .collect();
    files.sort();
    files
}

/// Test that multiple mutation batches on the same Range schema accumulate
/// range keys correctly. Each batch simulates a separate file being ingested.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn range_schema_multiple_batches_preserve_all_data() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().to_str().expect("path");
    let node = create_node(db_path).await;

    setup_schema(&node).await;
    let processor = OperationProcessor::new(node.clone());

    // Write 3 mutations in SEPARATE calls (simulates per-file ingestion)
    write_file_mutation(&processor, "notes.txt", "Meeting notes", "text").await;
    write_file_mutation(&processor, "report.pdf", "Quarterly report", "pdf").await;
    write_file_mutation(&processor, "photo.jpg", "Beach sunset", "image").await;

    // Verify all 3 range keys are queryable
    let files = query_source_files(&processor).await;
    assert_eq!(
        files,
        vec!["notes.txt", "photo.jpg", "report.pdf"],
        "All 3 files should be queryable"
    );
}

/// Test that re-loading a schema from JSON (as ingestion does for each file)
/// does not overwrite molecule state when the schema already exists locally.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schema_reload_from_json_preserves_molecules() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().to_str().expect("path");
    let node = create_node(db_path).await;

    setup_schema(&node).await;
    let processor = OperationProcessor::new(node.clone());

    // Write first file
    write_file_mutation(&processor, "file1.txt", "Content 1", "text").await;

    // Simulate what ingestion does for the SECOND file: reload schema from JSON.
    // Before the fix, this replaced the cached schema's molecule state.
    let schema_str = serde_json::to_string(&file_records_schema_json()).unwrap();
    {
        let fold_db = node.get_fold_db().await.expect("get fold_db");
        fold_db
            .schema_manager()
            .load_schema_from_json(&schema_str)
            .await
            .expect("reload schema");
    }

    // Write second file AFTER the schema reload
    write_file_mutation(&processor, "file2.txt", "Content 2", "text").await;

    // Both files must be present
    let files = query_source_files(&processor).await;
    assert_eq!(
        files,
        vec!["file1.txt", "file2.txt"],
        "Both files should survive schema reload"
    );
}

/// Test that mutations work when runtime_fields have molecule_uuid but no
/// loaded molecule (simulates the state after server restart / DB reload).
/// After deserialization, `populate_runtime_fields()` creates fresh fields
/// with molecule_uuid set but molecule=None. The mutation manager must
/// refresh molecules from DB before writing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mutations_work_after_simulated_restart() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().to_str().expect("path");
    let mut fold_db = FoldDB::new(db_path).await.expect("create FoldDB");

    let schema_str = serde_json::to_string(&file_records_schema_json()).unwrap();
    fold_db
        .schema_manager()
        .load_schema_from_json(&schema_str)
        .await
        .expect("load schema");
    fold_db
        .transform_manager()
        .db_ops
        .store_schema_state("FileRecords", &SchemaState::Approved)
        .await
        .expect("approve");

    // Write first mutation normally
    let mutation1 = common::create_test_mutation(
        &file_records_schema_json(),
        json!({
            "schema_name": "FileRecords",
            "pub_key": "test_user",
            "fields_and_values": {
                "source_file": "original.txt",
                "content": "Original content",
                "file_type": "text"
            }
        }),
    );
    fold_db
        .mutation_manager_mut()
        .write_mutations_batch_async(vec![mutation1])
        .await
        .expect("write mutation 1");

    // Simulate what happens after server restart: the schema is reloaded from DB.
    // populate_runtime_fields() creates fresh fields with molecule_uuid set
    // (from field_molecule_uuids) but molecule=None (not loaded from DB).
    // This is the exact state that caused the bug — write_mutation would see
    // molecule.is_none() and create a new molecule instead of appending.
    {
        let schemas = fold_db.schema_manager().get_schemas().expect("get schemas");
        let schema = schemas.get("FileRecords").expect("schema exists");

        // Verify molecule UUIDs exist before the simulated restart
        assert!(
            schema.field_molecule_uuids.as_ref().is_some_and(|m| !m.is_empty()),
            "should have field_molecule_uuids after mutation"
        );

        // Reload schema from DB (which calls populate_runtime_fields internally)
        // This creates fresh runtime_fields with molecule_uuid set but molecule=None
        let reloaded: fold_db::schema::types::Schema =
            fold_db.transform_manager().db_ops
                .get_schema("FileRecords")
                .await
                .unwrap()
                .expect("schema in DB");

        // Verify the reload state: molecule_uuid set but molecule is None
        let field = reloaded.runtime_fields.get("source_file").expect("field");
        assert!(
            field.common().molecule_uuid().is_some(),
            "molecule_uuid should be restored from field_molecule_uuids"
        );
        // The molecule itself is None (not loaded from DB yet)
        let keys = field.get_all_keys();
        let range_keys: Vec<_> = keys.iter().filter_map(|kv| kv.range.clone()).collect();
        assert!(
            range_keys.is_empty(),
            "Molecule data should not be loaded yet (simulates restart state)"
        );

        // Force the schema manager cache to use the reloaded schema (no molecules).
        // Use update_schema which replaces both DB and cache entries.
        fold_db.schema_manager()
            .update_schema(&reloaded)
            .await
            .expect("update schema with reloaded version");
    }

    // Write second mutation — this must refresh the molecule from DB before writing
    let mutation2 = common::create_test_mutation(
        &file_records_schema_json(),
        json!({
            "schema_name": "FileRecords",
            "pub_key": "test_user",
            "fields_and_values": {
                "source_file": "after_restart.txt",
                "content": "Post-restart content",
                "file_type": "text"
            }
        }),
    );
    fold_db
        .mutation_manager_mut()
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
    let mut range_keys: Vec<String> = keys
        .iter()
        .filter_map(|kv| kv.range.clone())
        .collect();
    range_keys.sort();

    assert_eq!(
        range_keys,
        vec!["after_restart.txt", "original.txt"],
        "Both files should be in the molecule after simulated restart"
    );

    fold_db.close().expect("close");
}

/// Test that the molecule UUID stays consistent across multiple batches
/// (mutations append to the same molecule, not create new ones).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn molecule_uuid_stays_consistent_across_batches() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().to_str().expect("path");
    let node = create_node(db_path).await;

    setup_schema(&node).await;
    let processor = OperationProcessor::new(node.clone());

    // Write first batch
    write_file_mutation(&processor, "a.txt", "Content A", "text").await;

    let mol_uuid_first = {
        let fold_db = node.get_fold_db().await.expect("get fold_db");
        let db_ops = &fold_db.transform_manager().db_ops;
        let schema = db_ops.get_schema("FileRecords").await.unwrap().expect("schema");
        schema
            .field_molecule_uuids
            .as_ref()
            .expect("mol uuids after first")
            .get("source_file")
            .expect("source_file mol uuid")
            .clone()
    };

    // Write second batch
    write_file_mutation(&processor, "b.txt", "Content B", "text").await;

    let mol_uuid_second = {
        let fold_db = node.get_fold_db().await.expect("get fold_db");
        let db_ops = &fold_db.transform_manager().db_ops;
        let schema = db_ops.get_schema("FileRecords").await.unwrap().expect("schema");
        schema
            .field_molecule_uuids
            .as_ref()
            .expect("mol uuids after second")
            .get("source_file")
            .expect("source_file mol uuid")
            .clone()
    };

    // Molecule UUID must be the same — mutations should append, not create new molecules
    assert_eq!(
        mol_uuid_first, mol_uuid_second,
        "Molecule UUID should stay the same across batches (append, not replace)"
    );
}
