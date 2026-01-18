use serde_json::{json, Value};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tokio::time::sleep;

/// HTTP Integration Test Helper
///
/// This module provides shared functionality for HTTP integration tests,
/// reducing code duplication and providing consistent test patterns.
///
/// Features:
/// - Server lifecycle management (start/stop)
/// - HTTP client operations with consistent error handling
/// - Test result tracking and reporting
/// - Common test operations (schema loading, mutations, queries)
/// - Consistent logging and output formatting
pub struct HttpTestResults {
    passed: u32,
    failed: u32,
    tests: Vec<(String, String)>,
}

impl Default for HttpTestResults {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTestResults {
    pub fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
            tests: Vec::new(),
        }
    }

    pub fn add_pass(&mut self, test_name: &str) {
        self.passed += 1;
        self.tests
            .push(("✅ PASS".to_string(), test_name.to_string()));
        // Test passed: test_name
    }

    pub fn add_fail(&mut self, test_name: &str, error_msg: &str) {
        self.failed += 1;
        self.tests.push((
            "❌ FAIL".to_string(),
            format!("{} - {}", test_name, error_msg),
        ));
        // Test failed: test_name - error_msg
    }

    pub fn get_passed(&self) -> u32 {
        self.passed
    }

    pub fn get_failed(&self) -> u32 {
        self.failed
    }

    #[allow(dead_code)]
    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }
}

pub struct HttpTestHelper {
    base_url: String,
    client: reqwest::Client,
}

impl Default for HttpTestHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTestHelper {
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:9001".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Start the HTTP server using the run_http_server.sh script
    pub async fn start_http_server(
        &self,
        _server_process: &mut Option<std::process::Child>,
        results: &mut HttpTestResults,
    ) -> bool {
        // Starting HTTP server on port 9001

        // First, kill any existing server processes
        // Cleaning up any existing server processes
        let kill_output = Command::new("pkill")
            .args(["-f", "datafold_http_server"])
            .output();

        if let Ok(output) = kill_output {
            if !output.status.success() && !output.stderr.is_empty() {
                // Note: server cleanup message
            }
        }

        // Give processes time to terminate
        sleep(Duration::from_millis(500)).await;

        // Run the server startup script
        // Running server startup script
        match Command::new("./run_http_server.sh")
            .arg("--empty-db")
            .output()
        {
            Ok(output) => {
                let stdout_str = String::from_utf8_lossy(&output.stdout);
                let stderr_str = String::from_utf8_lossy(&output.stderr);

                if !stdout_str.trim().is_empty() {
                    // Startup script stdout
                }

                if !stderr_str.trim().is_empty() {
                    // Startup script stderr
                }

                if output.status.success() {
                    // Server startup script completed successfully

                    // Give the server additional time to fully initialize
                    // Waiting for server initialization
                    sleep(Duration::from_secs(3)).await;

                    // Check if server is actually running by looking for the process
                    let ps_output = Command::new("pgrep")
                        .args(["-f", "datafold_http_server"])
                        .output();

                    match ps_output {
                        Ok(output) if output.status.success() => {
                            let stdout_str = String::from_utf8_lossy(&output.stdout);
                            let pid = stdout_str.trim();
                            if !pid.is_empty() {
                                // Server process confirmed running
                                results.add_pass("Start HTTP server");
                                true
                            } else {
                                results.add_fail(
                                    "Start HTTP server",
                                    "Server process not found after startup",
                                );
                                false
                            }
                        }
                        Ok(_) => {
                            results.add_fail(
                                "Start HTTP server",
                                "Server process not found after startup",
                            );
                            false
                        }
                        Err(e) => {
                            results.add_fail(
                                "Start HTTP server",
                                &format!("Failed to check server process: {}", e),
                            );
                            false
                        }
                    }
                } else {
                    results.add_fail(
                        "Start HTTP server",
                        &format!(
                            "Startup script failed with status: {:?}",
                            output.status.code()
                        ),
                    );
                    false
                }
            }
            Err(e) => {
                results.add_fail(
                    "Start HTTP server",
                    &format!("Failed to start server: {}", e),
                );
                false
            }
        }
    }

