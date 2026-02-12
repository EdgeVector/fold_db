use fold_db::datafold_node::DataFoldNode;
use fold_db::schema::SchemaState;
use fold_db::NodeConfig;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;

mod common;
use common::create_test_mutation;

/// Test that searching for a field name (like "email") returns all records with that field
#[tokio::test(flavor = "multi_thread")]
async fn test_search_by_field_name() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().to_path_buf();

    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let config = NodeConfig::new(db_path)
        .with_schema_service_url("test://mock")
        .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());
    let node = DataFoldNode::new(config)
        .await
        .expect("failed to create DataFoldNode");

    {
        let fold_db = node.get_fold_db().await.expect("failed to get FoldDB");
        // ... (skipping unchanged lines in replacement block is risky with replace_file_content if trying to match large block, better to match specific lines)
        // I will use replace_file_content with specific targets for each change

        let schema_json = json!({
            "name": "UserProfile",
            "key": {
                "hash_field": "id"
            },
            "fields": {
                "id": {},
                "name": {},
                "email": {},
                "phone": {}
            },
            "field_classifications": {
                "name": ["word"],
                "email": ["email", "word"],
                "phone": ["word"]
            }
        });

        let schema_str = serde_json::to_string(&schema_json).expect("schema serialization failed");
        fold_db
            .schema_manager()
            .load_schema_from_json(&schema_str)
            .await
            .expect("failed to load schema");

        fold_db
            .schema_manager()
            .set_schema_state("UserProfile", SchemaState::Approved)
            .await
            .expect("failed to approve schema");
    }

    println!("Creating user profiles...");

    // Create 3 user profiles with email fields
    for i in 1..=3 {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), json!(format!("user{}", i)));
        fields.insert("name".to_string(), json!(format!("User {}", i)));
        fields.insert("email".to_string(), json!(format!("user{}@example.com", i)));
        fields.insert("phone".to_string(), json!(format!("555-000{}", i)));

        let mutation_json = json!({
            "schema_name": "UserProfile",
            "fields_and_values": fields,
            "mutation_type": "Create"
        });

        // Actually, create_test_mutation uses "key" from schema_json.
        // Let's redefine the minimal needed schema structure here or just copy the key config.
        let schema_for_helper = json!({
            "key": {
                "hash_field": "id"
            }
        });

        let mutation = create_test_mutation(&schema_for_helper, mutation_json);

        node.mutate_batch_and_index(vec![mutation])
            .await
            .expect("mutation should succeed");
    }

    // Wait for any background processing to settle
    std::thread::sleep(std::time::Duration::from_secs(1));

    println!("\n========== Searching for 'email' ==========");

    // Search for "email" - should return all records with email field
    let email_results = {
        let fold_db = node.get_fold_db().await.expect("failed to get FoldDB");
        fold_db
            .native_search_all_classifications("email")
            .await
            .expect("search should succeed")
    };

    println!("Found {} results for 'email'", email_results.len());

    // Print results for inspection
    for (i, result) in email_results.iter().enumerate() {
        println!(
            "  Result {}: field={}, schema={}, metadata={:?}, key={:?}",
            i, result.field, result.schema_name, result.metadata, result.key_value
        );
    }

    // We should find:
    // 1. Results where "email" appears in the actual content (email classification results)
    // 2. Results for records that have an "email" field (field name results)

    // Filter field name results (these are from the email field itself)
    let field_name_results: Vec<_> = email_results
        .iter()
        .filter(|r| {
            r.field == "email"
                && r.metadata
                    .as_ref()
                    .and_then(|m| m.get("classification"))
                    .and_then(|c| c.as_str())
                    == Some("field")
        })
        .collect();

    println!(
        "\nField name results (records with 'email' field): {}",
        field_name_results.len()
    );
    assert!(
        field_name_results.len() >= 3,
        "Should find at least 3 records with email field (found {})",
        field_name_results.len()
    );

    // Verify all field name results have key_value (they represent actual records)
    for result in &field_name_results {
        // key_value is always present now (no longer optional)
        assert_eq!(
            result.field, "email",
            "Field name results should be from email field"
        );
    }

    println!("\n✅ Field name search test passed!");
    println!("   Searching for 'email' successfully returned all records with an email field");
}

