//! Performance Test: Single vs Batch Mutations (Direct Database)
//!
//! This test directly measures database performance by calling the mutation manager
//! directly, bypassing HTTP overhead and properly measuring execution time.
//!
//! Usage:
//!     cargo test --test mutation_performance_test -- --nocapture

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use serde_json::json;

use fold_db::fold_node::{FoldNode, NodeConfig};
use fold_db::schema::types::Mutation;
// use fold_db::MutationType; - Removed as it's implied or handled by helper if we pass "Create" string
// use fold_db::schema::types::key_value::KeyValue; - Removed as helper abstracts this

mod common;
use common::create_test_mutation;

/// Performance Test: Single vs Batch Mutations (Direct Database)
///
/// This test compares the performance difference between:
/// 1. Executing multiple mutations individually (single mode)
/// 2. Executing multiple mutations in a batch (batch mode)
///
/// The test directly measures database performance by calling the mutation manager
/// directly, bypassing HTTP overhead and properly measuring execution time.
///
/// Usage:
///     cargo test test_mutation_performance_direct -- --nocapture

#[tokio::test(flavor = "multi_thread")]
async fn test_mutation_performance_direct() {
    // Wrap entire test in a 10-second timeout
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_mutation_performance_test()
    ).await;

    assert!(result.is_ok(), "Performance test timed out after 10 seconds");
}

async fn run_mutation_performance_test() {
    println!("{}", "=".repeat(80));
    println!("Mutation Performance Test: Single vs Batch (Direct DB)");
    println!("{}", "=".repeat(80));
    println!("Date: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
    println!("{}", "=".repeat(80));

    // Setup test database
    let temp_db_path = Path::new("./test_db_perf");
    if temp_db_path.exists() {
        std::fs::remove_dir_all(temp_db_path).expect("Failed to cleanup test db");
    }

    // Create node configuration
    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let config = NodeConfig::new(temp_db_path.to_path_buf())
        .with_schema_service_url("test://mock")
        .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());

    let node = FoldNode::new(config)
        .await
        .expect("Failed to create test node");

    // Load BlogPost schema
    let schema_path = Path::new("./tests/schemas_for_testing/BlogPost.json");
    let schema_json = std::fs::read_to_string(schema_path).expect("Failed to read schema file");

    {
        let mut db_guard = node.get_fold_db().await.expect("Failed to get database");
        db_guard
            .load_schema_from_json(&schema_json)
            .await
            .expect("Failed to load BlogPost schema");
    }

    // Approve the schema
    {
        let db_guard = node.get_fold_db().await.expect("Failed to get database");
        db_guard
            .schema_manager()
            .approve("BlogPost")
            .await
            .expect("Failed to approve BlogPost schema");
    }

    println!("\n{}", "=".repeat(80));
    println!("Test Configuration");
    println!("{}", "=".repeat(80));
    const NUM_MUTATIONS: usize = 10;
    println!("Number of mutations: {}", NUM_MUTATIONS);
    println!("Schema: BlogPost");
    println!();

    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_json).expect("Failed to parse schema JSON");

    // Step 1: Performance test - Single Mutations
    println!("{}", "=".repeat(80));
    println!("Phase 1: Single Mutations");
    println!("{}", "=".repeat(80));

    let single_start = Instant::now();
    let single_times = execute_single_mutations_direct(&node, NUM_MUTATIONS, &schema_value).await;
    let single_duration = single_start.elapsed();

    println!("\nSingle Mutation Results:");
    println!("  Total time: {:.2}ms", single_duration.as_millis());
    println!(
        "  Average time per mutation: {:.2}ms",
        single_duration.as_millis() as f64 / NUM_MUTATIONS as f64
    );
    println!(
        "  Min time: {:.2}ms",
        single_times.iter().min().unwrap_or(&0)
    );
    println!(
        "  Max time: {:.2}ms",
        single_times.iter().max().unwrap_or(&0)
    );

    // Short delay between tests
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Step 2: Performance test - Batch Mutations
    println!("\n{}", "=".repeat(80));
    println!("Phase 2: Batch Mutations");
    println!("{}", "=".repeat(80));

    let batch_start = Instant::now();
    execute_batch_mutations_direct(&node, NUM_MUTATIONS, &schema_value).await;
    let batch_duration = batch_start.elapsed();

    println!("\nBatch Mutation Results:");
    println!("  Total time: {:.2}ms", batch_duration.as_millis());
    println!(
        "  Average time per mutation: {:.2}ms",
        batch_duration.as_millis() as f64 / NUM_MUTATIONS as f64
    );

    // Step 3: Performance Comparison
    println!("\n{}", "=".repeat(80));
    println!("Performance Comparison");
    println!("{}", "=".repeat(80));

    let improvement_ms = single_duration.as_millis() as f64 - batch_duration.as_millis() as f64;
    let improvement_pct = (improvement_ms / single_duration.as_millis() as f64) * 100.0;

    println!(
        "Single mutations:   {:.2}ms ({:.2}ms per mutation)",
        single_duration.as_millis(),
        single_duration.as_millis() as f64 / NUM_MUTATIONS as f64
    );
    println!(
        "Batch mutations:    {:.2}ms ({:.2}ms per mutation)",
        batch_duration.as_millis(),
        batch_duration.as_millis() as f64 / NUM_MUTATIONS as f64
    );
    println!(
        "Performance gain:   {:.2}ms ({:.1}% faster)",
        improvement_ms, improvement_pct
    );
    println!();

    if improvement_pct > 5.0 {
        println!("✅ Batch mutations are {:.1}% faster", improvement_pct);
    } else if improvement_pct > 0.0 {
        println!(
            "⚠️  Batch mutations are only {:.1}% faster",
            improvement_pct
        );
    } else {
        println!(
            "⚠️  Single mutations were faster by {:.1}% (unexpected)",
            -improvement_pct
        );
    }

    // Cleanup
    std::fs::remove_dir_all(temp_db_path).ok();

    // Log the results - we've already printed the comparison above
    println!("\n✅ Performance test completed");
}

