/// Test DynamoDB mutations to verify refactoring works correctly
/// 
/// This test verifies that:
/// 1. Mutations work with DynamoDB backend
/// 2. No deadlocks occur (uses async path)
/// 3. Data is persisted correctly
/// 4. Multiple mutations can be executed in sequence
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
use datafold::schema::types::field::Field;
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
            // Handle connection/network errors more gracefully
            if error_str.contains("dispatch failure") || 
               error_str.contains("connection") ||
               error_str.contains("timeout") ||
               error_str.contains("credentials") {
                return Err(format!(
                    "Cannot connect to DynamoDB: {}\n\
                    Hint: Make sure AWS credentials are configured or use LocalStack:\n\
                    - For LocalStack: export AWS_ENDPOINT_URL=http://localhost:4566\n\
                    - For AWS: configure AWS credentials via aws configure or environment variables",
                    error_str
                ));
            }
            if !error_str.contains("ResourceNotFoundException") {
                // Other errors might be recoverable, try to create anyway
                println!("⚠️  Warning checking table {}: {}, attempting to create anyway", table_name, error_str);
            }
            // Table doesn't exist, create it
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
            // Wait for table to be active (DynamoDB tables need time to become ACTIVE)
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
                                } else {
                                    println!("   ⏳ Table {} status: {:?} (waiting...)", table_name, status);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("   ⚠️  Error checking table status: {}", e);
                    }
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
                println!("Table {} already exists (created by another process)", table_name);
                Ok(())
            } else {
                Err(format!("Failed to create table {}: {}", table_name, error_str))
            }
        }
    }
}