    /// Wait for the server to be ready with health check
    pub async fn wait_for_server_ready(&self, results: &mut HttpTestResults) -> bool {
        // Waiting for server to be ready (timeout: 60s)

        let start_time = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(15);
        let mut attempt = 0;

        while start_time.elapsed() < timeout_duration {
            attempt += 1;

            // Log progress every 10 seconds
            if attempt % 10 == 1 {
                // Attempt with elapsed time
            }

            match self
                .client
                .get(format!("{}/api/system/status", self.base_url))
                .timeout(Duration::from_secs(5))
                .send()
                .await
            {
                Ok(response) if response.status() == 200 => {
                    // Server is ready
                    results.add_pass("Wait for server ready");
                    return true;
                }
                Ok(_response) => {
                    // Server responded but not with 200
                    if attempt % 10 == 1 {
                        // Server responded with status
                    }
                }
                Err(_e) => {
                    // Server not ready yet - log error details every 10 attempts
                    if attempt % 10 == 1 {
                        // Connection error on attempt
                    }
                }
            }

            sleep(Duration::from_millis(200)).await;
        }

        let elapsed = start_time.elapsed();
        let error_msg = format!(
            "Server failed to become ready within {}s ({} attempts)",
            elapsed.as_secs(),
            attempt
        );
        // Server failed to become ready

        // Check server logs for debugging when readiness check fails
        self.check_server_logs();

        results.add_fail("Wait for server ready", &error_msg);
        false
    }

    /// Check server logs for debugging
    pub fn check_server_logs(&self) {
        // Checking server logs for debugging

        // Check if server.log exists and show recent entries
        if let Ok(log_content) = std::fs::read_to_string("server.log") {
            let lines: Vec<&str> = log_content.lines().collect();
            let _recent_lines = if lines.len() > 20 {
                &lines[lines.len() - 20..]
            } else {
                &lines
            };

            // Recent server log entries
        } else {
            // No server.log file found
        }
    }

    /// Clean up server process
    pub fn cleanup_server(&self, _server_process: &mut Option<std::process::Child>) {
        // Stopping HTTP server

        // Check logs before cleanup for debugging
        self.check_server_logs();

        // Kill any running datafold server processes
        let _ = Command::new("pkill")
            .args(["-f", "datafold_http_server"])
            .output();

        // Wait a bit for cleanup
        thread::sleep(Duration::from_millis(500));

        // Server stopped
    }