/// Execute mutations one at a time (single mode) - directly on the database
async fn execute_single_mutations_direct(
    node: &FoldNode,
    count: usize,
    schema: &serde_json::Value,
) -> Vec<u64> {
    let mut times = Vec::new();

    for i in 0..count {
        let mutation = create_blogpost_mutation(i, schema);

        let start = Instant::now();
        let result = node.mutate_batch(vec![mutation]).await;
        let duration = start.elapsed();
        times.push(duration.as_millis() as u64);

        match result {
            Ok(_) => {
                if i % 10 == 0 {
                    print!(".");
                }
            }
            Err(e) => {
                println!("\n❌ Single mutation {} failed: {:?}", i, e);
            }
        }
    }

    println!();
    times
}

/// Execute mutations in a single batch - directly on the database
async fn execute_batch_mutations_direct(
    node: &FoldNode,
    count: usize,
    schema: &serde_json::Value,
) {
    let mut mutations = Vec::new();

    for i in 0..count {
        let mutation = create_blogpost_mutation(i, schema);
        mutations.push(mutation);
    }

    println!("  Executing batch of {} mutations", count);

    match node.mutate_batch(mutations).await {
        Ok(ids) => {
            println!(
                "✅ Batch mutation completed successfully - {} mutation IDs returned",
                ids.len()
            );
        }
        Err(e) => {
            println!("❌ Batch mutation failed: {:?}", e);
        }
    }
}

/// Create a test blog post mutation
fn create_blogpost_mutation(index: usize, schema: &serde_json::Value) -> Mutation {
    let publish_date = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let title = format!("Performance Test Blog Post {}", index);
    let content = format!(
        "This is test content for performance testing. Post number {} created.",
        index
    );

    let mut fields_and_values = HashMap::new();
    fields_and_values.insert("title".to_string(), json!(title));
    fields_and_values.insert("content".to_string(), json!(content));
    fields_and_values.insert("author".to_string(), json!("Performance Tester"));
    fields_and_values.insert(
        "publish_date".to_string(),
        json!(format!("{}-{:03}", publish_date, index)),
    );
    fields_and_values.insert(
        "tags".to_string(),
        json!(["performance", "test", "benchmark"]),
    );

    let mutation_json = json!({
        "schema_name": "BlogPost",
        "fields_and_values": fields_and_values,
        "mutation_type": "Create"
    });

    create_test_mutation(schema, mutation_json)
}
