//! Test to verify which API endpoints are secured and which are not
//! 
//! This test suite verifies the current security integration status of all API endpoints
//! and demonstrates how to properly secure endpoints that should require authentication.

use datafold::security::ClientSecurity;
use datafold::datafold_node::{DataFoldNode, NodeConfig, DataFoldHttpServer};
use reqwest::Client;
use serde_json::{json, Value};
use std::net::TcpListener;
use tempfile::tempdir;
use tokio::time::Duration;

/// Test helper to start a server
async fn start_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let temp_dir = tempdir().unwrap();
    let config = NodeConfig::new(temp_dir.path().to_path_buf());
    let node = DataFoldNode::load(config).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let bind_addr = format!("127.0.0.1:{}", addr.port());

    let server = DataFoldHttpServer::new(node, &bind_addr)
        .await
        .expect("server init");

    let server_addr = bind_addr.clone();
    let handle = tokio::spawn(async move {
        server.run().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(500)).await;
    (server_addr, handle)
}

#[tokio::test]
async fn test_current_api_security_status() {
    let (server_addr, _handle) = start_test_server().await;
    let client = Client::new();

    // Test which endpoints currently require authentication vs. those that don't
    
    // ===== ENDPOINTS THAT SHOULD NOT REQUIRE AUTH =====
    let public_endpoints = vec![
        "/api/schemas/status",
        "/api/system/status", 
        "/api/logs",
        "/api/security/status",
        "/api/security/examples",
    ];
    
    println!("Testing public endpoints (should work without auth):");
    for endpoint in public_endpoints {
        let url = format!("http://{}{}", server_addr, endpoint);
        let response = client.get(&url).send().await.unwrap();
        println!("  {} -> {}", endpoint, response.status());
        assert!(
            response.status().is_success(), 
            "Public endpoint {} should work without auth", 
            endpoint
        );
    }

    // ===== ENDPOINTS THAT CURRENTLY DON'T REQUIRE AUTH BUT MAYBE SHOULD =====
    let potentially_sensitive_endpoints = vec![
        ("/api/schemas", "GET"),
        ("/api/schema", "POST"), // Schema creation
        ("/api/execute", "POST"), // Query execution
        ("/api/query", "POST"),
        ("/api/mutation", "POST"),
        ("/api/ingestion/process", "POST"),
        ("/api/system/reset-database", "POST"), // Very sensitive!
    ];
    
    println!("\nTesting potentially sensitive endpoints (currently public):");
    for (endpoint, method) in potentially_sensitive_endpoints {
        let url = format!("http://{}{}", server_addr, endpoint);
        
        let response = match method {
            "GET" => client.get(&url).send().await.unwrap(),
            "POST" => client.post(&url).json(&json!({})).send().await.unwrap(),
            _ => continue,
        };
        
        println!("  {} {} -> {}", method, endpoint, response.status());
        
        // These endpoints currently work without auth, but this test documents
        // which ones might need to be secured in the future
        if response.status().is_success() || response.status() == 400 {
            println!("    ⚠️  This endpoint is accessible without authentication");
        }
    }

    // ===== ENDPOINTS THAT DO REQUIRE AUTH =====
    println!("\nTesting secured endpoints:");
    
    // Only the security/protected endpoint currently requires auth
    let protected_url = format!("http://{}/api/security/protected", server_addr);
    let invalid_message = json!({
        "payload": "aW52YWxpZCBwYXlsb2FkIGRhdGE=", // Base64 encoded string instead of object
        "signature": "invalid",
        "public_key_id": "fake",
        "timestamp": 1234567890
    });
    
    let response = client
        .post(&protected_url)
        .json(&invalid_message)
        .send()
        .await
        .unwrap();
    
    println!("  POST /api/security/protected -> {}", response.status());
    assert_eq!(response.status(), 401, "Protected endpoint should reject invalid auth");
}

#[tokio::test]
async fn test_security_middleware_integration_example() {
    let (server_addr, _handle) = start_test_server().await;
    let client = Client::new();

    // 1. Set up a valid client
    let keypair = ClientSecurity::generate_client_keypair().unwrap();
    let registration_request = ClientSecurity::create_registration_request(
        &keypair,
        "test_user".to_string(),
        vec!["read".to_string(), "write".to_string(), "admin".to_string()],
    );
    
    // Register the key
    let register_url = format!("http://{}/api/security/system-key", server_addr);
    let response = client
        .post(&register_url)
        .json(&registration_request)
        .send()
        .await
        .unwrap();
    
    assert!(response.status().is_success());

    // Create a signer
    let signer = ClientSecurity::create_signer(keypair);

    // 2. Test the one existing protected endpoint
    let payload = json!({
        "action": "access_protected_resource",
        "resource_id": "test_resource"
    });
    
    let signed_message = ClientSecurity::sign_message(&signer, payload).unwrap();
    
    let protected_url = format!("http://{}/api/security/protected", server_addr);
    let response = client
        .post(&protected_url)
        .json(&signed_message)
        .send()
        .await
        .unwrap();
    
    assert!(response.status().is_success());
    let protected_response: Value = response.json().await.unwrap();
    assert!(protected_response["success"].as_bool().unwrap());
    assert_eq!(protected_response["authenticated_user"], "test_user");
    assert!(protected_response["data"].is_string());
}

