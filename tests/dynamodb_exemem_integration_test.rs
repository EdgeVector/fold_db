/// Integration test for DynamoDB storage with exemem's table structure
///
/// This test validates that the storage abstraction works correctly with:
/// - Separate DynamoDB tables per namespace (feature)
/// - User IDs as partition keys
/// - Actual keys as sort keys
///
/// Run with LocalStack:
/// ```bash
/// # Start LocalStack
/// docker run -d -p 4566:4566 localstack/localstack
///
/// # Set environment variable
/// export AWS_ENDPOINT_URL=http://localhost:4566
///
/// # Run test
/// cargo test --test dynamodb_exemem_integration_test -- --ignored --nocapture
/// ```

use datafold::db_operations::DbOperationsV2;
use aws_sdk_dynamodb::{Client, types::{AttributeDefinition, KeySchemaElement, KeyType, ScalarAttributeType, BillingMode}};

async fn create_test_tables(client: &Client, base_name: &str, namespaces: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    for namespace in namespaces {
        let table_name = format!("{}-{}", base_name, namespace);
        
        println!("📋 Creating table: {}", table_name);
        
        let result = client
            .create_table()
            .table_name(&table_name)
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("PK")
                    .key_type(KeyType::Hash)
                    .build()
                    .unwrap()
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("SK")
                    .key_type(KeyType::Range)
                    .build()
                    .unwrap()
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("PK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .unwrap()
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("SK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .unwrap()
            )
            .billing_mode(BillingMode::PayPerRequest)
            .send()
            .await;
            
        match result {
            Ok(_) => println!("✅ Created table: {}", table_name),
            Err(e) => println!("⚠️  Table {} may already exist: {}", table_name, e),
        }
    }
    
    Ok(())
}

async fn verify_partition_key_structure(
    client: &Client,
    table_name: &str,
    expected_pk: &str,
    expected_sk: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let result = client
        .get_item()
        .table_name(table_name)
        .key("PK", aws_sdk_dynamodb::types::AttributeValue::S(expected_pk.to_string()))
        .key("SK", aws_sdk_dynamodb::types::AttributeValue::S(expected_sk.to_string()))
        .send()
        .await?;
        
    Ok(result.item.is_some())
}

#[tokio::test]
#[ignore] // Run with: cargo test --test dynamodb_exemem_integration_test -- --ignored --nocapture
async fn test_exemem_dynamodb_structure() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  Testing DynamoDB Storage with Exemem Table Structure     ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
    
    // Setup AWS client with LocalStack endpoint
    let endpoint_url = std::env::var("AWS_ENDPOINT_URL")
        .unwrap_or_else(|_| "http://localhost:4566".to_string());
    
    println!("🔗 Using endpoint: {}", endpoint_url);
    
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region("us-east-1")
        .endpoint_url(&endpoint_url)
        .load()
        .await;
    
    let client = Client::new(&config);
    
    // Create tables matching exemem's structure
    let base_table_name = "TestDataFoldStorage";
    let namespaces = vec!["main", "metadata", "schemas", "transforms"];
    
    create_test_tables(&client, base_table_name, &namespaces).await?;
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 1: Single User with Multi-Tenant Isolation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    // Test with user_123
    let user_id = "user_123";
    println!("👤 Testing with user: {}", user_id);
    
    let db_ops = DbOperationsV2::from_dynamodb(
        client.clone(),
        base_table_name.to_string(),
        Some(user_id.to_string())
    ).await?;
    
    // Store some data
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    struct TestData {
        name: String,
        value: i32,
    }
    
    let test_item = TestData {
        name: "test_item".to_string(),
        value: 42,
    };
    
    println!("💾 Storing item: {:?}", test_item);
    db_ops.store_item("test:item1", &test_item).await?;
    
    // Verify it's stored in the correct partition
    let table_name = format!("{}-main", base_table_name);
    let exists = verify_partition_key_structure(
        &client,
        &table_name,
        user_id,  // PK should be user_id
        "test:item1",  // SK should be the actual key
    ).await?;
    
    assert!(exists, "Item should exist with PK={} and SK=test:item1", user_id);
    println!("✅ Item stored with correct partition key structure");
    println!("   PK = {}", user_id);
    println!("   SK = test:item1");
    
    // Retrieve and verify
    let retrieved: Option<TestData> = db_ops.get_item("test:item1").await?;
    assert_eq!(retrieved, Some(test_item));
    println!("✅ Item retrieved successfully");
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 2: Multiple Users - Data Isolation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    // Create separate instances for different users
    let user_alice = "alice_456";
    let user_bob = "bob_789";
    
    println!("👥 Testing isolation between {} and {}", user_alice, user_bob);
    
    let db_ops_alice = DbOperationsV2::from_dynamodb(
        client.clone(),
        base_table_name.to_string(),
        Some(user_alice.to_string())
    ).await?;
    
    let db_ops_bob = DbOperationsV2::from_dynamodb(
        client.clone(),
        base_table_name.to_string(),
        Some(user_bob.to_string())
    ).await?;
    
    // Alice stores data
    let alice_data = TestData {
        name: "alice_secret".to_string(),
        value: 100,
    };
    db_ops_alice.store_item("secret:data", &alice_data).await?;
    println!("💾 Alice stored: {:?}", alice_data);
    
    // Bob stores data with same key
    let bob_data = TestData {
        name: "bob_secret".to_string(),
        value: 200,
    };
    db_ops_bob.store_item("secret:data", &bob_data).await?;
    println!("💾 Bob stored: {:?}", bob_data);
    
    // Verify Alice gets her data (not Bob's)
    let alice_retrieved: Option<TestData> = db_ops_alice.get_item("secret:data").await?;
    assert_eq!(alice_retrieved, Some(alice_data.clone()));
    println!("✅ Alice retrieved her own data: {:?}", alice_retrieved);
    
    // Verify Bob gets his data (not Alice's)
    let bob_retrieved: Option<TestData> = db_ops_bob.get_item("secret:data").await?;
    assert_eq!(bob_retrieved, Some(bob_data.clone()));
    println!("✅ Bob retrieved his own data: {:?}", bob_retrieved);
    
    // Verify partition keys are different
    let alice_exists = verify_partition_key_structure(
        &client,
        &table_name,
        user_alice,
        "secret:data",
    ).await?;
    
    let bob_exists = verify_partition_key_structure(
        &client,
        &table_name,
        user_bob,
        "secret:data",
    ).await?;
    
    assert!(alice_exists, "Alice's data should exist in her partition");
    assert!(bob_exists, "Bob's data should exist in his partition");
    println!("✅ Data correctly isolated in separate partitions:");
    println!("   Alice: PK={}, SK=secret:data", user_alice);
    println!("   Bob:   PK={}, SK=secret:data", user_bob);
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 3: Prefix Scanning within User Partition");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    // Store multiple items with same prefix for Alice
    for i in 1..=5 {
        let item = TestData {
            name: format!("item_{}", i),
            value: i,
        };
        db_ops_alice.store_item(&format!("prefix:item{}", i), &item).await?;
    }
    println!("💾 Stored 5 items with 'prefix:' for Alice");
    
    // Store items with different prefix
    for i in 1..=3 {
        let item = TestData {
            name: format!("other_{}", i),
            value: i * 10,
        };
        db_ops_alice.store_item(&format!("other:item{}", i), &item).await?;
    }
    println!("💾 Stored 3 items with 'other:' for Alice");
    
    // Scan for prefix items
    let prefix_items = db_ops_alice.list_items_with_prefix("prefix:").await?;
    assert_eq!(prefix_items.len(), 5, "Should find exactly 5 items with 'prefix:'");
    println!("✅ Found {} items with 'prefix:' prefix", prefix_items.len());
    
    // Scan for other items
    let other_items = db_ops_alice.list_items_with_prefix("other:").await?;
    assert_eq!(other_items.len(), 3, "Should find exactly 3 items with 'other:'");
    println!("✅ Found {} items with 'other:' prefix", other_items.len());
    
    // Verify Bob doesn't see Alice's items
    let bob_prefix_items = db_ops_bob.list_items_with_prefix("prefix:").await?;
    assert_eq!(bob_prefix_items.len(), 0, "Bob should not see Alice's items");
    println!("✅ Bob correctly sees 0 items (isolated from Alice)");
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 4: Multiple Namespaces (Separate Tables)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    let user_charlie = "charlie_999";
    let db_ops_charlie = DbOperationsV2::from_dynamodb(
        client.clone(),
        base_table_name.to_string(),
        Some(user_charlie.to_string())
    ).await?;
    
    // Store in main namespace
    let main_data = TestData {
        name: "main_data".to_string(),
        value: 111,
    };
    db_ops_charlie.store_item("data:main", &main_data).await?;
    println!("💾 Stored in 'main' namespace");
    
    // Store in metadata namespace
    let metadata_data = TestData {
        name: "metadata_data".to_string(),
        value: 222,
    };
    db_ops_charlie.store_in_namespace("metadata", "data:meta", &metadata_data).await?;
    println!("💾 Stored in 'metadata' namespace");
    
    // Verify data is in different tables
    let main_exists = verify_partition_key_structure(
        &client,
        &format!("{}-main", base_table_name),
        user_charlie,
        "data:main",
    ).await?;
    
    let metadata_exists = verify_partition_key_structure(
        &client,
        &format!("{}-metadata", base_table_name),
        user_charlie,
        "data:meta",
    ).await?;
    
    assert!(main_exists, "Data should exist in main table");
    assert!(metadata_exists, "Data should exist in metadata table");
    println!("✅ Data correctly stored in separate tables:");
    println!("   Table: {}-main", base_table_name);
    println!("   Table: {}-metadata", base_table_name);
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 5: Batch Operations with Correct Partition Keys");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    let user_dave = "dave_111";
    let db_ops_dave = DbOperationsV2::from_dynamodb(
        client.clone(),
        base_table_name.to_string(),
        Some(user_dave.to_string())
    ).await?;
    
    // Batch store
    let batch_items: Vec<(String, TestData)> = (1..=10)
        .map(|i| {
            (
                format!("batch:item{}", i),
                TestData {
                    name: format!("batch_{}", i),
                    value: i * 100,
                }
            )
        })
        .collect();
    
    db_ops_dave.batch_store_items(&batch_items).await?;
    println!("💾 Batch stored {} items", batch_items.len());
    
    // Verify all items exist in Dave's partition
    for (key, _) in &batch_items {
        let exists = verify_partition_key_structure(
            &client,
            &table_name,
            user_dave,
            key,
        ).await?;
        assert!(exists, "Item {} should exist in Dave's partition", key);
    }
    println!("✅ All {} batch items verified in correct partition (PK={})", batch_items.len(), user_dave);
    
    // Test batch retrieval
    let retrieved_items = db_ops_dave.list_items_with_prefix("batch:").await?;
    assert_eq!(retrieved_items.len(), 10, "Should retrieve all 10 batch items");
    println!("✅ Retrieved all {} batch items via prefix scan", retrieved_items.len());
    
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  ✅ ALL TESTS PASSED - DynamoDB Structure Verified!       ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
    
    println!("Summary:");
    println!("  ✅ User IDs correctly used as partition keys (PK)");
    println!("  ✅ Actual keys correctly used as sort keys (SK)");
    println!("  ✅ Multi-tenant isolation works correctly");
    println!("  ✅ Prefix scanning works within user partitions");
    println!("  ✅ Multiple namespaces use separate tables");
    println!("  ✅ Batch operations maintain correct partition keys");
    println!("\nThe storage abstraction is fully compatible with exemem's");
    println!("'separate table per feature with user IDs as partition keys' design!\n");
    
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_single_tenant_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  Testing Single-Tenant Mode (No User ID)                  ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
    
    let endpoint_url = std::env::var("AWS_ENDPOINT_URL")
        .unwrap_or_else(|_| "http://localhost:4566".to_string());
    
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region("us-east-1")
        .endpoint_url(&endpoint_url)
        .load()
        .await;
    
    let client = Client::new(&config);
    
    let base_table_name = "TestSingleTenant";
    create_test_tables(&client, base_table_name, &["main"]).await?;
    
    // Create DbOperations without user_id
    let db_ops = DbOperationsV2::from_dynamodb(
        client.clone(),
        base_table_name.to_string(),
        None  // No user_id
    ).await?;
    
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    struct TestData {
        value: String,
    }
    
    let data = TestData {
        value: "single_tenant_data".to_string(),
    };
    
    db_ops.store_item("test:key", &data).await?;
    println!("💾 Stored item without user_id");
    
    // Verify it uses "default" as partition key
    let table_name = format!("{}-main", base_table_name);
    let exists = verify_partition_key_structure(
        &client,
        &table_name,
        "default",  // PK should be "default" when no user_id
        "test:key",
    ).await?;
    
    assert!(exists, "Item should exist with PK=default");
    println!("✅ Item correctly stored with PK='default' (single-tenant mode)");
    
    let retrieved: Option<TestData> = db_ops.get_item("test:key").await?;
    assert_eq!(retrieved, Some(data));
    println!("✅ Item retrieved successfully in single-tenant mode");
    
    println!("\n✅ Single-tenant mode works correctly!");
    
    Ok(())
}
