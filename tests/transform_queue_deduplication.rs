use datafold::fold_db_core::orchestration::queue_manager::QueueManager;
use datafold::fold_db_core::FoldDB;
use tempfile::TempDir;

/// Test to verify that the transform queue properly deduplicates items based on mutation_id
#[tokio::test]
async fn test_transform_queue_deduplication_by_mutation_id() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir
        .path()
        .to_str()
        .expect("Failed to convert path to string");

    // Create a new FoldDB instance
    let fold_db = FoldDB::new(test_db_path)
        .await
        .expect("Failed to create FoldDB");

    // Get the transform orchestrator and its queue manager
    let orchestrator = fold_db
        .transform_orchestrator()
        .expect("Transform orchestrator should be available");
    let queue_manager = orchestrator.get_queue_manager();

    // Test case 1: Add the same transform with the same mutation_id multiple times
    let transform_id = "TestTransform_word";
    let mutation_id = "mutation-123";

    // First addition should succeed
    let result1 = queue_manager
        .add_item(transform_id, mutation_id)
        .expect("Failed to add first item");
    assert!(
        result1,
        "First addition should return true (item was added)"
    );

    // Second addition with same transform_id and mutation_id should be deduplicated
    let result2 = queue_manager
        .add_item(transform_id, mutation_id)
        .expect("Failed to attempt second addition");
    assert!(
        !result2,
        "Second addition should return false (item was deduplicated)"
    );

    // Third addition should also be deduplicated
    let result3 = queue_manager
        .add_item(transform_id, mutation_id)
        .expect("Failed to attempt third addition");
    assert!(
        !result3,
        "Third addition should return false (item was deduplicated)"
    );

    // Verify only one item is in the queue
    let queued_transforms = queue_manager
        .list_queued_transforms()
        .expect("Failed to list queued transforms");
    assert_eq!(
        queued_transforms.len(),
        1,
        "Should have exactly 1 item in queue"
    );
    assert_eq!(
        queued_transforms[0], transform_id,
        "Queued transform should match"
    );

    // Test case 2: Add the same transform with different mutation_ids
    let mutation_id_2 = "mutation-456";
    let mutation_id_3 = "mutation-789";

    // Adding same transform with different mutation_id should succeed
    let result4 = queue_manager
        .add_item(transform_id, mutation_id_2)
        .expect("Failed to add item with different mutation_id");
    assert!(
        result4,
        "Addition with different mutation_id should return true"
    );

    let result5 = queue_manager
        .add_item(transform_id, mutation_id_3)
        .expect("Failed to add item with third mutation_id");
    assert!(
        result5,
        "Addition with third mutation_id should return true"
    );

    // Verify we now have 3 items in the queue (same transform, different mutations)
    let queued_transforms_after = queue_manager
        .list_queued_transforms()
        .expect("Failed to list queued transforms after multiple additions");
    assert_eq!(
        queued_transforms_after.len(),
        3,
        "Should have exactly 3 items in queue"
    );

    // All items should be the same transform_id
    for transform in &queued_transforms_after {
        assert_eq!(
            transform, transform_id,
            "All queued items should have same transform_id"
        );
    }

    // Test case 3: Add different transforms with the same mutation_id
    let transform_id_2 = "TestTransform_content";
    let transform_id_3 = "TestTransform_author";
    let shared_mutation_id = "shared-mutation-999";

    // Adding different transforms with same mutation_id should all succeed
    let result6 = queue_manager
        .add_item(transform_id_2, shared_mutation_id)
        .expect("Failed to add second transform with shared mutation_id");
    assert!(result6, "Addition of second transform should return true");

    let result7 = queue_manager
        .add_item(transform_id_3, shared_mutation_id)
        .expect("Failed to add third transform with shared mutation_id");
    assert!(result7, "Addition of third transform should return true");

    // Verify we now have 5 items total in the queue
    let final_queued_transforms = queue_manager
        .list_queued_transforms()
        .expect("Failed to list final queued transforms");
    assert_eq!(
        final_queued_transforms.len(),
        5,
        "Should have exactly 5 items in queue"
    );

    // Verify the queue contains all expected transform_ids
    assert!(final_queued_transforms.contains(&transform_id.to_string()));
    assert!(final_queued_transforms.contains(&transform_id_2.to_string()));
    assert!(final_queued_transforms.contains(&transform_id_3.to_string()));

    // Test case 4: Try to add duplicates again and verify they're still deduplicated
    let duplicate_result1 = queue_manager
        .add_item(transform_id, mutation_id)
        .expect("Failed to attempt duplicate addition");
    assert!(
        !duplicate_result1,
        "Duplicate addition should still be deduplicated"
    );

    let duplicate_result2 = queue_manager
        .add_item(transform_id_2, shared_mutation_id)
        .expect("Failed to attempt duplicate addition");
    assert!(
        !duplicate_result2,
        "Duplicate addition should still be deduplicated"
    );

    // Queue size should remain the same
    let final_size = queue_manager
        .list_queued_transforms()
        .expect("Failed to list queued transforms for final check");
    assert_eq!(
        final_size.len(),
        5,
        "Queue size should remain unchanged after duplicate attempts"
    );

    println!("✅ All transform queue deduplication tests passed!");
    println!("📊 Final queue state: {:?}", final_size);
}

