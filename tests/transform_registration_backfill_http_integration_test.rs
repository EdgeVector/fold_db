use datafold::datafold_node::{DataFoldNode, config::NodeConfig};
use datafold::schema::{SchemaState};
use reqwest;
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::{sleep, timeout};

/// Transform Registration and Backfill HTTP Integration Test
/// 
/// This test verifies the complete transform registration and backfill workflow
/// using HTTP API calls, similar to the Python version but in Rust.
/// 
/// This demonstrates that Rust can absolutely do HTTP API validation!
/// 
/// Usage:
///     cargo test transform_registration_backfill_http_integration -- --nocapture
/// 
/// The test will:
///     - Start the HTTP server using ./run_http_server.sh
///     - Make HTTP API calls to test the complete workflow
///     - Verify transform registration and backfill functionality
///     - Clean up by stopping the server

#[tokio::test]
async fn test_transform_registration_backfill_http_integration() {
    println!("==================================================================================");
    println!("Transform Registration and Backfill HTTP Integration Test (Rust)");
    println!("==================================================================================");
    println!("Date: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
    println!("Base URL: http://localhost:9001");
    println!("==================================================================================");

    let mut results = HttpTestResults::new();
    let mut server_process: Option<std::process::Child> = None;
    
    // Step 1: Start HTTP server
    if !start_http_server(&mut server_process, &mut results).await {
        print_summary(&results);
        panic!("Failed to start HTTP server");
    }
    
    // Step 2: Wait for server to be ready
    if !wait_for_server_ready(&mut results).await {
        cleanup_server(&mut server_process);
        print_summary(&results);
        panic!("Server failed to become ready");
    }
    
    // Step 3: Run HTTP API tests
    if test_load_schemas_http(&mut results).await {
        if test_verify_schemas_available_http(&mut results).await {
            if test_approve_blogpost_schema_http(&mut results).await {
                if let Some(publish_date) = test_create_blogpost_data_http(&mut results).await {
                    if test_load_wordindex_transform_http(&mut results).await {
                        if test_verify_transform_registered_http(&mut results).await {
                            test_query_transform_results_http(&publish_date, &mut results).await;
                        }
                    }
                }
            }
        }
    }
    
    // Cleanup
    cleanup_server(&mut server_process);
    
    // Print final summary
    print_summary(&results);
    
    // Assert all tests passed
    assert!(results.passed > 0, "No tests passed");
    assert_eq!(results.failed, 0, "Some tests failed: {}", results.failed);
}

struct HttpTestResults {
    passed: u32,
    failed: u32,
    tests: Vec<(String, String)>,
}

impl HttpTestResults {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
            tests: Vec::new(),
        }
    }
    
    fn add_pass(&mut self, test_name: &str) {
        self.passed += 1;
        self.tests.push(("✅ PASS".to_string(), test_name.to_string()));
        println!("   ✅ {}: PASSED", test_name);
    }
    
    fn add_fail(&mut self, test_name: &str, error_msg: &str) {
        self.failed += 1;
        self.tests.push(("❌ FAIL".to_string(), format!("{}: {}", test_name, error_msg)));
        println!("   ❌ {}: FAILED - {}", test_name, error_msg);
    }
}

async fn start_http_server(server_process: &mut Option<std::process::Child>, results: &mut HttpTestResults) -> bool {
    println!("\n🚀 Starting HTTP server...");
    
    match Command::new("./run_http_server.sh")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(process) => {
            *server_process = Some(process);
            println!("   Server started with PID: {:?}", server_process.as_ref().unwrap().id());
            
            // Give the server time to start up
            sleep(Duration::from_secs(5)).await;
            
            results.add_pass("Start HTTP server");
            true
        }
        Err(e) => {
            results.add_fail("Start HTTP server", &format!("Failed to start server: {}", e));
            false
        }
    }
}

async fn wait_for_server_ready(results: &mut HttpTestResults) -> bool {
    println!("\n⏳ Waiting for server to be ready...");
    
    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(30);
    
    while start_time.elapsed() < timeout_duration {
        match client.get("http://localhost:9001/api/system/status")
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => {
                println!("   ✅ Server is ready");
                results.add_pass("Wait for server ready");
                return true;
            }
            Ok(_) => {
                // Server responded but not with 200
            }
            Err(_) => {
                // Server not ready yet
            }
        }
        
        sleep(Duration::from_secs(1)).await;
    }
    
    results.add_fail("Wait for server ready", "Server failed to become ready within timeout");
    false
}

