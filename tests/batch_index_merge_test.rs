//! Test that batch indexing correctly merges with existing index entries
//!
//! This test verifies the fix for a bug where batch_index_field_values_with_classifications
//! was replacing index entries instead of merging them, causing records to disappear
//! when multiple batches indexed the same term.

use fold_db::datafold_node::DataFoldNode;
use fold_db::schema::SchemaState;
use fold_db::NodeConfig;
use serde_json::json;
use tempfile::TempDir;

mod common;

#[tokio::test(flavor = "multi_thread")]
async fn test_batch_index_merges_existing_entries() {
    eprintln!("\n=== Testing batch index merge behavior ===\n");

    // Create a test database
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().to_path_buf();

    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let config = NodeConfig::new(db_path)
        .with_schema_service_url("test://mock")
        .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());
    let node = DataFoldNode::new(config)
        .await
        .expect("failed to create DataFoldNode");

    // Create a simple schema with a text field
    let test_schema = json!({
        "name": "TestPost",
        "key": {
            "hash_field": "id"
        },
        "fields": {
            "id": {},
            "content": {}
        }
    });

    {
        let fold_db = node.get_fold_db().await.expect("failed to get FoldDB");

        let schema_str = serde_json::to_string(&test_schema).expect("schema serialization failed");
        fold_db
            .schema_manager()
            .load_schema_from_json(&schema_str)
            .await
            .expect("failed to load schema");

        fold_db
            .schema_manager()
            .set_schema_state("TestPost", SchemaState::Approved)
            .await
            .expect("failed to approve schema");
    }

    // BATCH 1: Create first record with word "foo"
    eprintln!("=== BATCH 1: Creating record A with word 'foo' ===");
    let mutation_a = common::create_test_mutation(
        &test_schema,
        json!({
            "schema_name": "TestPost",
            "pub_key": "test",
            "fields_and_values": {
                "id": "recordA",
                "content": "foo bar"
            },
            "mutation_type": "Create"
        }),
    );

    let results = node.mutate_batch_and_index(vec![mutation_a]).await.unwrap();
    assert_eq!(results.len(), 1, "First batch should process 1 mutation");

    // No sleep needed - mutate_batch_and_index does synchronous indexing

    // Search for "foo" - should find record A
    eprintln!("\n=== Searching for 'foo' after BATCH 1 ===");
    let search_results = {
        let fold_db = node.get_fold_db().await.expect("failed to get FoldDB");
        fold_db
            .native_search_all_classifications("foo")
            .await
            .expect("search should succeed")
    };
    eprintln!("Found {} results for 'foo'", search_results.len());
    assert!(
        !search_results.is_empty(),
        "Should find results for 'foo' after first batch"
    );

    let foo_records: Vec<_> = search_results
        .iter()
        .filter(|r| r.key_value.hash.as_deref() == Some("recordA"))
        .collect();
    assert_eq!(
        foo_records.len(),
        1,
        "Should find exactly 1 occurrence of recordA for 'foo'"
    );

    // BATCH 2: Create second record ALSO with word "foo"
    eprintln!("\n=== BATCH 2: Creating record B with word 'foo' ===");
    let mutation_b = common::create_test_mutation(
        &test_schema,
        json!({
            "schema_name": "TestPost",
            "pub_key": "test",
            "fields_and_values": {
                "id": "recordB",
                "content": "foo baz"
            },
            "mutation_type": "Create"
        }),
    );

    let results = node.mutate_batch_and_index(vec![mutation_b]).await.unwrap();
    assert_eq!(results.len(), 1, "Second batch should process 1 mutation");

    // No sleep needed - mutate_batch_and_index does synchronous indexing

    // THE CRITICAL TEST: Search for "foo" - should find BOTH records A and B
    eprintln!("\n=== Searching for 'foo' after BATCH 2 ===");
    let search_results = {
        let fold_db = node.get_fold_db().await.expect("failed to get FoldDB");
        fold_db
            .native_search_all_classifications("foo")
            .await
            .expect("search should succeed")
    };
    eprintln!("Found {} results for 'foo'", search_results.len());

    for result in &search_results {
        eprintln!(
            "  - Record: {:?}, Field: {}, Value: {}",
            result.key_value, result.field, result.value
        );
    }

    assert!(
        !search_results.is_empty(),
        "Should find results for 'foo' after second batch"
    );

    // Verify we have both recordA and recordB
    let record_a_count = search_results
        .iter()
        .filter(|r| r.key_value.hash.as_deref() == Some("recordA"))
        .count();
    let record_b_count = search_results
        .iter()
        .filter(|r| r.key_value.hash.as_deref() == Some("recordB"))
        .count();

    eprintln!("\nRecord counts:");
    eprintln!("  recordA: {}", record_a_count);
    eprintln!("  recordB: {}", record_b_count);

    assert_eq!(record_a_count, 1,
        "BUG: recordA disappeared after second batch! The batch indexing is replacing instead of merging.");
    assert_eq!(
        record_b_count, 1,
        "recordB should be present after second batch"
    );

    eprintln!("\n✅ SUCCESS: Both records are present in the index!");
    eprintln!("The batch indexing correctly merged new entries with existing ones.\n");
}
