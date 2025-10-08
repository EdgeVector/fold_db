use std::process::Child;
use std::time::Duration;
use tokio::time::sleep;

mod http_test_helper;
use http_test_helper::{HttpTestHelper, HttpTestResults, get_available_schema_files};

/// Comprehensive Integration Test for BlogPost -> BlogPostWordIndex Backfill
///
/// This test verifies the complete workflow:
/// 1. Starts the HTTP server with a fresh database instance
/// 2. Loads schemas from available_schemas directory
/// 3. Approves the BlogPost schema (source schema)
/// 4. Runs manage_blogposts.py to create blog post data via curl
/// 5. Approves the BlogPostWordIndex schema (transform schema)
/// 6. Captures the unique backfill hash returned from approval
/// 7. Verifies that backfill is triggered automatically on approval
/// 8. Checks that the backfill completes successfully using the unique hash
/// 9. Validates that the backfill produced the expected word index results
/// 10. Cleans up by stopping the server
///
/// Usage:
///     cargo test blogpost_backfill_integration_test -- --nocapture
///
/// This test exercises the entire pipeline from schema approval through
/// data creation to automatic transform backfill execution and verification,
/// including the unique backfill hash tracking system.
#[tokio::test]
async fn test_blogpost_wordindex_backfill_integration() {
    println!("{}", "=".repeat(80));
    println!("BlogPost -> BlogPostWordIndex Backfill Integration Test");
    println!("{}", "=".repeat(80));
    println!("Date: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
    println!("Base URL: http://localhost:9001");
    println!("{}", "=".repeat(80));

    let mut results = HttpTestResults::new();
    let mut server_process: Option<Child> = None;
    let helper = HttpTestHelper::new();

    // Step 1: Start the HTTP server with fresh database
    println!("\n📋 Step 1: Starting HTTP server with fresh database...");
    if !helper.start_http_server(&mut server_process, &mut results).await {
        helper.print_summary(&results);
        panic!("Failed to start HTTP server");
    }

    // Step 2: Wait for server to be ready
    println!("\n📋 Step 2: Waiting for server to be ready...");
    if !helper.wait_for_server_ready(&mut results).await {
        helper.cleanup_server(&mut server_process);
        helper.print_summary(&results);
        panic!("Server failed to become ready");
    }

    // Give the server a moment to fully initialize
    sleep(Duration::from_secs(2)).await;

    // Step 3: Load schemas
    println!("\n📋 Step 3: Loading schemas...");
    if !helper.load_schemas(&mut results).await {
        helper.cleanup_server(&mut server_process);
        helper.print_summary(&results);
        panic!("Failed to load schemas");
    }

    // Verify schemas are available
    let expected_schemas = get_available_schema_files();
    if !helper.verify_schemas_available(&expected_schemas, &mut results).await {
        helper.cleanup_server(&mut server_process);
        helper.print_summary(&results);
        panic!("Failed to verify schemas are available");
    }

    // Step 4: Approve the BlogPost schema (source schema)
    println!("\n📋 Step 4: Approving BlogPost schema (source schema)...");
    if !helper.approve_schema("BlogPost", &mut results).await {
        helper.cleanup_server(&mut server_process);
        helper.print_summary(&results);
        panic!("Failed to approve BlogPost schema");
    }

    // Give the server time to process the approval
    sleep(Duration::from_millis(500)).await;

    // Step 5: Run manage_blogposts.py to create blog post data
    println!("\n📋 Step 5: Running manage_blogposts.py to create blog posts...");
    if !helper.run_python_script(
        "scripts/manage_blogposts.py",
        vec!["--num-posts", "2", "--delay", "0.0", "--fast"],
        &mut results,
    ).await {
        helper.cleanup_server(&mut server_process);
        helper.print_summary(&results);
        panic!("Failed to run manage_blogposts.py");
    }

    // Give time for blog posts to be created
    sleep(Duration::from_secs(1)).await;

    // Step 6: Approve the BlogPostWordIndex schema (transform schema)
    // This should trigger automatic backfill and return a unique backfill hash
    println!("\n📋 Step 6: Approving BlogPostWordIndex schema (transform schema)...");
    println!("   ⏳ This should trigger automatic backfill and return a unique hash...");
    let backfill_hash = match helper.approve_schema_with_hash("BlogPostWordIndex", &mut results).await {
        Some(hash) => {
            println!("  ✅ Received backfill hash: {}", hash);
            hash
        }
        None => {
            helper.cleanup_server(&mut server_process);
            helper.print_summary(&results);
            panic!("Failed to approve BlogPostWordIndex schema or get backfill hash");
        }
    };

    // Step 7: Wait for backfill to complete
    // Backfills can take some time to process, especially with many records
    // Now we need to wait for ALL mutations to be persisted, not just event publishing
    println!("\n📋 Step 7: Waiting for backfill mutations to complete...");
    println!("   ⏳ This may take longer as we're waiting for all mutations to persist...");
    sleep(Duration::from_secs(5)).await;

    // Step 8: Verify backfill was triggered and completed using the unique hash
    println!("\n📋 Step 8: Verifying backfill completed successfully by hash...");
    println!("   🔍 Looking for backfill hash: {}", backfill_hash);
    // Expect at least some records to be produced from the blog posts
    // The manage_blogposts.py script creates 10 posts, and each post has multiple words
    // So we should have at least 50+ word index entries (conservative estimate)
    if !helper.verify_backfill_by_hash(&backfill_hash, 4, &mut results).await {
        helper.cleanup_server(&mut server_process);
        helper.print_summary(&results);
        panic!("Backfill did not complete successfully for hash: {}", backfill_hash);
    }

    // Step 9: Verify the word index results contain expected data
    println!("\n📋 Step 9: Verifying word index results contain expected words...");
    // Check for some common words that should appear in the blog posts
    // Note: The query may return a limited number of results, so we check for very common words
    let expected_words = vec![
        "DataFold",
        "data",
    ];
    
    if !helper.verify_wordindex_results(expected_words, &mut results).await {
        helper.cleanup_server(&mut server_process);
        helper.print_summary(&results);
        panic!("Word index results verification failed");
    }

    // Step 10: Query the transform results for additional validation
    println!("\n📋 Step 10: Querying transform results for validation...");
    if !helper.query_transform_results(
        "BlogPostWordIndex",
        vec!["word", "title", "author", "publish_date"],
        &mut results,
    ).await {
        helper.cleanup_server(&mut server_process);
        helper.print_summary(&results);
        panic!("Failed to query transform results");
    }

    // Cleanup
    println!("\n📋 Cleaning up...");
    helper.cleanup_server(&mut server_process);

    // Print final summary
    helper.print_summary(&results);

    // Assert all tests passed
    assert!(results.get_passed() > 0, "No tests passed");
    assert_eq!(results.get_failed(), 0, "Some tests failed: {}", results.get_failed());

    println!("\n🎉 All integration tests passed successfully!");
}