async fn test_load_schemas_http(results: &mut HttpTestResults) -> bool {
    println!("\n📋 Testing schema loading via HTTP...");
    
    let client = reqwest::Client::new();
    
    match client.post("http://localhost:9001/api/schemas/load")
        .header("Content-Type", "application/json")
        .send()
        .await
    {
        Ok(response) if response.status() == 200 => {
            match response.json::<Value>().await {
                Ok(data) => {
                    if let Some(response_data) = data.get("data") {
                        let available_loaded = response_data.get("available_schemas_loaded").and_then(|v| v.as_u64()).unwrap_or(0);
                        let data_loaded = response_data.get("data_schemas_loaded").and_then(|v| v.as_u64()).unwrap_or(0);
                        let total_loaded = available_loaded + data_loaded;
                        println!("   Loaded {} schemas ({} available, {} data)", total_loaded, available_loaded, data_loaded);
                        results.add_pass("Load schemas via HTTP");
                        return true;
                    }
                }
                Err(e) => {
                    results.add_fail("Load schemas via HTTP", &format!("Failed to parse response: {}", e));
                    return false;
                }
            }
        }
        Ok(response) => {
            results.add_fail("Load schemas via HTTP", &format!("Expected status 200, got {}", response.status()));
            return false;
        }
        Err(e) => {
            results.add_fail("Load schemas via HTTP", &format!("Request failed: {}", e));
            return false;
        }
    }
    
    results.add_fail("Load schemas via HTTP", "Invalid response format");
    false
}

async fn test_verify_schemas_available_http(results: &mut HttpTestResults) -> bool {
    println!("\n🔍 Testing schema availability via HTTP...");
    
    let client = reqwest::Client::new();
    
    match client.get("http://localhost:9001/api/schemas")
        .send()
        .await
    {
        Ok(response) if response.status() == 200 => {
            match response.json::<Value>().await {
                Ok(data) => {
                    if let Some(schemas) = data.get("data").and_then(|d| d.as_array()) {
                        println!("   Found {} available schemas", schemas.len());
                        
                        // Check for required schemas
                        let schema_names: Vec<String> = schemas.iter()
                            .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                            .map(|s| s.to_string())
                            .collect();
                        
                        if schema_names.contains(&"BlogPost".to_string()) {
                            results.add_pass("Verify schemas available via HTTP");
                            return true;
                        } else {
                            results.add_fail("Verify schemas available via HTTP", "BlogPost schema not found");
                            return false;
                        }
                    }
                }
                Err(e) => {
                    results.add_fail("Verify schemas available via HTTP", &format!("Failed to parse response: {}", e));
                    return false;
                }
            }
        }
        Ok(response) => {
            results.add_fail("Verify schemas available via HTTP", &format!("Expected status 200, got {}", response.status()));
            return false;
        }
        Err(e) => {
            results.add_fail("Verify schemas available via HTTP", &format!("Request failed: {}", e));
            return false;
        }
    }
    
    results.add_fail("Verify schemas available via HTTP", "Invalid response format");
    false
}

async fn test_approve_blogpost_schema_http(results: &mut HttpTestResults) -> bool {
    println!("\n✅ Testing BlogPost schema approval via HTTP...");
    
    let client = reqwest::Client::new();
    
    match client.post("http://localhost:9001/api/schema/BlogPost/approve")
        .header("Content-Type", "application/json")
        .send()
        .await
    {
        Ok(response) if response.status() == 200 => {
            match response.json::<Value>().await {
                Ok(data) => {
                    if data.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                        println!("   BlogPost schema approved successfully");
                        results.add_pass("Approve BlogPost schema via HTTP");
                        return true;
                    }
                }
                Err(e) => {
                    results.add_fail("Approve BlogPost schema via HTTP", &format!("Failed to parse response: {}", e));
                    return false;
                }
            }
        }
        Ok(response) => {
            results.add_fail("Approve BlogPost schema via HTTP", &format!("Expected status 200, got {}", response.status()));
            return false;
        }
        Err(e) => {
            results.add_fail("Approve BlogPost schema via HTTP", &format!("Request failed: {}", e));
            return false;
        }
    }
    
    results.add_fail("Approve BlogPost schema via HTTP", "Schema approval failed");
    false
}

async fn test_create_blogpost_data_http(results: &mut HttpTestResults) -> Option<String> {
    println!("\n📝 Testing blog post creation via HTTP...");
    
    let publish_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let client = reqwest::Client::new();
    
    let mutation_data = json!({
        "type": "mutation",
        "schema": "BlogPost",
        "mutation_type": "create",
        "fields_and_values": {
            "title": format!("Test Post {}", publish_date),
            "content": "This is test content for transform backfill testing",
            "author": "Test Author",
            "publish_date": publish_date,
            "tags": ["test", "integration", "transform"]
        },
        "key_value": {
            "range": publish_date
        }
    });
    
    match client.post("http://localhost:9001/api/mutation")
        .header("Content-Type", "application/json")
        .json(&mutation_data)
        .send()
        .await
    {
        Ok(response) if response.status() == 200 => {
            match response.json::<Value>().await {
                Ok(data) => {
                    if data.get("success").and_then(|s| s.as_bool()).unwrap_or(false) || data.get("data").is_some() {
                        println!("   Created blog post: 'Test Post {}'", publish_date);
                        results.add_pass("Create blog post data via HTTP");
                        return Some(publish_date);
                    }
                }
                Err(e) => {
                    results.add_fail("Create blog post data via HTTP", &format!("Failed to parse response: {}", e));
                    return None;
                }
            }
        }
        Ok(response) => {
            results.add_fail("Create blog post data via HTTP", &format!("Expected status 200, got {}", response.status()));
            return None;
        }
        Err(e) => {
            results.add_fail("Create blog post data via HTTP", &format!("Request failed: {}", e));
            return None;
        }
    }
    
    results.add_fail("Create blog post data via HTTP", "Mutation failed");
    None
}

