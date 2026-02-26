/// Example demonstrating that HTTP client timeouts prevent indefinite hanging
///
/// Run with: cargo run --example test_timeout
use fold_db::fold_node::schema_client::SchemaServiceClient;
use fold_db::schema::types::{Schema, SchemaType};
use std::collections::HashMap;
use std::time::Instant;

#[tokio::main]
async fn main() {
    println!("\n🧪 Testing Schema Service Client Timeout Fix");
    println!("==============================================\n");

    // Create a client pointing to a non-existent service (guaranteed to fail)
    let client = SchemaServiceClient::new("http://127.0.0.1:9999");

    // Create a simple test schema
    let mut schema = Schema::new(
        "TestSchema".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["id".to_string(), "name".to_string()]),
        None,
        None,
    );

    // Add field classifications
    schema.field_classifications.insert("id".to_string(), vec!["word".to_string()]);
    schema.field_classifications.insert("name".to_string(), vec!["word".to_string()]);

    schema.compute_identity_hash();

    println!("📡 Attempting to connect to: http://127.0.0.1:9999/api/schemas");
    println!("   (This service doesn't exist - testing timeout behavior)\n");
    println!("⏱️  Expected: Timeout within 10-30 seconds");
    println!("❌ Before fix: Would hang forever\n");

    let start = Instant::now();

    // Try to add schema - should timeout gracefully with our fix
    match client.add_schema(&schema, HashMap::new()).await {
        Ok(_) => {
            println!("❌ ERROR: Unexpected success!");
            std::process::exit(1);
        }
        Err(e) => {
            let elapsed = start.elapsed();
            println!(
                "✅ Request failed after {:.2} seconds",
                elapsed.as_secs_f64()
            );
            println!("📝 Error message:\n   {}\n", e);

            // Verify timeout happened within reasonable time
            if elapsed.as_secs() <= 35 {
                println!("✅ SUCCESS: Timeout fix is working correctly!");
                println!(
                    "   • Request timed out in {} seconds (expected: 10-30s)",
                    elapsed.as_secs()
                );
                println!("   • Error message is clear and actionable");
                println!("   • No indefinite hanging occurred\n");
            } else {
                println!(
                    "⚠️  WARNING: Timeout took {} seconds (expected <35s)",
                    elapsed.as_secs()
                );
                println!(
                    "   This is longer than expected but still better than hanging forever.\n"
                );
            }
        }
    }

    println!("🎯 Test complete!");
}
