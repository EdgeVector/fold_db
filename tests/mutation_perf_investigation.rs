use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

use fold_db::fold_node::{FoldNode, NodeConfig};
use fold_db::ingestion::mutation_generator;

mod common;
use common::create_test_mutation;

// Mock DynamoDB client if needed, or use local if feature not enabled
#[cfg(feature = "aws-backend")]
#[tokio::test(flavor = "multi_thread")]
async fn test_mutation_performance_investigation() {
    println!("{}", "=".repeat(80));
    println!("Mutation Performance Investigation");
    println!("{}", "=".repeat(80));

    // 1. Mutation Generation Performance
    _test_mutation_generation_performance().await;

    // 2. Local Backend Mutation Execution Performance
    _test_local_mutation_execution().await;

    // 3. DynamoDB Backend Mutation Execution Performance (Mock/Simulated)
    // We will conditionally run this if we can set up a mock or if configured
    #[cfg(feature = "aws-backend")]
    test_dynamodb_mutation_execution().await;
}

async fn _test_mutation_generation_performance() {
    println!("\n--- Phase 1: Mutation Generation Performance ---");

    let schema_name = "PerformanceTestSchema";

    // Prepare sample data (1000 items)
    let count = 1000;
    println!("Generating mutations for {} items...", count);

    let mut all_data = Vec::new();
    for i in 0..count {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), json!(format!("id_{}", i)));
        fields.insert(
            "content".to_string(),
            json!(format!("content for item {}", i)),
        );
        fields.insert("value".to_string(), json!(i));

        // Simulating some complexity
        fields.insert(
            "metadata".to_string(),
            json!({
                "timestamp": 1234567890,
                "source": "test",
                "tags": ["a", "b", "c"]
            }),
        );

        all_data.push(fields);
    }

    let mut mappers = HashMap::new();
    // Assuming simple mapping where fields map to themselves if not specified,
    // or we can leave empty for default behavior if MutationGenerator supports it.
    // Correct mapping: JSON field -> Schema field
    mappers.insert("id".to_string(), "PerformanceTestSchema.id".to_string());

    let start = Instant::now();

    let mut total_mutations = 0;
    for fields in &all_data {
        let keys_values = HashMap::from([(
            "id".to_string(),
            fields.get("id").unwrap().as_str().unwrap().to_string(),
        )]);

        let mutations = mutation_generator::generate_mutations(
                schema_name,
                &keys_values,
                fields,
                &mappers,
                0,
                "test_pub_key".to_string(),
                None,
            )
            .expect("Failed to generate mutations");

        total_mutations += mutations.len();
    }

    let duration = start.elapsed();
    println!(
        "Generated {} mutations in {:.2?}",
        total_mutations, duration
    );
    println!(
        "Average time per item: {:.4}ms",
        duration.as_millis() as f64 / count as f64
    );

    if duration.as_secs_f64() > 1.0 {
        println!("⚠️  Mutation generation is SLOW (> 1s)");
    } else {
        println!("✅ Mutation generation is FAST");
    }
}

async fn _test_local_mutation_execution() {
    println!("\n--- Phase 2: Local Mutation Execution Performance ---");

    // Setup temp local DB
    let temp_dir =
        std::env::temp_dir().join(format!("fold_db_perf_local_{}", uuid::Uuid::new_v4()));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let config = NodeConfig::new(temp_dir.clone())
        .with_schema_service_url("test://mock")
        .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());

    let node = FoldNode::new(config)
        .await
        .expect("Failed to create node");

    // Load schema
    let schema_json = r#"{
        "name": "PerfSchema",
        "schema_type": "Single",
        "fields": ["id", "content"],
        "field_topologies": {
            "id": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}},
            "content": {"root": {"type": "Primitive", "value": "String", "classifications": ["word"]}}
        },
        "key": { "hash_field": "id" }
    }"#;

    {
        let mut db = node.get_fold_db().await.expect("Failed to get DB");
        db.load_schema_from_json(schema_json)
            .await
            .expect("Failed to load schema");
        db.schema_manager()
            .approve("PerfSchema")
            .await
            .expect("Failed to approve schema");
    }

    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("Failed to parse schema");

    // Prepare mutations
    let count = 100;
    let mut mutations = Vec::new();
    for i in 0..count {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), json!(format!("local_{}", i)));
        fields.insert("content".to_string(), json!("some content"));

        let mutation_json = json!({
            "schema_name": "PerfSchema",
            "fields_and_values": fields,
            "mutation_type": "Create",
            // The original used "sig_{i}" as pub_key/signature?
            // create_test_mutation uses "pub_key" field in JSON.
            // But wait, create_test_mutation sets pub_key from json["pub_key"] or defaults to "default_key".
            // The original code passed `format!("sig_{}", i)` as 4th arg (pub_key).
            "pub_key": format!("sig_{}", i)
        });

        let mutation = create_test_mutation(&schema_value, mutation_json);
        mutations.push(mutation);
    }

    println!("Executing {} mutations (Local)...", count);
    let start = Instant::now();

    let _ids = node
        .mutate_batch(mutations)
        .await
        .expect("Failed to mutate");

    let duration = start.elapsed();
    println!("Executed {} mutations in {:.2?}", count, duration);
    println!(
        "Average time per mutation: {:.4}ms",
        duration.as_millis() as f64 / count as f64
    );

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[cfg(feature = "aws-backend")]
async fn test_dynamodb_mutation_execution() {
    println!("\n--- Phase 3: DynamoDB Mutation Execution Performance ---");

    // NOTE: This usually requires LocalStack or real creds.
    // If we are just unit testing logic, we might mock it.
    // For this investigation, if the user has an environment that supports it, we try.
    // Otherwise we might skip or fail gracefully.

    if std::env::var("AWS_ACCESS_KEY_ID").is_err() {
        println!("⚠️  Skipping DynamoDB test: AWS credentials not found");
        return;
    }

    // Similar setup but with DynamoDB config
    // ... implementation ...
    println!("(DynamoDB test placeholder - requires env setup)");
}