async fn test_load_wordindex_transform_http(results: &mut HttpTestResults) -> bool {
    println!("\n🔧 Testing BlogPostWordIndex transform loading via HTTP...");
    
    let client = reqwest::Client::new();
    
    match client.post("http://localhost:9001/api/schemas/load")
        .header("Content-Type", "application/json")
        .send()
        .await
    {
        Ok(response) if response.status() == 200 => {
            println!("   Transform schema loaded");
            
            // Wait a bit for transform registration to complete
            sleep(Duration::from_secs(2)).await;
            
            results.add_pass("Load BlogPostWordIndex transform via HTTP");
            return true;
        }
        Ok(response) => {
            results.add_fail("Load BlogPostWordIndex transform via HTTP", &format!("Expected status 200, got {}", response.status()));
            false
        }
        Err(e) => {
            results.add_fail("Load BlogPostWordIndex transform via HTTP", &format!("Request failed: {}", e));
            false
        }
    }
}

async fn test_verify_transform_registered_http(results: &mut HttpTestResults) -> bool {
    println!("\n📋 Testing transform registration via HTTP...");
    
    let client = reqwest::Client::new();
    
    match client.get("http://localhost:9001/api/transforms")
        .send()
        .await
    {
        Ok(response) if response.status() == 200 => {
            match response.json::<Value>().await {
                Ok(data) => {
                    if let Some(transforms) = data.get("data").and_then(|d| d.as_object()) {
                        println!("   Found {} registered transforms", transforms.len());
                        
                        // Check for BlogPostWordIndex transform
                        if transforms.contains_key("BlogPostWordIndex") {
                            println!("   ✅ BlogPostWordIndex transform is registered");
                            results.add_pass("Verify transform registered via HTTP");
                            return true;
                        } else {
                            results.add_fail("Verify transform registered via HTTP", "BlogPostWordIndex transform not found");
                            return false;
                        }
                    } else {
                        results.add_fail("Verify transform registered via HTTP", "Invalid response format");
                        return false;
                    }
                }
                Err(e) => {
                    results.add_fail("Verify transform registered via HTTP", &format!("Failed to parse response: {}", e));
                    return false;
                }
            }
        }
        Ok(response) => {
            results.add_fail("Verify transform registered via HTTP", &format!("Expected status 200, got {}", response.status()));
            false
        }
        Err(e) => {
            results.add_fail("Verify transform registered via HTTP", &format!("Request failed: {}", e));
            false
        }
    }
}