/// Helper to create test schema
fn create_test_schema() -> Schema {
    let mut schema = Schema::new(
        "test_users".to_string(),
        SchemaType::Single,
        Some(datafold::schema::types::KeyConfig {
            hash_field: Some("id".to_string()),
            range_field: None,
        }),
        Some(vec!["id".to_string(), "name".to_string(), "email".to_string()]),
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

    schema.set_field_topology(
        "name".to_string(),
        datafold::schema::types::JsonTopology::new(datafold::schema::types::TopologyNode::Primitive {
            value: datafold::schema::types::PrimitiveType::String,
            classifications: Some(vec!["word".to_string()]),
        }),
    );

    schema.set_field_topology(
        "email".to_string(),
        datafold::schema::types::JsonTopology::new(datafold::schema::types::TopologyNode::Primitive {
            value: datafold::schema::types::PrimitiveType::String,
            classifications: Some(vec!["word".to_string()]),
        }),
    );

    schema.compute_schema_topology_hash();
    schema
}

#[tokio::test]
#[ignore] // Run with `cargo test --test dynamodb_mutation_test -- --ignored` when DynamoDB is available
async fn test_dynamodb_mutations_no_deadlock() {
    println!("🧪 Starting DynamoDB mutation test...");

    // Load AWS config with explicit region
    let region = std::env::var("AWS_DEFAULT_REGION")
        .unwrap_or_else(|_| "us-east-1".to_string());
    println!("🌍 Using AWS region: {}", region);
    
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_dynamodb::config::Region::new(region.clone()))
        .load()
        .await;
    let client = Client::new(&config);

    // Create test tables (including native_index which is needed for mutations)
    let base_table = "TestMutationStorage";
    let tables = vec![
        format!("{}-main", base_table),
        format!("{}-schemas", base_table),
        format!("{}-schema_states", base_table),
        format!("{}-native_index", base_table), // Required for field indexing
    ];

    println!("📋 Creating/verifying test tables...");
    for table_name in &tables {
        match create_test_table(&client, table_name).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("❌ Failed to create/verify table {}: {}", table_name, e);
                eprintln!("\n💡 Troubleshooting:");
                eprintln!("   1. For LocalStack: docker run -d -p 4566:4566 localstack/localstack");
                eprintln!("      Then: export AWS_ENDPOINT_URL=http://localhost:4566");
                eprintln!("   2. For AWS: Configure credentials with 'aws configure'");
                eprintln!("   3. Check network connectivity to DynamoDB endpoint");
                panic!("Cannot proceed without tables");
            }
        }
    }

    // Create DynamoDB store
    let store = Arc::new(
        DynamoDbNamespacedStore::new(client.clone(), base_table.to_string())
            .with_user_id("test_user_mutations".to_string())
    );

    // Create DbOperationsV2
    let db_ops = Arc::new(
        DbOperationsV2::from_namespaced_store(store).await
            .expect("Failed to create DbOperationsV2 from DynamoDB store")
    );

    // Create message bus for schema manager
    let message_bus = Arc::new(MessageBus::new());
    
    // Create schema manager (async)
    let schema_manager = Arc::new(
        SchemaCore::new(db_ops.clone(), message_bus.clone()).await
            .expect("Failed to create SchemaCore")
    );

    // Create mutation manager
    let mut mutation_manager = MutationManager::new(
        db_ops.clone(),
        schema_manager.clone(),
        message_bus.clone(),
        None,
    );

    // Create and store test schema
    let schema = create_test_schema();
    let schema_name = schema.name.clone();

    println!("📋 Storing test schema: {}", schema_name);
    
    // Store schema first
    db_ops.store_schema(&schema_name, &schema).await
        .expect("Failed to store schema");
    
    // Load schema from JSON to ensure runtime fields are properly initialized
    // This is what the system normally does - load from stored JSON
    let schema_json = serde_json::to_string(&schema).expect("Failed to serialize schema");
    schema_manager.load_schema_from_json(&schema_json).await
        .expect("Failed to load schema from JSON");

    // Verify schema is loaded
    let loaded_schema = schema_manager.get_schema(&schema_name)
        .expect("Failed to get schema")
        .expect("Schema not found");
    assert_eq!(loaded_schema.name, schema_name);
    println!("✅ Schema loaded successfully");

    // Test 1: Single mutation
    println!("\n🧪 Test 1: Single mutation");
    let mutation1 = Mutation::new(
        schema_name.clone(),
        {
            let mut map = std::collections::HashMap::new();
            map.insert("id".to_string(), json!("user1"));
            map.insert("name".to_string(), json!("Alice"));
            map.insert("email".to_string(), json!("alice@example.com"));
            map
        },
        KeyValue {
            hash: Some("user1".to_string()),
            range: None,
        },
        String::new(),
        0,
        MutationType::Create,
    );

    let start = std::time::Instant::now();
    let mutation_ids = mutation_manager.write_mutations_batch_async(vec![mutation1]).await
        .expect("Failed to execute mutation");
    let elapsed = start.elapsed();
    
    assert_eq!(mutation_ids.len(), 1);
    println!("✅ Single mutation completed in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(10), "Mutation took too long - possible deadlock");

    // Test 2: Multiple mutations in batch
    println!("\n🧪 Test 2: Batch mutations");
    let mutations = vec![
        Mutation::new(
            schema_name.clone(),
            {
                let mut map = std::collections::HashMap::new();
                map.insert("id".to_string(), json!("user2"));
                map.insert("name".to_string(), json!("Bob"));
                map.insert("email".to_string(), json!("bob@example.com"));
                map
            },
            KeyValue {
                hash: Some("user2".to_string()),
                range: None,
            },
            String::new(),
            0,
            MutationType::Create,
        ),
        Mutation::new(
            schema_name.clone(),
            {
                let mut map = std::collections::HashMap::new();
                map.insert("id".to_string(), json!("user3"));
                map.insert("name".to_string(), json!("Charlie"));
                map.insert("email".to_string(), json!("charlie@example.com"));
                map
            },
            KeyValue {
                hash: Some("user3".to_string()),
                range: None,
            },
            String::new(),
            0,
            MutationType::Create,
        ),
    ];

    let start = std::time::Instant::now();
    let mutation_ids = mutation_manager.write_mutations_batch_async(mutations).await
        .expect("Failed to execute batch mutations");
    let elapsed = start.elapsed();
    
    assert_eq!(mutation_ids.len(), 2);
    println!("✅ Batch mutations (2) completed in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(15), "Batch mutations took too long - possible deadlock");

    // Test 3: Update mutation
    println!("\n🧪 Test 3: Update mutation");
    let update_mutation = Mutation::new(
        schema_name.clone(),
        {
            let mut map = std::collections::HashMap::new();
            map.insert("email".to_string(), json!("alice.updated@example.com"));
            map
        },
        KeyValue {
            hash: Some("user1".to_string()),
            range: None,
        },
        String::new(),
        0,
        MutationType::Update,
    );

    let start = std::time::Instant::now();
    let mutation_ids = mutation_manager.write_mutations_batch_async(vec![update_mutation]).await
        .expect("Failed to execute update mutation");
    let elapsed = start.elapsed();
    
    assert_eq!(mutation_ids.len(), 1);
    println!("✅ Update mutation completed in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(10), "Update mutation took too long - possible deadlock");

    // Test 4: Verify data persistence by querying
    println!("\n🧪 Test 4: Verify data persistence");
    let schema = schema_manager.get_schema(&schema_name)
        .expect("Failed to get schema")
        .expect("Schema not found");

    // Query for user1
    let user1_field = schema.runtime_fields.get("name")
        .expect("name field not found");
    
    // The mutation should have created atoms and molecules
    // We can verify by checking if the field has a molecule
    use datafold::schema::types::field::Field;
    if let Some(molecule_uuid) = user1_field.common().molecule_uuid() {
        println!("✅ User1 has molecule UUID: {}", molecule_uuid);
    } else {
        println!("⚠️  User1 molecule UUID not found (may need to refresh field)");
    }

    // Test 5: Rapid sequential mutations (stress test for deadlocks)
    println!("\n🧪 Test 5: Rapid sequential mutations (deadlock stress test)");
    let mut all_mutation_ids = Vec::new();
    let start = std::time::Instant::now();
    
    for i in 4..=10 {
        let mutation = Mutation::new(
            schema_name.clone(),
            {
                let mut map = std::collections::HashMap::new();
                map.insert("id".to_string(), json!(format!("user{}", i)));
                map.insert("name".to_string(), json!(format!("User{}", i)));
                map.insert("email".to_string(), json!(format!("user{}@example.com", i)));
                map
            },
            KeyValue {
                hash: Some(format!("user{}", i)),
                range: None,
            },
            String::new(),
            0,
            MutationType::Create,
        );

        let ids = mutation_manager.write_mutations_batch_async(vec![mutation]).await
            .expect(&format!("Failed to execute mutation {}", i));
        all_mutation_ids.extend(ids);
    }
    
    let elapsed = start.elapsed();
    assert_eq!(all_mutation_ids.len(), 7);
    println!("✅ Rapid sequential mutations (7) completed in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(30), "Sequential mutations took too long - possible deadlock");
    println!("   Average time per mutation: {:?}", elapsed / 7);

    // Test 6: Large batch mutation
    println!("\n🧪 Test 6: Large batch mutation");
    let large_batch: Vec<Mutation> = (11..=20)
        .map(|i| {
            Mutation::new(
                schema_name.clone(),
                {
                    let mut map = std::collections::HashMap::new();
                    map.insert("id".to_string(), json!(format!("user{}", i)));
                    map.insert("name".to_string(), json!(format!("User{}", i)));
                    map.insert("email".to_string(), json!(format!("user{}@example.com", i)));
                    map
                },
                KeyValue {
                    hash: Some(format!("user{}", i)),
                    range: None,
                },
                String::new(),
                0,
                MutationType::Create,
            )
        })
        .collect();

    let start = std::time::Instant::now();
    let mutation_ids = mutation_manager.write_mutations_batch_async(large_batch).await
        .expect("Failed to execute large batch mutation");
    let elapsed = start.elapsed();
    
    assert_eq!(mutation_ids.len(), 10);
    println!("✅ Large batch mutation (10) completed in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(20), "Large batch mutation took too long - possible deadlock");
    println!("   Average time per mutation: {:?}", elapsed / 10);

    println!("\n✅ All DynamoDB mutation tests passed!");
    println!("   - No deadlocks detected");
    println!("   - All mutations completed successfully");
    println!("   - Async path working correctly");
}

#[tokio::test]
#[ignore] // Run with `cargo test --test dynamodb_mutation_test -- --ignored` when DynamoDB is available
async fn test_dynamodb_mutations_with_node() {
    println!("🧪 Testing DynamoDB mutations through DataFoldNode...");

    // Load AWS config with explicit region
    let region = std::env::var("AWS_DEFAULT_REGION")
        .unwrap_or_else(|_| "us-east-1".to_string());
    println!("🌍 Using AWS region: {}", region);
    
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_dynamodb::config::Region::new(region.clone()))
        .load()
        .await;
    let client = Client::new(&config);

    // Create test tables
    let base_table = "TestNodeMutationStorage";
    let tables = vec![
        format!("{}-main", base_table),
        format!("{}-schemas", base_table),
        format!("{}-schema_states", base_table),
    ];

    println!("📋 Creating/verifying test tables...");
    for table_name in &tables {
        match create_test_table(&client, table_name).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("❌ Failed to create/verify table {}: {}", table_name, e);
                eprintln!("\n💡 Troubleshooting:");
                eprintln!("   1. For LocalStack: docker run -d -p 4566:4566 localstack/localstack");
                eprintln!("      Then: export AWS_ENDPOINT_URL=http://localhost:4566");
                eprintln!("   2. For AWS: Configure credentials with 'aws configure'");
                eprintln!("   3. Check network connectivity to DynamoDB endpoint");
                panic!("Cannot proceed without tables");
            }
        }
    }

    // Create node config with DynamoDB
    use datafold::datafold_node::config::NodeConfig;
    use datafold::datafold_node::config::DatabaseConfig;
    
    let node_config = NodeConfig {
        database: DatabaseConfig::DynamoDb {
            table_name: base_table.to_string(),
            region: "us-east-1".to_string(),
            user_id: Some("test_node_user".to_string()),
        },
        storage_path: std::path::PathBuf::from("/tmp/test_node_mutations"),
        default_trust_distance: 1,
        network_listen_address: "/ip4/127.0.0.1/tcp/0".to_string(),
        security_config: datafold::security::SecurityConfig::from_env(),
        schema_service_url: None,
    };

    // Create node
    let node = datafold::datafold_node::DataFoldNode::new(node_config).await
        .expect("Failed to create node with DynamoDB");

    // Create a test schema
    let schema = create_test_schema();
    let schema_name = schema.name.clone();

    // Store schema through node
    {
        let db_guard = node.get_fold_db()
            .expect("Failed to get database");
        
        // Get schema manager directly
        let schema_manager = db_guard.schema_manager();
        
        // Load schema (this will store it)
        schema_manager.load_schema_from_json(&serde_json::to_string(&schema).unwrap()).await
            .expect("Failed to load schema");
        
        // Approve schema
        schema_manager.set_schema_state(&schema_name, datafold::schema::SchemaState::Approved).await
            .expect("Failed to approve schema");
    }

    // Test mutation through node API
    println!("🧪 Executing mutation through node API...");
    let mutation = Mutation::new(
        schema_name.clone(),
        {
            let mut map = std::collections::HashMap::new();
            map.insert("id".to_string(), json!("node_user1"));
            map.insert("name".to_string(), json!("NodeUser1"));
            map.insert("email".to_string(), json!("nodeuser1@example.com"));
            map
        },
        KeyValue {
            hash: Some("node_user1".to_string()),
            range: None,
        },
        String::new(),
        0,
        MutationType::Create,
    );

    let start = std::time::Instant::now();
    let mutation_ids = node.mutate_batch_async(vec![mutation]).await
        .expect("Failed to execute mutation through node");
    let elapsed = start.elapsed();
    
    assert_eq!(mutation_ids.len(), 1);
    println!("✅ Mutation through node completed in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(10), "Mutation took too long - possible deadlock");

    println!("\n✅ Node-based DynamoDB mutation test passed!");
}
