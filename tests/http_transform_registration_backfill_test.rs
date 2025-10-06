use std::process::Child;
use std::time::Duration;
use tokio::time::sleep;

mod http_test_helper;
use http_test_helper::{HttpTestHelper, HttpTestResults};

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
    // Transform Registration and Backfill HTTP Integration Test (Rust)

    let mut results = HttpTestResults::new();
    let mut server_process: Option<Child> = None;
    let helper = HttpTestHelper::new();
    
    // Step 1: Start HTTP server
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
    
    // Step 3: Run HTTP API tests
    if helper.load_schemas(&mut results).await
        && helper.verify_schemas_available(&["BlogPost".to_string()], &mut results).await
        && helper.approve_schema("BlogPost", &mut results).await {
            let publish_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            if (helper.create_custom_blogpost_mutation(
                &format!("Test Post {}", publish_date),
                "This is test content for transform backfill testing",
                "Test Author",
                &publish_date,
                vec!["test", "integration", "transform"],
                &mut results
            ).await).is_some()
                && helper.load_schemas(&mut results).await {
                    // Wait for transform registration
                    sleep(Duration::from_millis(500)).await;
                    if helper.verify_transforms_registered(&["BlogPostWordIndex".to_string()], &mut results).await {
                        // Wait for backfill to complete
                        sleep(Duration::from_millis(1000)).await;
                        helper.query_transform_results("BlogPostWordIndex", 
                            vec!["word", "publish_date", "content", "author", "title", "tags"], 
                            &mut results).await;
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