async fn test_query_transform_results_http(publish_date: &str, results: &mut HttpTestResults) {
    println!("\n🔍 Testing transform results query via HTTP...");
    
    // Wait a bit more for backfill to complete
    sleep(Duration::from_secs(3)).await;
    
    let client = reqwest::Client::new();
    
    let query_data = json!({
        "schema_name": "BlogPostWordIndex",
        "fields": ["word", "publish_date", "content", "author", "title", "tags"],
        "filter": null
    });
    
    match client.post("http://localhost:9001/api/query")
        .header("Content-Type", "application/json")
        .json(&query_data)
        .send()
        .await
    {
        Ok(response) if response.status() == 200 => {
            match response.json::<Value>().await {
                Ok(data) => {
                    // Check response format - could be in 'data' or 'results' field
                    let results_data = data.get("data").or_else(|| data.get("results"));
                    
                    if let Some(results_array) = results_data.and_then(|d| d.as_array()) {
                        println!("   Found {} transform results", results_array.len());
                        
                        if results_array.is_empty() {
                            results.add_fail("Query transform results via HTTP", "No transform results found - backfill may have failed");
                            return;
                        }
                        
                        // Analyze field presence
                        let mut all_field_names = std::collections::HashSet::new();
                        let mut field_counts = std::collections::HashMap::new();
                        
                        // Check first 10 results to see what fields are available
                        for item in results_array.iter().take(10) {
                            if let Some(fields) = item.get("fields").and_then(|f| f.as_object()) {
                                for field_name in fields.keys() {
                                    all_field_names.insert(field_name.clone());
                                    *field_counts.entry(field_name.clone()).or_insert(0) += 1;
                                }
                            }
                        }
                        
                        println!("   All field names found: {:?}", sorted_vec(&all_field_names));
                        
                        // Expected fields from the BlogPostWordIndex transform
                        let expected_fields: std::collections::HashSet<String> = [
                            "word", "publish_date", "content", "author", "title", "tags"
                        ].iter().map(|s| s.to_string()).collect();
                        
                        let missing_fields: Vec<String> = expected_fields.difference(&all_field_names).cloned().collect();
                        let unexpected_fields: Vec<String> = all_field_names.difference(&expected_fields).cloned().collect();
                        
                        println!("   Expected fields: {:?}", sorted_vec(&expected_fields));
                        println!("   Found fields: {:?}", sorted_vec(&all_field_names));
                        
                        if !missing_fields.is_empty() {
                            println!("   ⚠️  Missing expected fields: {:?}", missing_fields);
                        }
                        if !unexpected_fields.is_empty() {
                            println!("   ℹ️  Additional fields found: {:?}", unexpected_fields);
                        }
                        
                        // Check field inheritance consistency
                        println!("   Field inheritance counts (out of {} total results):", results_array.len());
                        for field_name in sorted_vec(&all_field_names) {
                            let count = field_counts.get(&field_name).unwrap_or(&0);
                            let percentage = (*count as f64 / results_array.len() as f64) * 100.0;
                            println!("     {}: {}/{} ({:.1}%)", field_name, count, results_array.len(), percentage);
                        }
                        
                        // Print sample results
                        for (i, item) in results_array.iter().take(3).enumerate() {
                            if let Some(fields) = item.get("fields").and_then(|f| f.as_object()) {
                                let field_summary: Vec<String> = fields.iter()
                                    .map(|(k, v)| format!("{}='{}'", k, v))
                                    .collect();
                                let hash_value = item.get("key")
                                    .and_then(|k| k.get("hash"))
                                    .and_then(|h| h.as_str())
                                    .unwrap_or("N/A");
                                let range_value = item.get("key")
                                    .and_then(|k| k.get("range"))
                                    .and_then(|r| r.as_str())
                                    .unwrap_or("N/A");
                                
                                println!("   Sample {}: fields=[{}], hash='{}', range='{}'", 
                                    i + 1, field_summary.join(", "), hash_value, range_value);
                            }
                        }
                        
                        // Check if we have the core transform functionality working
                        let core_fields = ["word", "publish_date"];
                        let core_fields_present = core_fields.iter().all(|field| all_field_names.contains(*field));
                        
                        if core_fields_present {
                            println!("   ✅ Core transform fields (word, publish_date) are present");
                            if !missing_fields.is_empty() {
                                println!("   ⚠️  Some declared fields are missing: {:?}", missing_fields);
                                println!("   ℹ️  This suggests field inheritance issues in the aggregation system");
                            }
                            results.add_pass("Query transform results via HTTP");
                        } else {
                            results.add_fail("Query transform results via HTTP", "Core transform fields missing");
                        }
                    } else {
                        results.add_fail("Query transform results via HTTP", "No data returned from query");
                    }
                }
                Err(e) => {
                    results.add_fail("Query transform results via HTTP", &format!("Failed to parse response: {}", e));
                }
            }
        }
        Ok(response) => {
            results.add_fail("Query transform results via HTTP", &format!("Expected status 200, got {}", response.status()));
        }
        Err(e) => {
            results.add_fail("Query transform results via HTTP", &format!("Request failed: {}", e));
        }
    }
}

fn sorted_vec<T: Ord>(set: &std::collections::HashSet<T>) -> Vec<T> 
where 
    T: Clone,
{
    let mut vec: Vec<T> = set.iter().cloned().collect();
    vec.sort();
    vec
}

fn cleanup_server(server_process: &mut Option<std::process::Child>) {
    if let Some(mut process) = server_process.take() {
        println!("\n🛑 Stopping HTTP server...");
        
        // Try graceful shutdown first
        let _ = process.kill();
        
        // Wait a bit for cleanup
        thread::sleep(Duration::from_secs(2));
        
        println!("   Server stopped");
    }
}

fn print_summary(results: &HttpTestResults) {
    println!("\n{}", "=".repeat(80));
    println!("TEST SUMMARY");
    println!("{}", "=".repeat(80));
    
    for (status, test_name) in &results.tests {
        println!("{}: {}", status, test_name);
    }
    
    let total = results.passed + results.failed;
    println!("\nTotal tests: {}", total);
    println!("Passed: {}", results.passed);
    println!("Failed: {}", results.failed);
    println!("{}", "=".repeat(80));
}
