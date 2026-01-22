#![cfg(feature = "aws-backend")]
use datafold::datafold_node::config::DatabaseConfig;
use datafold::datafold_node::{DataFoldNode, NodeConfig};
use datafold::schema::types::Mutation;
use datafold::storage::{DynamoDbConfig, ExplicitTables};
use serde_json::json;
use std::collections::HashMap;

mod common;
use common::create_test_mutation;

// Mock schema definition
fn get_test_schema() -> (String, String) {
    let schema_name = "PerformanceTestSchema".to_string();
    let schema_json = r#"{
        "name": "PerformanceTestSchema",
        "schema_type": "Single",
        "fields": ["id", "content"],
        "field_topologies": {
            "id": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
            "content": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}}
        },
        "key": { "hash_field": "id" }
    }"#.to_string();
    (schema_name, schema_json)
}

fn generate_mutations(count: usize) -> Vec<Mutation> {
    let mut mutations = Vec::with_capacity(count);
    let (schema_name, schema_json_str) = get_test_schema();
    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_json_str).expect("Failed to parse schema");

    for i in 0..count {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), json!(format!("item_{}", i)));
        fields.insert(
            "content".to_string(),
            json!(format!("This is content for item {}", i)),
        );

        let mutation_json = json!({
            "schema_name": schema_name,
            "fields_and_values": fields,
            "mutation_type": "Create",
            "pub_key": "test_pub_key"
        });

        let mutation = create_test_mutation(&schema_value, mutation_json);
        mutations.push(mutation);
    }
    mutations
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_dynamodb_mutation_performance() {
    // 1. Setup Configuration for DynamoDB
    let table_name = "DataFoldStorage"; // Using the same table name as run script
    let region = "us-west-2";

    println!("=== DynamoDB Mutation Performance Test ===");
    println!("Target Table: {}", table_name);
    println!("Target Region: {}", region);

    // Initialize AWS defaults (the node relies on env/credentials)
    // We don't need to manually pass AWS config to NodeConfig constructor,
    // as it builds its own client.

    let tables = ExplicitTables {
        main: format!("{}-main", table_name),
        metadata: format!("{}-metadata", table_name),
        permissions: format!("{}-node_id_schema_permissions", table_name),
        transforms: format!("{}-transforms", table_name),
        orchestrator: format!("{}-orchestrator_state", table_name),
        schema_states: format!("{}-schema_states", table_name),
        schemas: format!("{}-schemas", table_name),
        public_keys: format!("{}-public_keys", table_name),
        transform_queue: format!("{}-transform_queue_tree", table_name),
        native_index: format!("{}-native_index", table_name),
        process: format!("{}-process", table_name),
        logs: format!("{}-logs", table_name),
    };

    // Create Node Config
    let config = NodeConfig {
        database: DatabaseConfig::DynamoDb(DynamoDbConfig {
            region: region.to_string(),
            tables,
            auto_create: true,
            user_id: None,
            file_storage_bucket: None,
        }),
        default_trust_distance: 1,
        network_listen_address: "/ip4/127.0.0.1/tcp/0".to_string(),
        security_config: Default::default(),
        schema_service_url: Some("test://mock".to_string()), // Use test schema service to avoid needing running service
        public_key: Some(
            datafold::security::Ed25519KeyPair::generate()
                .unwrap()
                .public_key_base64(),
        ),
        private_key: Some(
            datafold::security::Ed25519KeyPair::generate()
                .unwrap()
                .secret_key_base64(),
        ),
    };

    // 2. Initialize Node
    println!("Initializing DynamoDB Node...");
    let node = DataFoldNode::new(config.clone())
        .await
        .expect("Failed to initialize node");

    // 3. Load Schema
    // In test environment, we can manually load schema using internal DB access or schema service?
    // DataFoldNode doesn't expose schema_client?
    // We use get_fold_db() pattern from other tests

    let (schema_name, schema_json) = get_test_schema();
    println!("Loading schema: {}", schema_name);

    {
        let mut db_guard = node.get_fold_db().await.expect("Failed to get DB lock");
        db_guard
            .load_schema_from_json(&schema_json)
            .await
            .expect("Failed to load schema");
        db_guard
            .schema_manager()
            .approve(&schema_name)
            .await
            .expect("Failed to approve schema");
    }

    // 4. Test Execution
    // Start with small batch
    let batch_sizes = vec![10, 50, 100];

    for size in batch_sizes {
        println!("\n--- Testing Batch Size: {} ---", size);
        let mutations = generate_mutations(size);

        // Ensure mutations have unique IDs per run if we want to avoid overwrites (though overwrites measure performance too)

        let start = std::time::Instant::now();
        match node.mutate_batch(mutations).await {
            Ok(_) => {
                let duration = start.elapsed();
                println!(
                    "✅ Success: {} items in {:.2}s",
                    size,
                    duration.as_secs_f64()
                );
                println!(
                    "   Average: {:.2}ms per item",
                    duration.as_millis() as f64 / size as f64
                );
                println!(
                    "   Throughput: {:.2} items/sec",
                    size as f64 / duration.as_secs_f64()
                );
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
            }
        }
    }
}