#[tokio::test]
async fn test_sensitive_operations_without_auth() {
    let (server_addr, _handle) = start_test_server().await;
    let client = Client::new();

    // Test that sensitive operations currently work without authentication
    // This test documents the current state and can be used to verify
    // when these endpoints are properly secured

    // Database reset - VERY sensitive operation!
    let reset_url = format!("http://{}/api/system/reset-database", server_addr);
    let response = client.post(&reset_url).send().await.unwrap();
    
    println!("Database reset without auth: {}", response.status());
    // This might return 500 due to test environment, but shouldn't return 401/403
    assert_ne!(response.status(), 401);
    assert_ne!(response.status(), 403);

    // Schema creation
    let schema_url = format!("http://{}/api/schema", server_addr);
    let test_schema = json!({
        "name": "test_schema",
        "fields": []
    });
    
    let response = client
        .post(&schema_url)
        .json(&test_schema)
        .send()
        .await
        .unwrap();
    
    println!("Schema creation without auth: {}", response.status());
    // Should be accessible (though might fail for other reasons)
    assert_ne!(response.status(), 401);
    assert_ne!(response.status(), 403);

    // Query execution
    let query_url = format!("http://{}/api/query", server_addr);
    let test_query = json!({
        "query": "test query"
    });
    
    let response = client
        .post(&query_url)
        .json(&test_query)
        .send()
        .await
        .unwrap();
    
    println!("Query execution without auth: {}", response.status());
    assert_ne!(response.status(), 401);
    assert_ne!(response.status(), 403);
}

#[tokio::test]
async fn test_security_recommendations() {
    // This test documents which endpoints should be secured in the future
    
    let recommendations = vec![
        ("POST /api/schema", "Schema creation should require write permissions"),
        ("DELETE /api/schema/{name}", "Schema deletion should require admin permissions"),
        ("POST /api/system/reset-database", "Database reset should require admin permissions"),
        ("POST /api/mutation", "Data mutations should require write permissions"),
        ("POST /api/ingestion/process", "Data ingestion should require write permissions"),
        ("POST /api/ingestion/openrouter-config", "Config changes should require admin permissions"),
        ("POST /api/transforms/queue/{id}", "Transform execution should require write permissions"),
    ];
    
    println!("Security recommendations for API endpoints:");
    for (endpoint, recommendation) in recommendations {
        println!("  {}: {}", endpoint, recommendation);
    }
    
    // This test always passes - it's just for documentation
    // This test always passes - it's just for documentation
}

/// Example test showing how to implement endpoint-specific security
#[tokio::test]
async fn test_endpoint_security_implementation_example() {
    let (server_addr, _handle) = start_test_server().await;
    let client = Client::new();

    // 1. Register a system-wide public key
    let keypair = ClientSecurity::generate_client_keypair().unwrap();
    let registration_request = ClientSecurity::create_registration_request(
        &keypair,
        "system_admin".to_string(),
        vec!["admin".to_string()],
    );

    let register_url = format!("http://{}/api/security/system-key", server_addr);
    let response = client
        .post(&register_url)
        .json(&registration_request)
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());

    // 2. Create a client signer
    let signer = ClientSecurity::create_signer(keypair);

    // 3. Access a protected endpoint with a valid signature
    let payload = json!({ "action": "get_admin_data" });
    let signed_message = ClientSecurity::sign_message(&signer, payload).unwrap();

    let verify_url = format!("http://{}/api/security/verify", server_addr);
    let response = client
        .post(&verify_url)
        .json(&signed_message)
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
    let verify_response: Value = response.json().await.unwrap();
    assert!(verify_response["success"].as_bool().unwrap());
    let verification_result = &verify_response["verification_result"];
    assert!(verification_result["is_valid"].as_bool().unwrap());
    assert_eq!(verification_result["owner_id"], "system_admin");
}

#[tokio::test]
#[ignore] // This requires admin privileges which are not yet fully implemented
async fn test_system_reset_with_admin_privileges() {
    let (server_addr, node_handle) = start_test_server().await;
    let client = Client::new();

    // Test that sensitive operations currently work without authentication
    // This test documents the current state and can be used to verify
    // when these endpoints are properly secured

    // Database reset - VERY sensitive operation!
    let reset_url = format!("http://{}/api/system/reset-database", server_addr);
    let response = client.post(&reset_url).send().await.unwrap();
    
    println!("Database reset without auth: {}", response.status());
    // Verify the response
    assert_eq!(response.status(), 200);
    let reset_response: Value = response.json().await.unwrap();
    assert_eq!(reset_response["message"], "Database reset successfully");

    // Clean up
    node_handle.abort();
}