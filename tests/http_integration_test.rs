use std::process::Child;
use std::time::Duration;
use tokio::time::sleep;

mod http_test_helper;
use http_test_helper::{HttpTestHelper, HttpTestResults, get_available_schema_files};

/// Integration Test for DataFold HTTP Server (Rust)
///
/// This test verifies the complete workflow:
/// 1. Starts the HTTP server using ./run_http_server.sh
/// 2. Loads schemas from available_schemas directory
/// 3. Verifies all schemas are discovered and accessible
/// 4. Approves the BlogPost schema
/// 5. Creates a mutation to write a blog post
/// 6. Queries the schema to verify the data
/// 7. Cleans up by stopping the server
///
/// All operations are performed using HTTP API calls with reqwest.
///
/// Usage:
///     cargo test integration_test_http -- --nocapture
///
/// The test will:
///     - Automatically start and stop the HTTP server
///     - Create a test blog post with timestamp-based data
///     - Validate the complete create -> query workflow
///     - Exit with success/failure status
///
/// Output:
///     - Detailed progress for each test step
///     - ✅ PASS for successful tests
///     - ❌ FAIL with error details for failed tests
///     - Final summary with pass/fail counts
///
/// Requirements:
///     - Rust and Cargo (for building the server)
///     - reqwest HTTP client library
///     - BlogPost schema in available_schemas/

#[tokio::test]
async fn test_datafold_http_integration() {
    println!("{}", "=".repeat(80));
    println!("DataFold HTTP Server Integration Test (Rust)");
    println!("{}", "=".repeat(80));
    println!("Date: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
    println!("Base URL: http://localhost:9001");
    println!("{}", "=".repeat(80));

    let mut results = HttpTestResults::new();
    let mut server_process: Option<Child> = None;
    let helper = HttpTestHelper::new();

    // Step 1: Start the HTTP server
    if !helper.start_http_server(&mut server_process, &mut results).await {
        helper.print_summary(&results);
        panic!("Failed to start HTTP server");
    }

    // Step 2: Wait for server to be ready
    if !helper.wait_for_server_ready(&mut results).await {
        helper.cleanup_server(&mut server_process);
        helper.print_summary(&results);
        panic!("Server failed to become ready");
    }

    // Give the server a moment to fully initialize
    sleep(Duration::from_secs(2)).await;

    // Step 3: Run tests
    if helper.load_schemas(&mut results).await {
        let expected_schemas = get_available_schema_files();
        if helper.verify_schemas_available(&expected_schemas, &mut results).await
            && helper.approve_schema("BlogPost", &mut results).await {
                if let Some(date) = helper.create_blogpost_mutation(&mut results).await {
                    helper.query_blogpost_data(&date, &mut results).await;
                }
            }
    }

    // Cleanup
    helper.cleanup_server(&mut server_process);

    // Print final summary
    helper.print_summary(&results);

    // Assert all tests passed
    assert!(results.get_passed() > 0, "No tests passed");
    assert_eq!(results.get_failed(), 0, "Some tests failed: {}", results.get_failed());
}