/// Test to verify queue manager behavior with empty initial state
#[test]
fn test_transform_queue_deduplication_empty_state() {
    // Create a queue manager with empty state
    let queue_manager = QueueManager::new_empty();

    // Test adding items to an empty queue
    let transform_id = "EmptyStateTransform";
    let mutation_id = "mutation-empty-1";

    // First addition should succeed
    let result = queue_manager
        .add_item(transform_id, mutation_id)
        .expect("Failed to add item to empty queue");
    assert!(result, "First addition to empty queue should return true");

    // Verify item is in queue
    let queued_transforms = queue_manager
        .list_queued_transforms()
        .expect("Failed to list queued transforms");
    assert_eq!(
        queued_transforms.len(),
        1,
        "Empty queue should have 1 item after addition"
    );

    // Duplicate addition should be deduplicated
    let duplicate_result = queue_manager
        .add_item(transform_id, mutation_id)
        .expect("Failed to attempt duplicate addition");
    assert!(!duplicate_result, "Duplicate addition should return false");

    // Queue size should remain 1
    let final_queued_transforms = queue_manager
        .list_queued_transforms()
        .expect("Failed to list final queued transforms");
    assert_eq!(
        final_queued_transforms.len(),
        1,
        "Queue size should remain 1 after duplicate"
    );

    println!("✅ Empty state deduplication test passed!");
}

/// Test to verify the key generation logic used for deduplication
#[test]
fn test_transform_queue_key_generation() {
    // Create a queue manager
    let queue_manager = QueueManager::new_empty();

    // Test that the same transform_id and mutation_id combination generates the same key
    let transform_id = "KeyTestTransform";
    let mutation_id = "key-test-mutation";

    // Add the same combination multiple times
    let result1 = queue_manager
        .add_item(transform_id, mutation_id)
        .expect("Failed to add first item");
    assert!(result1, "First addition should succeed");

    let result2 = queue_manager
        .add_item(transform_id, mutation_id)
        .expect("Failed to attempt duplicate addition");
    assert!(!result2, "Duplicate addition should be deduplicated");

    // Test that different combinations create different keys
    let different_mutation = "different-mutation";
    let result3 = queue_manager
        .add_item(transform_id, different_mutation)
        .expect("Failed to add with different mutation");
    assert!(result3, "Different mutation should create new entry");

    let different_transform = "DifferentTransform";
    let result4 = queue_manager
        .add_item(different_transform, mutation_id)
        .expect("Failed to add with different transform");
    assert!(result4, "Different transform should create new entry");

    // Verify we have 3 unique entries
    let queued_transforms = queue_manager
        .list_queued_transforms()
        .expect("Failed to list queued transforms");
    assert_eq!(queued_transforms.len(), 3, "Should have 3 unique entries");

    // Verify all expected transform IDs are present
    assert!(queued_transforms.contains(&transform_id.to_string()));
    assert!(queued_transforms.contains(&different_transform.to_string()));
    // Note: We can't easily verify the mutation_id part from the public API,
    // but the deduplication behavior confirms the key generation is working

    println!("✅ Key generation test passed!");
    println!("📋 Final queue contents: {:?}", queued_transforms);
}

/// Test to verify queue behavior under concurrent access simulation
#[test]
fn test_transform_queue_deduplication_concurrent_simulation() {
    // Create a queue manager
    let queue_manager = QueueManager::new_empty();

    let transform_id = "ConcurrentTestTransform";
    let mutation_id = "concurrent-mutation-123";

    // Simulate multiple rapid additions of the same item (like concurrent access)
    let mut results = Vec::new();
    for i in 0..10 {
        let result = queue_manager
            .add_item(transform_id, mutation_id)
            .unwrap_or_else(|_| panic!("Failed to add item in iteration {}", i));
        results.push(result);
    }

    // Only the first addition should return true, all others should be deduplicated
    assert!(results[0], "First addition should succeed");
    for (i, result) in results.iter().enumerate().skip(1) {
        assert!(!(*result), "Addition {} should be deduplicated", i + 1);
    }

    // Verify only one item is in the queue
    let queued_transforms = queue_manager
        .list_queued_transforms()
        .expect("Failed to list queued transforms");
    assert_eq!(
        queued_transforms.len(),
        1,
        "Should have exactly 1 item despite multiple additions"
    );
    assert_eq!(
        queued_transforms[0], transform_id,
        "Queued transform should match"
    );

    println!("✅ Concurrent simulation test passed!");
    println!("📊 Results: {:?}", results);
}
