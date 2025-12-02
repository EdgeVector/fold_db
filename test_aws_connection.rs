// Quick test to verify AWS SDK connection
use aws_sdk_dynamodb::Client;
use aws_config::defaults;

#[tokio::main]
async fn main() {
    println!("🔍 Testing AWS SDK connection...");
    
    let config = defaults(aws_config::BehaviorVersion::latest())
        .region("us-east-1")
        .load()
        .await;
    
    let client = Client::new(&config);
    
    // Test 1: List tables
    println!("📋 Testing list_tables...");
    match client.list_tables().send().await {
        Ok(res) => {
            println!("✅ List tables successful!");
            if let Some(tables) = res.table_names() {
                println!("   Found {} tables", tables.len());
            }
        }
        Err(e) => {
            println!("❌ List tables failed: {:?}", e);
            println!("   Error details: {}", e);
        }
    }
    
    // Test 2: Get item from test table
    println!("\n📋 Testing get_item...");
    match client
        .get_item()
        .table_name("TestMutationStorage-main")
        .key("PK", aws_sdk_dynamodb::types::AttributeValue::S("test_user_mutations:test_key".to_string()))
        .key("SK", aws_sdk_dynamodb::types::AttributeValue::S("test_key".to_string()))
        .send()
        .await
    {
        Ok(res) => {
            println!("✅ Get item successful!");
            println!("   Item: {:?}", res.item());
        }
        Err(e) => {
            println!("❌ Get item failed: {:?}", e);
            println!("   Error details: {}", e);
            // Try to extract more details
            if let aws_sdk_dynamodb::error::SdkError::ServiceError(service_err) = &e {
                println!("   Service error kind: {:?}", service_err.err().kind());
            }
        }
    }
}