    /// Add test schemas to the schema service, then load them into the node
    pub async fn load_schemas(&self, results: &mut HttpTestResults) -> bool {
        // First, add test schemas to the schema service
        let schema_dir = "tests/schemas_for_testing";
        let schema_service_url = "http://localhost:9002";

        let entries = match std::fs::read_dir(schema_dir) {
            Ok(e) => e,
            Err(e) => {
                results.add_fail(
                    "Load schemas",
                    &format!("Failed to read schema directory: {}", e),
                );
                return false;
            }
        };

        let mut added_count = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let schema_content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        println!("  ⚠️  Failed to read {}: {}", path.display(), e);
                        continue;
                    }
                };

                let schema_value: Value = match serde_json::from_str(&schema_content) {
                    Ok(v) => v,
                    Err(e) => {
                        println!("  ⚠️  Failed to parse {}: {}", path.display(), e);
                        continue;
                    }
                };

                // Wrap schema in the format expected by the schema service
                let request_body = json!({
                    "schema": schema_value,
                    "mutation_mappers": {}
                });

                // POST schema to schema service
                match self
                    .client
                    .post(format!("{}/api/schemas", schema_service_url))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await
                {
                    Ok(response) if response.status().is_success() => {
                        added_count += 1;
                        println!(
                            "  ✅ Added schema to service: {}",
                            path.file_name().unwrap().to_string_lossy()
                        );
                    }
                    Ok(response) if response.status() == 409 => {
                        added_count += 1;
                        println!(
                            "  ✅ Schema already exists (skipped): {}",
                            path.file_name().unwrap().to_string_lossy()
                        );
                    }
                    Ok(response) => {
                        println!(
                            "  ⚠️  Failed to add {}: status {}",
                            path.file_name().unwrap().to_string_lossy(),
                            response.status()
                        );
                    }
                    Err(e) => {
                        println!(
                            "  ⚠️  Request failed for {}: {}",
                            path.file_name().unwrap().to_string_lossy(),
                            e
                        );
                    }
                }
            }
        }

        if added_count == 0 {
            results.add_fail("Load schemas", "No schemas were added to schema service");
            return false;
        }

        // Now load schemas from schema service into the node
        match self
            .client
            .post(format!("{}/api/schemas/load", self.base_url))
            .header("Content-Type", "application/json")
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => match response.json::<Value>().await {
                Ok(data) => {
                    let _available_loaded = data
                        .get("available_schemas_loaded")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let schemas_loaded_to_db = data
                        .get("schemas_loaded_to_db")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    if schemas_loaded_to_db == 0 {
                        results
                            .add_fail("Load schemas", "No schemas were loaded into node database");
                        false
                    } else {
                        println!(
                            "  ✅ Loaded {} schemas from service into node",
                            schemas_loaded_to_db
                        );
                        results.add_pass("Load schemas");
                        true
                    }
                }
                Err(e) => {
                    results.add_fail("Load schemas", &format!("Failed to parse response: {}", e));
                    false
                }
            },
            Ok(response) => {
                results.add_fail(
                    "Load schemas",
                    &format!("Expected status 200, got {}", response.status()),
                );
                false
            }
            Err(e) => {
                results.add_fail("Load schemas", &format!("Request failed: {}", e));
                false
            }
        }
    }

    /// Verify that expected schemas are available
    #[allow(dead_code)]
    pub async fn verify_schemas_available(
        &self,
        expected_schemas: &[String],
        results: &mut HttpTestResults,
    ) -> bool {
        // Verifying schemas are available

        if expected_schemas.is_empty() {
            results.add_fail("Verify schemas available", "No expected schemas provided");
            return false;
        }

        // Expected schemas

        match self
            .client
            .get(format!("{}/api/schemas", self.base_url))
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => {
                match response.json::<Value>().await {
                    Ok(data) => {
                        if let Some(schemas_data) = data.as_array() {
                            // Discovered schemas in database

                            // Extract schema names from the list
                            let mut discovered_schema_names = Vec::new();
                            for schema_obj in schemas_data {
                                if let Some(name) = schema_obj.get("name").and_then(|n| n.as_str())
                                {
                                    discovered_schema_names.push(name.to_string());
                                }
                            }

                            // Verify each expected schema is present
                            let mut all_found = true;
                            for expected_name in expected_schemas {
                                if discovered_schema_names.contains(expected_name) {
                                    // Find the schema object to get field count
                                    if let Some(schema_obj) = schemas_data.iter().find(|s| {
                                        s.get("name").and_then(|n| n.as_str())
                                            == Some(expected_name)
                                    }) {
                                        if let Some(fields) =
                                            schema_obj.get("fields").and_then(|f| f.as_object())
                                        {
                                            let _field_count = fields.len();
                                            // Schema found with fields
                                        } else {
                                            // Schema found and loaded
                                        }
                                    }
                                } else {
                                    // Schema not found
                                    results.add_fail(
                                        "Verify schemas available",
                                        &format!(
                                            "Schema '{}' not found in API response",
                                            expected_name
                                        ),
                                    );
                                    all_found = false;
                                }
                            }

                            if !all_found {
                                println!(
                                    "\n  Discovered schemas: {}",
                                    discovered_schema_names.join(", ")
                                );
                                let missing: Vec<String> = expected_schemas
                                    .iter()
                                    .filter(|s| !discovered_schema_names.contains(s))
                                    .cloned()
                                    .collect();
                                println!("  Missing schemas: {}", missing.join(", "));
                                return false;
                            }

                            results.add_pass("Verify schemas available");
                            true
                        } else {
                            results.add_fail("Verify schemas available", "Invalid response format");
                            false
                        }
                    }
                    Err(e) => {
                        results.add_fail(
                            "Verify schemas available",
                            &format!("Failed to parse response: {}", e),
                        );
                        false
                    }
                }
            }
            Ok(response) => {
                results.add_fail(
                    "Verify schemas available",
                    &format!("Expected status 200, got {}", response.status()),
                );
                false
            }
            Err(e) => {
                results.add_fail(
                    "Verify schemas available",
                    &format!("Request failed: {}", e),
                );
                false
            }
        }
    }

    /// Approve a schema by name
    pub async fn approve_schema(&self, schema_name: &str, results: &mut HttpTestResults) -> bool {
        println!("\n✅ Approving {} schema...", schema_name);

        match self
            .client
            .post(format!(
                "{}/api/schema/{}/approve",
                self.base_url, schema_name
            ))
            .header("Content-Type", "application/json")
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => {
                match response.json::<Value>().await {
                    Ok(data) => {
                        // Response is now either a string (backfill hash) or null
                        // Check for error field to determine failure
                        if data.get("error").is_some() {
                            results.add_fail(
                                &format!("Approve {} schema", schema_name),
                                "Schema approval failed",
                            );
                            return false;
                        }

                        println!("  {} schema approved successfully", schema_name);

                        // Check if a backfill hash was returned (for transform schemas)
                        if let Some(hash) = data.as_str() {
                            println!("  🔄 Backfill hash: {}", hash);
                        }

                        results.add_pass(&format!("Approve {} schema", schema_name));
                        true
                    }
                    Err(e) => {
                        results.add_fail(
                            &format!("Approve {} schema", schema_name),
                            &format!("Failed to parse response: {}", e),
                        );
                        false
                    }
                }
            }
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                results.add_fail(
                    &format!("Approve {} schema", schema_name),
                    &format!("Expected status 200, got {} - Body: {}", status, body),
                );
                false
            }
            Err(e) => {
                results.add_fail(
                    &format!("Approve {} schema", schema_name),
                    &format!("Request failed: {}", e),
                );
                false
            }
        }
    }

    /// Create a blog post mutation with test data
    #[allow(dead_code)]
    pub async fn create_blogpost_mutation(&self, results: &mut HttpTestResults) -> Option<String> {
        println!("\n📝 Creating blog post mutation...");

        let publish_date = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let mutation_data = json!({
            "type": "mutation",
            "schema": "BlogPost",
            "mutation_type": "create",
            "fields_and_values": {
                "title": "Integration Test Blog Post",
                "content": "This blog post was created by the integration test to verify the complete workflow of the DataFold HTTP API.",
                "author": "Integration Test Suite",
                "publish_date": publish_date,
                "tags": ["test", "integration", "automation"]
            },
            "key_value": {
                "hash": null,
                "range": publish_date
            }
        });

        println!(
            "  Creating blog post: {}",
            mutation_data["fields_and_values"]["title"]
        );
        println!("  Author: {}", mutation_data["fields_and_values"]["author"]);
        println!("  Publish date: {}", publish_date);

        match self
            .client
            .post(format!("{}/api/mutation", self.base_url))
            .header("Content-Type", "application/json")
            .json(&mutation_data)
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => {
                match response.json::<Value>().await {
                    Ok(data) => {
                        // Check if response indicates an error
                        let is_error = data.get("error").is_some();

                        if !is_error {
                            println!("  Mutation created successfully");
                            results.add_pass("Create blog post mutation");
                            Some(publish_date)
                        } else {
                            let error_msg = data
                                .get("error")
                                .and_then(|e| e.as_str())
                                .unwrap_or("Unknown error");
                            results.add_fail(
                                "Create blog post mutation",
                                &format!("Mutation failed: {}", error_msg),
                            );
                            println!(
                                "  Full response: {}",
                                serde_json::to_string_pretty(&data).unwrap_or_default()
                            );
                            None
                        }
                    }
                    Err(e) => {
                        results.add_fail(
                            "Create blog post mutation",
                            &format!("Failed to parse response: {}", e),
                        );
                        None
                    }
                }
            }
            Ok(response) => {
                let error_msg = format!("Expected status 200, got {}", response.status());
                results.add_fail("Create blog post mutation", &error_msg);
                None
            }
            Err(e) => {
                results.add_fail(
                    "Create blog post mutation",
                    &format!("Request failed: {}", e),
                );
                None
            }
        }
    }

    /// Create a blog post mutation with custom data
    #[allow(dead_code)]
    pub async fn create_custom_blogpost_mutation(
        &self,
        title: &str,
        content: &str,
        author: &str,
        publish_date: &str,
        tags: Vec<&str>,
        results: &mut HttpTestResults,
    ) -> Option<String> {
        println!("\n📝 Creating custom blog post mutation...");

        let mutation_data = json!({
            "type": "mutation",
            "schema": "BlogPost",
            "mutation_type": "create",
            "fields_and_values": {
                "title": title,
                "content": content,
                "author": author,
                "publish_date": publish_date,
                "tags": tags
            },
            "key_value": {
                "hash": null,
                "range": publish_date
            }
        });

        println!("  Creating blog post: {}", title);
        println!("  Author: {}", author);
        println!("  Publish date: {}", publish_date);

        match self
            .client
            .post(format!("{}/api/mutation", self.base_url))
            .header("Content-Type", "application/json")
            .json(&mutation_data)
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => {
                match response.json::<Value>().await {
                    Ok(data) => {
                        // Check if response indicates success (either a boolean true or any successful data)
                        let is_error = data.get("error").is_some();

                        if !is_error {
                            println!("  Custom mutation created successfully");
                            results.add_pass("Create custom blog post mutation");
                            Some(publish_date.to_string())
                        } else {
                            let error_msg = data
                                .get("error")
                                .and_then(|e| e.as_str())
                                .unwrap_or("Unknown error");
                            results.add_fail(
                                "Create custom blog post mutation",
                                &format!("Mutation failed: {}", error_msg),
                            );
                            None
                        }
                    }
                    Err(e) => {
                        results.add_fail(
                            "Create custom blog post mutation",
                            &format!("Failed to parse response: {}", e),
                        );
                        None
                    }
                }
            }
            Ok(response) => {
                results.add_fail(
                    "Create custom blog post mutation",
                    &format!("Expected status 200, got {}", response.status()),
                );
                None
            }
            Err(e) => {
                results.add_fail(
                    "Create custom blog post mutation",
                    &format!("Request failed: {}", e),
                );
                None
            }
        }
    }

    /// Query blog post data
    #[allow(dead_code)]
    pub async fn query_blogpost_data(
        &self,
        publish_date: &str,
        results: &mut HttpTestResults,
    ) -> bool {
        println!("\n🔍 Querying blog post data...");

        let query_data = json!({
            "schema_name": "BlogPost",
            "fields": ["title", "author", "publish_date", "tags", "content"]
        });

        println!("  Querying all blog posts...");

        match self
            .client
            .post(format!("{}/api/query", self.base_url))
            .header("Content-Type", "application/json")
            .json(&query_data)
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => {
                match response.json::<Value>().await {
                    Ok(data) => {
                        if let Some(results_array) = data.as_array() {
                            println!("  Query returned {} result(s)", results_array.len());

                            // Search for our test post in the returned data
                            let mut found_test_post = false;

                            // Iterate through results to find our test post
                            for item in results_array {
                                if let (Some(fields), Some(key)) =
                                    (item.get("fields"), item.get("key"))
                                {
                                    // Check if this is our test post by matching the range key (publish_date)
                                    if key.get("range").and_then(|r| r.as_str())
                                        == Some(publish_date)
                                    {
                                        found_test_post = true;
                                        println!("  ✅ Found test blog post!");
                                        println!(
                                            "  📝 Title: {}",
                                            fields
                                                .get("title")
                                                .and_then(|t| t.as_str())
                                                .unwrap_or("N/A")
                                        );
                                        println!(
                                            "  👤 Author: {}",
                                            fields
                                                .get("author")
                                                .and_then(|a| a.as_str())
                                                .unwrap_or("N/A")
                                        );
                                        println!(
                                            "  📅 Published: {}",
                                            key.get("range")
                                                .and_then(|r| r.as_str())
                                                .unwrap_or("N/A")
                                        );

                                        if let Some(tags) = fields.get("tags") {
                                            if let Some(tags_array) = tags.as_array() {
                                                let tags_str: Vec<String> = tags_array
                                                    .iter()
                                                    .filter_map(|t| t.as_str())
                                                    .map(|s| s.to_string())
                                                    .collect();
                                                println!("  🏷️  Tags: {}", tags_str.join(", "));
                                            } else {
                                                println!("  🏷️  Tags: {}", tags);
                                            }
                                        } else {
                                            println!("  🏷️  Tags: N/A");
                                        }

                                        break;
                                    }
                                }
                            }

                            if !found_test_post {
                                results.add_fail(
                                    "Query blog post data",
                                    &format!(
                                        "Test post with publish_date {} not found in results",
                                        publish_date
                                    ),
                                );
                                println!(
                                    "  Response structure: {}",
                                    serde_json::to_string_pretty(&results_array)
                                        .unwrap_or_default()
                                );
                                return false;
                            }

                            results.add_pass("Query blog post data");
                            true
                        } else {
                            results.add_fail("Query blog post data", "No data returned from query");
                            false
                        }
                    }
                    Err(e) => {
                        results.add_fail(
                            "Query blog post data",
                            &format!("Failed to parse response: {}", e),
                        );
                        false
                    }
                }
            }
            Ok(response) => {
                results.add_fail(
                    "Query blog post data",
                    &format!("Expected status 200, got {}", response.status()),
                );
                false
            }
            Err(e) => {
                results.add_fail("Query blog post data", &format!("Request failed: {}", e));
                false
            }
        }
    }

    /// Query transform results
    #[allow(dead_code)]
    pub async fn query_transform_results(
        &self,
        schema_name: &str,
        fields: Vec<&str>,
        results: &mut HttpTestResults,
    ) -> bool {
        println!("\n🔍 Querying transform results for {}...", schema_name);

        let query_data = json!({
            "schema_name": schema_name,
            "fields": fields,
            "filter": null
        });

        match self
            .client
            .post(format!("{}/api/query", self.base_url))
            .header("Content-Type", "application/json")
            .json(&query_data)
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => {
                match response.json::<Value>().await {
                    Ok(data) => {
                        if let Some(results_array) = data.as_array() {
                            println!("  Found {} transform results", results_array.len());

                            if results_array.is_empty() {
                                results.add_fail(
                                    "Query transform results",
                                    "No transform results found",
                                );
                                return false;
                            }

                            // Analyze field presence
                            let mut all_field_names = std::collections::HashSet::new();
                            let mut field_counts = std::collections::HashMap::new();

                            // Check first 10 results to see what fields are available
                            for item in results_array.iter().take(10) {
                                if let Some(fields) = item.get("fields").and_then(|f| f.as_object())
                                {
                                    for field_name in fields.keys() {
                                        all_field_names.insert(field_name.clone());
                                        *field_counts.entry(field_name.clone()).or_insert(0) += 1;
                                    }
                                }
                            }

                            println!(
                                "  All field names found: {:?}",
                                sorted_vec(&all_field_names)
                            );

                            // Print sample results
                            for (i, item) in results_array.iter().take(3).enumerate() {
                                if let Some(fields) = item.get("fields").and_then(|f| f.as_object())
                                {
                                    let field_summary: Vec<String> = fields
                                        .iter()
                                        .map(|(k, v)| format!("{}='{}'", k, v))
                                        .collect();
                                    let hash_value = item
                                        .get("key")
                                        .and_then(|k| k.get("hash"))
                                        .and_then(|h| h.as_str())
                                        .unwrap_or("N/A");
                                    let range_value = item
                                        .get("key")
                                        .and_then(|k| k.get("range"))
                                        .and_then(|r| r.as_str())
                                        .unwrap_or("N/A");

                                    println!(
                                        "  Sample {}: fields=[{}], hash='{}', range='{}'",
                                        i + 1,
                                        field_summary.join(", "),
                                        hash_value,
                                        range_value
                                    );
                                }
                            }

                            results.add_pass("Query transform results");
                            true
                        } else {
                            results
                                .add_fail("Query transform results", "No data returned from query");
                            false
                        }
                    }
                    Err(e) => {
                        results.add_fail(
                            "Query transform results",
                            &format!("Failed to parse response: {}", e),
                        );
                        false
                    }
                }
            }
            Ok(response) => {
                results.add_fail(
                    "Query transform results",
                    &format!("Expected status 200, got {}", response.status()),
                );
                false
            }
            Err(e) => {
                results.add_fail("Query transform results", &format!("Request failed: {}", e));
                false
            }
        }
    }

    /// Verify that transforms are registered
    #[allow(dead_code)]
    pub async fn verify_transforms_registered(
        &self,
        expected_transforms: &[String],
        results: &mut HttpTestResults,
    ) -> bool {
        println!("\n📋 Verifying transforms are registered...");

        match self
            .client
            .get(format!("{}/api/transforms", self.base_url))
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => match response.json::<Value>().await {
                Ok(data) => {
                    if let Some(transforms) = data.as_object() {
                        println!("  Found {} registered transforms", transforms.len());

                        let mut all_found = true;
                        for expected_transform in expected_transforms {
                            if transforms.contains_key(expected_transform) {
                                println!("  ✅ {} transform is registered", expected_transform);
                            } else {
                                println!("  ❌ {} transform not found", expected_transform);
                                results.add_fail(
                                    "Verify transforms registered",
                                    &format!("{} transform not found", expected_transform),
                                );
                                all_found = false;
                            }
                        }

                        if all_found {
                            results.add_pass("Verify transforms registered");
                            true
                        } else {
                            false
                        }
                    } else {
                        results.add_fail("Verify transforms registered", "Invalid response format");
                        false
                    }
                }
                Err(e) => {
                    results.add_fail(
                        "Verify transforms registered",
                        &format!("Failed to parse response: {}", e),
                    );
                    false
                }
            },
            Ok(response) => {
                results.add_fail(
                    "Verify transforms registered",
                    &format!("Expected status 200, got {}", response.status()),
                );
                false
            }
            Err(e) => {
                results.add_fail(
                    "Verify transforms registered",
                    &format!("Request failed: {}", e),
                );
                false
            }
        }
    }

    /// Check backfill status for a given transform/schema
    #[allow(dead_code)]
    pub async fn check_backfill_status(
        &self,
        schema_name: &str,
        results: &mut HttpTestResults,
    ) -> Option<Value> {
        println!("\n🔄 Checking backfill status for {}...", schema_name);

        match self
            .client
            .get(format!("{}/api/transforms/backfills", self.base_url))
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => {
                match response.json::<Value>().await {
                    Ok(backfills) => {
                        if let Some(backfills_array) = backfills.as_array() {
                            println!("  Found {} total backfill(s)", backfills_array.len());

                            // Find the backfill for this schema
                            for backfill in backfills_array {
                                if backfill.get("transform_id").and_then(|t| t.as_str())
                                    == Some(schema_name)
                                {
                                    println!("  ✅ Found backfill for {}", schema_name);

                                    let status = backfill
                                        .get("status")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or("Unknown");
                                    let records = backfill
                                        .get("records_produced")
                                        .and_then(|r| r.as_u64())
                                        .unwrap_or(0);
                                    let hash = backfill
                                        .get("backfill_hash")
                                        .and_then(|h| h.as_str())
                                        .unwrap_or("Unknown");

                                    println!("     Status: {}", status);
                                    println!("     Records produced: {}", records);
                                    println!("     Backfill hash: {}", hash);

                                    results.add_pass(&format!(
                                        "Check backfill status for {}",
                                        schema_name
                                    ));
                                    return Some(backfill.clone());
                                }
                            }

                            results.add_fail(
                                &format!("Check backfill status for {}", schema_name),
                                "Backfill not found",
                            );
                            return None;
                        }
                    }
                    Err(e) => {
                        results.add_fail(
                            &format!("Check backfill status for {}", schema_name),
                            &format!("Failed to parse response: {}", e),
                        );
                        return None;
                    }
                }
            }
            Ok(response) => {
                results.add_fail(
                    &format!("Check backfill status for {}", schema_name),
                    &format!("Expected status 200, got {}", response.status()),
                );
                return None;
            }
            Err(e) => {
                results.add_fail(
                    &format!("Check backfill status for {}", schema_name),
                    &format!("Request failed: {}", e),
                );
                return None;
            }
        }

        results.add_fail(
            &format!("Check backfill status for {}", schema_name),
            "Invalid response format",
        );
        None
    }

    /// Print test summary
    pub fn print_summary(&self, results: &HttpTestResults) {
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

        if results.failed > 0 {
            println!("\nFailed tests:");
            for (_, test_name) in &results.tests {
                if test_name.starts_with("❌ FAIL") {
                    println!("  {}", test_name);
                }
            }
        }

        println!("{}", "=".repeat(80));
    }

    /// Execute a mutation with JSON payload
    #[allow(dead_code)]
    pub async fn execute_mutation_json(
        &self,
        mutation_data: Value,
        results: &mut HttpTestResults,
    ) -> bool {
        match self
            .client
            .post(format!("{}/api/mutation", self.base_url))
            .header("Content-Type", "application/json")
            .json(&mutation_data)
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => match response.json::<Value>().await {
                Ok(data) => {
                    if data == true
                        || (data.is_object() && data.get("success") == Some(&json!(true)))
                    {
                        return true;
                    } else if data.is_object() && data.get("error").is_some() {
                        let error_msg = data
                            .get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("Unknown error");
                        results.add_fail(
                            "Execute mutation",
                            &format!("Mutation failed: {}", error_msg),
                        );
                        return false;
                    }
                    true
                }
                Err(e) => {
                    results.add_fail(
                        "Execute mutation",
                        &format!("Failed to parse response: {}", e),
                    );
                    false
                }
            },
            Ok(response) => {
                let error_msg = format!("Expected status 200, got {}", response.status());
                results.add_fail("Execute mutation", &error_msg);
                false
            }
            Err(e) => {
                results.add_fail("Execute mutation", &format!("Request failed: {}", e));
                false
            }
        }
    }

    /// Execute AI query analysis
    #[allow(dead_code)]
    pub async fn execute_ai_analyze(&self, request: Value) -> Result<Value, String> {
        match self
            .client
            .post(format!("{}/api/llm-query/analyze", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => response
                .json::<Value>()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e)),
            Ok(response) if response.status() == 503 => {
                Err("AI service not configured".to_string())
            }
            Ok(response) => Err(format!("AI query failed with status {}", response.status())),
            Err(e) => Err(format!("Request failed: {}", e)),
        }
    }

    /// Execute AI query plan
    #[allow(dead_code)]
    pub async fn execute_ai_query_plan(&self, request: Value) -> Result<Value, String> {
        match self
            .client
            .post(format!("{}/api/llm-query/execute", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => response
                .json::<Value>()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e)),
            Ok(response) if response.status() == 503 => {
                Err("AI service not configured".to_string())
            }
            Ok(response) => Err(format!(
                "AI execute query failed with status {}",
                response.status()
            )),
            Err(e) => Err(format!("Request failed: {}", e)),
        }
    }

    /// Execute AI chat (follow-up question)
    #[allow(dead_code)]
    pub async fn execute_ai_chat(&self, request: Value) -> Result<Value, String> {
        match self
            .client
            .post(format!("{}/api/llm-query/chat", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => response
                .json::<Value>()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e)),
            Ok(response) if response.status() == 404 => Err("Session not found".to_string()),
            Ok(response) if response.status() == 503 => {
                Err("AI service not configured".to_string())
            }
            Ok(response) => Err(format!("AI chat failed with status {}", response.status())),
            Err(e) => Err(format!("Request failed: {}", e)),
        }
    }

    /// Ingest data to create schema through the ingestion API
    #[allow(dead_code)]
    pub async fn ingest_data(&self, data: Value, results: &mut HttpTestResults) -> bool {
        self.ingest_data_with_label(data, "Ingest data", results)
            .await
    }

    pub async fn ingest_data_with_label(
        &self,
        data: Value,
        label: &str,
        results: &mut HttpTestResults,
    ) -> bool {
        let request_body = json!({
            "data": data,
            "options": {
                "auto_execute_mutations": true
            }
        });

        match self
            .client
            .post(format!("{}/api/ingestion/process", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
        {
            Ok(response) if response.status() == 200 => match response.json::<Value>().await {
                Ok(data) => {
                    if let Some(schema_name) = data.get("schema_used").and_then(|s| s.as_str()) {
                        println!("    ✅ Schema: {}", schema_name);
                    }
                    results.add_pass(label);
                    true
                }
                Err(e) => {
                    results.add_fail(label, &format!("Failed to parse response: {}", e));
                    false
                }
            },
            Ok(response) => {
                results.add_fail(
                    label,
                    &format!("Expected status 200, got {}", response.status()),
                );
                false
            }
            Err(e) => {
                results.add_fail(label, &format!("Request failed: {}", e));
                false
            }
        }
    }
}

/// Get available schema files from the tests/schemas_for_testing directory
#[allow(dead_code)]
pub fn get_available_schema_files() -> Vec<String> {
    let available_schemas_dir = "tests/schemas_for_testing";
    let mut schema_files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(available_schemas_dir) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    // Extract schema name (filename without .json extension)
                    if let Some(schema_name) = file_name.strip_suffix(".json") {
                        schema_files.push(schema_name.to_string());
                    }
                }
            }
        }
    } else {
        println!("  ⚠️  Error reading tests/schemas_for_testing directory");
    }

    schema_files
}

/// Helper function to sort a HashSet into a Vec
#[allow(dead_code)]
pub fn sorted_vec<T: Ord + Clone>(set: &std::collections::HashSet<T>) -> Vec<T> {
    let mut vec: Vec<T> = set.iter().cloned().collect();
    vec.sort();
    vec
}
