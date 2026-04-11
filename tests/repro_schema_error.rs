use fold_db::fold_db_core::fold_db::FoldDB;
use fold_db::test_helpers::TestSchemaBuilder;

#[tokio::test]
async fn test_reproduce_schema_mismatch() {
    // Enable logging
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

    // 1. Setup FoldDB with a temp dir
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let db_path = temp_dir.path().to_str().expect("failed to get path");
    let db = FoldDB::new(db_path).await.expect("Failed to create DB");

    let schema_json = TestSchemaBuilder::new("lowercase_hash")
        .hash_key("id")
        .field("data")
        .build_json();

    // 2. Load "lowercase_hash"
    db.load_schema_from_json(&schema_json)
        .await
        .expect("Failed to load schema");
    db.schema_manager()
        .approve("lowercase_hash")
        .await
        .expect("Failed to approve schema");

    // 3. Test Strict Case (Mismatch)
    // We expect this to return None because strict mode is enabled.
    let result = db
        .schema_manager()
        .get_schema("LOWERCASE_HASH")
        .await
        .expect("get_schema failed");
    assert!(
        result.is_none(),
        "Strict Mode Verification Failed: Found schema even with case mismatch!"
    );

    // 4. Test Exact Match
    // We expect this to return Some(schema)
    let result_exact = db
        .schema_manager()
        .get_schema("lowercase_hash")
        .await
        .expect("get_schema failed");
    assert!(
        result_exact.is_some(),
        "Exact Match Verification Failed: Could not find schema with correct case!"
    );

    println!("Strict case sensitivity verified successfully.");
}
