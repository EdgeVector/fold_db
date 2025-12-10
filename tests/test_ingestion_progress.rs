use datafold::ingestion::progress::{DynamoDbProgressStore, IngestionProgress, ProgressStore};
use datafold::logging::core::run_with_user;
use datafold::ingestion::IngestionStep;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn test_dynamodb_progress_store_lifecycle() {
    let table_name = format!("DataFoldStorageTest-Progress-{}", Uuid::new_v4());
    
    // Create store (will create table)
    let store = DynamoDbProgressStore::new(table_name.clone()).await.expect("Failed to create store");
    let progress_id = Uuid::new_v4().to_string();
    let user_id = "test-user-1";

    run_with_user(user_id, async {
        // 1. Save initial progress
        let progress = IngestionProgress::new(progress_id.clone());
        store.save(&progress).await.expect("Failed to save progress");
        
        // 2. Load and verify
        let loaded = store.load(&progress_id).await.expect("Failed to load").expect("Progress not found");
        assert_eq!(loaded.id, progress_id);
        assert_eq!(loaded.current_step, IngestionStep::ValidatingConfig);

        // 3. Update progress
        let mut updated = loaded.clone();
        updated.update_step(IngestionStep::FlatteningData, "Flattening...".to_string());
        store.save(&updated).await.expect("Failed to update");

        // 4. Verify update
        let reloaded = store.load(&progress_id).await.expect("Failed to reload").expect("Progress not found");
        assert_eq!(reloaded.current_step, IngestionStep::FlatteningData);
        assert_eq!(reloaded.status_message, "Flattening...");

        // 5. List progress
        let list = store.list().await.expect("Failed to list");
        assert!(list.iter().any(|p| p.id == progress_id));
        
        // 6. Delete
        store.delete(&progress_id).await.expect("Failed to delete");
        let deleted = store.load(&progress_id).await.expect("Failed to load after delete");
        assert!(deleted.is_none());
    }).await;
    
    // Clean up table (optional, but good for tests)
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_dynamodb::Client::new(&config);
    let _ = client.delete_table().table_name(table_name).send().await;
}

#[tokio::test]
async fn test_progress_user_isolation() {
    let table_name = format!("DataFoldStorageTest-Progress-Isolation-{}", Uuid::new_v4());
    let store = DynamoDbProgressStore::new(table_name.clone()).await.expect("Failed to create store");
    
    let progress_id = Uuid::new_v4().to_string();
    
    // User 1 saves progress
    run_with_user("user-1", async {
        let p = IngestionProgress::new(progress_id.clone());
        store.save(&p).await.expect("User 1 failed to save");
    }).await;

    // User 2 should NOT see it
    run_with_user("user-2", async {
        let loaded = store.load(&progress_id).await.expect("User 2 load failed");
        assert!(loaded.is_none(), "User 2 should not see User 1's progress");
        
        let list = store.list().await.expect("User 2 list failed");
        assert!(list.is_empty(), "User 2 list should be empty");
    }).await;

    // User 1 SHOULD see it
    run_with_user("user-1", async {
        let loaded = store.load(&progress_id).await.expect("User 1 load failed");
        assert!(loaded.is_some(), "User 1 should see their own progress");
    }).await;
    
    // Cleanup
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_dynamodb::Client::new(&config);
    let _ = client.delete_table().table_name(table_name).send().await;
}
