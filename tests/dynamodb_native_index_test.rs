/// Test Native Indexing with DynamoDB backend
/// 
/// This test verifies that:
/// 1. Data ingested into DynamoDB is correctly indexed
/// 2. Native index searches return the expected results
/// 3. Empty classifications are handled correctly (defaulting to "word")
/// 
/// Requires LocalStack or real DynamoDB:
/// - Set AWS_ENDPOINT_URL=http://localhost:4566 for LocalStack
/// - Or configure AWS credentials for real DynamoDB

use datafold::db_operations::DbOperationsV2;
use datafold::fold_db_core::MutationManager;
use datafold::schema::{Schema, SchemaCore, SchemaType};
use datafold::storage::dynamodb_backend::DynamoDbNamespacedStore;
use datafold::schema::types::{Mutation, KeyValue};
use datafold::schema::types::operations::MutationType;
use datafold::fold_db_core::infrastructure::MessageBus;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::types::{AttributeDefinition, KeySchemaElement, KeyType, ScalarAttributeType, BillingMode};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

/// Helper to create a test DynamoDB table
async fn create_test_table(client: &Client, table_name: &str) -> Result<(), String> {
    // Check if table exists
    match client.describe_table().table_name(table_name).send().await {
        Ok(_) => {
            println!("✅ Table {} already exists", table_name);
            return Ok(());
        }
        Err(e) => {
            let error_str = e.to_string();
            if !error_str.contains("ResourceNotFoundException") {
                println!("⚠️  Warning checking table {}: {}, attempting to create anyway", table_name, error_str);
            }
        }
    }

    // Create table
    match client
        .create_table()
        .table_name(table_name)
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("PK")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(|e| format!("Failed to build attribute definition: {}", e))?
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("SK")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(|e| format!("Failed to build attribute definition: {}", e))?
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("PK")
                .key_type(KeyType::Hash)
                .build()
                .map_err(|e| format!("Failed to build key schema: {}", e))?
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("SK")
                .key_type(KeyType::Range)
                .build()
                .map_err(|e| format!("Failed to build key schema: {}", e))?
        )
        .billing_mode(BillingMode::PayPerRequest)
        .send()
        .await
    {
        Ok(_) => {
            println!("✅ Created table: {}", table_name);
            // Wait for table to be active
            println!("   Waiting for table to become active...");
            let mut retries = 0;
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                match client.describe_table().table_name(table_name).send().await {
                    Ok(response) => {
                        if let Some(table) = response.table() {
                            if let Some(status) = table.table_status() {
                                if status == &aws_sdk_dynamodb::types::TableStatus::Active {
                                    println!("   ✅ Table {} is now ACTIVE", table_name);
                                    break;
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
                retries += 1;
                if retries > 15 {
                    println!("   ⚠️  Timeout waiting for table to become active, proceeding anyway");
                    break;
                }
            }
            Ok(())
        }
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("ResourceInUseException") {
                println!("Table {} already exists", table_name);
                Ok(())
            } else {
                Err(format!("Failed to create table {}: {}", table_name, error_str))
            }
        }
    }
}

/// Helper to create test schema with empty classifications to test the fix
fn create_test_schema() -> Schema {
    let mut schema = Schema::new(
        "test_native_index".to_string(),
        SchemaType::Single,
        Some(datafold::schema::types::KeyConfig {
            hash_field: Some("id".to_string()),
            range_field: None,
        }),
        Some(vec!["id".to_string(), "content".to_string()]),
        None,
        None,
    );

    // Add field topologies
    schema.set_field_topology(
        "id".to_string(),
        datafold::schema::types::JsonTopology::new(datafold::schema::types::TopologyNode::Primitive {
            value: datafold::schema::types::PrimitiveType::String,
            classifications: Some(vec!["word".to_string()]),
        }),
    );

    // IMPORTANT: This field has empty classifications to test the fix
    schema.set_field_topology(
        "content".to_string(),
        datafold::schema::types::JsonTopology::new(datafold::schema::types::TopologyNode::Primitive {
            value: datafold::schema::types::PrimitiveType::String,
            classifications: Some(vec![]), // Empty classifications!
        }),
    );

    schema.compute_schema_topology_hash();
    schema
}

#[tokio::test]
#[ignore]
async fn test_dynamodb_native_indexing() {
    println!("🧪 Starting DynamoDB native indexing test...");

    // Load AWS config
    let region = std::env::var("AWS_DEFAULT_REGION")
        .unwrap_or_else(|_| "us-east-1".to_string());
    println!("🌍 Using AWS region: {}", region);
    
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_dynamodb::config::Region::new(region.clone()))
        .load()
        .await;
    let client = Client::new(&config);

    // Create test tables
    let base_table = "TestNativeIndexStorage";
    let tables = vec![
        format!("{}-main", base_table),
        format!("{}-schemas", base_table),
        format!("{}-schema_states", base_table),
        format!("{}-native_index", base_table),
    ];

    println!("📋 Creating/verifying test tables...");
    for table_name in &tables {
        create_test_table(&client, table_name).await.expect("Failed to create table");
    }

    // Create DynamoDB store
    let store = Arc::new(
        DynamoDbNamespacedStore::new(client.clone(), base_table.to_string())
            .with_user_id("test_user_indexing".to_string())
    );

    // Create DbOperationsV2
    let db_ops = Arc::new(
        DbOperationsV2::from_namespaced_store(store).await
            .expect("Failed to create DbOperationsV2 from DynamoDB store")
    );

    // Create message bus
    let message_bus = Arc::new(MessageBus::new());
    
    // Create schema manager
    let schema_manager = Arc::new(
        SchemaCore::new(db_ops.clone(), message_bus.clone()).await
            .expect("Failed to create SchemaCore")
    );

    // Create mutation manager
    let mut mutation_manager = MutationManager::new(
        db_ops.clone(),
        schema_manager.clone(),
        message_bus.clone(),
    );

    // Create and store test schema
    let schema = create_test_schema();
    let schema_name = schema.name.clone();

    println!("📋 Storing test schema: {}", schema_name);
    db_ops.store_schema(&schema_name, &schema).await.expect("Failed to store schema");
    
    let schema_json = serde_json::to_string(&schema).expect("Failed to serialize schema");
    schema_manager.load_schema_from_json(&schema_json).await.expect("Failed to load schema");

    // Create mutation with content to index
    println!("\n🧪 Executing mutation with searchable content...");
    let mutation = Mutation::new(
        schema_name.clone(),
        {
            let mut map = std::collections::HashMap::new();
            map.insert("id".to_string(), json!("doc1"));
            map.insert("content".to_string(), json!("The quick brown fox jumps over the lazy dog"));
            map
        },
        KeyValue {
            hash: Some("doc1".to_string()),
            range: None,
        },
        String::new(),
        0,
        MutationType::Create,
    );

    // Execute mutation
    let mutation_ids = mutation_manager.write_mutations_batch_async(vec![mutation]).await
        .expect("Failed to execute mutation");
    assert_eq!(mutation_ids.len(), 1);
    println!("✅ Mutation executed successfully");

    // Wait a bit for eventual consistency / async indexing if any (though native index manager is called synchronously in mutation manager)
    // But DynamoDB writes might take a moment to be visible
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify indexing
    println!("\n🧪 Verifying native index search...");
    
    // Search for "fox"
    let results = db_ops.native_index_manager().unwrap()
        .search_word_async("fox").await
        .expect("Search failed");
    
    println!("Found {} results for 'fox'", results.len());
    assert!(!results.is_empty(), "Should find 'fox' in index");
    assert_eq!(results[0].key_value, KeyValue::new(Some("doc1".to_string()), None));

    // Search for "lazy"
    let results = db_ops.native_index_manager().unwrap()
        .search_word_async("lazy").await
        .expect("Search failed");
    
    println!("Found {} results for 'lazy'", results.len());
    assert!(!results.is_empty(), "Should find 'lazy' in index");

    println!("\n✅ DynamoDB native indexing test passed!");
}
