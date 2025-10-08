use std::time::Duration;

/// Test to verify that backfill failure threshold is respected
/// This is a simplified test that verifies the backfill tracker's failure detection logic
#[test]
fn test_backfill_failure_threshold_detection() {
    use datafold::fold_db_core::infrastructure::backfill_tracker::{BackfillTracker, BackfillStatus};
    
    // Create a backfill tracker
    let tracker = BackfillTracker::new();
    
    // Generate and start a backfill
    let backfill_hash = BackfillTracker::generate_hash("TestTransform", "TestSource");
    tracker.start_backfill_with_hash(
        backfill_hash.clone(),
        "TestTransform".to_string(),
        "TestSource".to_string()
    );
    
    println!("✅ Started backfill with hash: {}", backfill_hash);
    
    // Set expected mutations
    tracker.set_mutations_expected(&backfill_hash, 100);
    
    // Simulate 15 failures and 5 successes (75% failure rate, should trigger failure)
    for i in 0..15 {
        tracker.increment_mutation_failed(&backfill_hash, format!("Test error {}", i));
    }
    
    for _ in 0..5 {
        tracker.increment_mutation_completed(&backfill_hash);
    }
    
    // Give a moment for state updates
    std::thread::sleep(Duration::from_millis(100));
    
    // Check backfill status
    let backfills = tracker.get_all_backfills();
    let backfill = backfills.iter()
        .find(|b| b.backfill_hash == backfill_hash)
        .expect("Backfill should exist");
    
    println!("📊 Backfill status after failures:");
    println!("   - Status: {:?}", backfill.status);
    println!("   - Mutations completed: {}", backfill.mutations_completed);
    println!("   - Mutations failed: {}", backfill.mutations_failed);
    println!("   - Error: {:?}", backfill.error);
    
    // Verify that high failure rate triggered backfill failure
    // The threshold is 10% with at least 10 total mutations
    // We have 75% failure rate (15/20), which should trigger failure
    assert_eq!(
        backfill.status,
        BackfillStatus::Failed,
        "Backfill should be marked as Failed with 75% failure rate"
    );
    
    assert!(
        backfill.error.is_some(),
        "Failed backfill should have an error message"
    );
    
    println!("✅ Backfill failure threshold detection working correctly");
}

/// Test to verify that backfills with low failure rates complete successfully
#[test]
fn test_backfill_low_failure_rate_completes() {
    use datafold::fold_db_core::infrastructure::backfill_tracker::{BackfillTracker, BackfillStatus};
    
    // Create a backfill tracker
    let tracker = BackfillTracker::new();
    
    // Generate and start a backfill
    let backfill_hash = BackfillTracker::generate_hash("TestTransform", "TestSource");
    tracker.start_backfill_with_hash(
        backfill_hash.clone(),
        "TestTransform".to_string(),
        "TestSource".to_string()
    );
    
    println!("✅ Started backfill with hash: {}", backfill_hash);
    
    // Set expected mutations
    let expected_count = 100;
    tracker.set_mutations_expected(&backfill_hash, expected_count);
    
    // Simulate 5 failures and 95 successes (5% failure rate, should complete when all done)
    for i in 0..5 {
        tracker.increment_mutation_failed(&backfill_hash, format!("Test error {}", i));
    }
    
    for _ in 0..expected_count {
        tracker.increment_mutation_completed(&backfill_hash);
    }
    
    // Give a moment for state updates
    std::thread::sleep(Duration::from_millis(100));
    
    // Check backfill status
    let backfills = tracker.get_all_backfills();
    let backfill = backfills.iter()
        .find(|b| b.backfill_hash == backfill_hash)
        .expect("Backfill should exist");
    
    println!("📊 Backfill status after low failure rate:");
    println!("   - Status: {:?}", backfill.status);
    println!("   - Mutations completed: {}", backfill.mutations_completed);
    println!("   - Mutations failed: {}", backfill.mutations_failed);
    
    // Verify that low failure rate allowed backfill to complete
    // Note: It completes when mutations_completed >= mutations_expected
    assert_eq!(
        backfill.status,
        BackfillStatus::Completed,
        "Backfill should be Completed when all expected mutations are processed"
    );
    
    assert!(
        backfill.error.is_none(),
        "Completed backfill should not have an error message"
    );
    
    println!("✅ Backfill with low failure rate completed successfully");
}