/// Test searching for field name that doesn't exist
#[tokio::test(flavor = "multi_thread")]
async fn test_search_nonexistent_field_name() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().to_path_buf();

    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let config = NodeConfig::new(db_path)
        .with_schema_service_url("test://mock")
        .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());
    let node = DataFoldNode::new(config)
        .await
        .expect("failed to create DataFoldNode");

    {
        let fold_db = node.get_fold_db().await.expect("failed to get FoldDB");
        // ... (skipping unchanged lines in replacement block is risky with replace_file_content if trying to match large block, better to match specific lines)
        // I will use replace_file_content with specific targets for each change

        let schema_json = json!({
            "name": "BlogPost",
            "key": {
                "hash_field": "id"
            },
            "fields": {
                "id": {},
                "title": {},
                "content": {}
            }
        });

        let schema_str = serde_json::to_string(&schema_json).expect("schema serialization failed");
        fold_db
            .schema_manager()
            .load_schema_from_json(&schema_str)
            .await
            .expect("failed to load schema");

        fold_db
            .schema_manager()
            .set_schema_state("BlogPost", SchemaState::Approved)
            .await
            .expect("failed to approve schema");
    }

    // Create a post
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), json!("post1"));
    fields.insert("title".to_string(), json!("Test Post"));
    fields.insert("content".to_string(), json!("Content here"));

    let mutation_json = json!({
        "schema_name": "BlogPost",
        "fields_and_values": fields,
        "mutation_type": "Create"
    });

    let schema_for_helper = json!({
        "key": {
            "hash_field": "id"
        }
    });

    let mutation = create_test_mutation(&schema_for_helper, mutation_json);

    node.mutate_batch_and_index(vec![mutation])
        .await
        .expect("mutation should succeed");

    // Wait for indexing
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Search for field name that doesn't exist
    let results = {
        let fold_db = node.get_fold_db().await.expect("failed to get FoldDB");
        fold_db
            .native_search_all_classifications("email")
            .await
            .expect("search should succeed")
    };

    assert_eq!(
        results.len(),
        0,
        "Should return no results for non-existent field name"
    );
    println!("✅ Non-existent field name test passed!");
}

/// Test that field name search works alongside regular word search
#[tokio::test(flavor = "multi_thread")]
async fn test_combined_field_name_and_word_search() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().to_path_buf();

    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let config = NodeConfig::new(db_path)
        .with_schema_service_url("test://mock")
        .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());
    let node = DataFoldNode::new(config)
        .await
        .expect("failed to create DataFoldNode");

    {
        let fold_db = node.get_fold_db().await.expect("failed to get FoldDB");
        // ... (skipping unchanged lines in replacement block is risky with replace_file_content if trying to match large block, better to match specific lines)
        // I will use replace_file_content with specific targets for each change

        let schema_json = json!({
            "name": "Article",
            "key": {
                "hash_field": "id"
            },
            "fields": {
                "id": {},
                "title": {},
                "content": {}
            }
        });

        let schema_str = serde_json::to_string(&schema_json).expect("schema serialization failed");
        fold_db
            .schema_manager()
            .load_schema_from_json(&schema_str)
            .await
            .expect("failed to load schema");

        fold_db
            .schema_manager()
            .set_schema_state("Article", SchemaState::Approved)
            .await
            .expect("failed to approve schema");
    }

    // Create an article where the word "title" appears in content
    // AND there's a field named "title"
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), json!("article1"));
    fields.insert("title".to_string(), json!("My Article"));
    fields.insert(
        "content".to_string(),
        json!("The title of this article is important"),
    );

    let mutation_json = json!({
        "schema_name": "Article",
        "fields_and_values": fields,
        "mutation_type": "Create"
    });

    let schema_for_helper = json!({
        "key": {
            "hash_field": "id"
        }
    });

    let mutation = create_test_mutation(&schema_for_helper, mutation_json);

    node.mutate_batch_and_index(vec![mutation])
        .await
        .expect("mutation should succeed");

    // Wait for indexing
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Search for "title" - should find both:
    // 1. The word "title" in the content field
    // 2. The record with a "title" field
    let results = {
        let fold_db = node.get_fold_db().await.expect("failed to get FoldDB");
        fold_db
            .native_search_all_classifications("title")
            .await
            .expect("search should succeed")
    };

    println!("Found {} results for 'title'", results.len());

    // Should have results from both word match and field name match
    assert!(
        results.len() >= 2,
        "Should find at least 2 results: word match in content + field name match (found {})",
        results.len()
    );

    let word_matches = results.iter().filter(|r| r.field == "content").count();

    let field_name_matches = results
        .iter()
        .filter(|r| {
            r.field == "title"
                && r.metadata
                    .as_ref()
                    .and_then(|m| m.get("classification"))
                    .and_then(|c| c.as_str())
                    == Some("field")
        })
        .count();

    println!("  Word matches in content: {}", word_matches);
    println!("  Field name matches: {}", field_name_matches);

    assert!(word_matches >= 1, "Should find 'title' word in content");
    assert!(
        field_name_matches >= 1,
        "Should find record with 'title' field"
    );

    println!("✅ Combined search test passed!");
}
