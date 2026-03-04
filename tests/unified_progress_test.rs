#[cfg(feature = "aws-backend")]
#[tokio::test]
async fn test_dynamo_progress_persistence_and_backfill_integration() {
    use fold_db::fold_db_core::infrastructure::backfill_tracker::BackfillTracker;
    use fold_db::progress::{DynamoDbProgressStore, Job, JobType, ProgressStore};
    use fold_db::storage::config::CloudConfig;
    use std::sync::Arc;

    println!("Starting DynamoDB Progress Tracker Test...");

    // 1. Setup - Create Store
    // We assume a table exists or we use a unique one if possible.
    // In CI/Dev env, "process" table usually exists.
    // We'll use a unique ID to avoid collision.
    let table_name =
        std::env::var("TABLE_NAME").unwrap_or_else(|_| "folddb-process-dev".to_string());
    let region = "us-east-1".to_string();

    println!("Connecting to DynamoDB table: {}", table_name);

    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_dynamodb::config::Region::new(region))
        .load()
        .await;
    let client = aws_sdk_dynamodb::Client::new(&config);

    // Ensure table exists
    use aws_sdk_dynamodb::types::{
        AttributeDefinition, BillingMode, KeySchemaElement, KeyType, ScalarAttributeType,
    };

    let table_exists = client
        .describe_table()
        .table_name(&table_name)
        .send()
        .await
        .is_ok();
    if !table_exists {
        println!("Creating table {}", table_name);
        client
            .create_table()
            .table_name(&table_name)
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("PK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .unwrap(),
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("SK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .unwrap(),
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("PK")
                    .key_type(KeyType::Hash)
                    .build()
                    .unwrap(),
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("SK")
                    .key_type(KeyType::Range)
                    .build()
                    .unwrap(),
            )
            .billing_mode(BillingMode::PayPerRequest)
            .send()
            .await
            .expect("Failed to create table");

        // Wait for active
        loop {
            let resp = client
                .describe_table()
                .table_name(&table_name)
                .send()
                .await
                .unwrap();
            if let Some(desc) = resp.table {
                if let Some(status) = desc.table_status {
                    if matches!(status, aws_sdk_dynamodb::types::TableStatus::Active) {
                        break;
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        println!("Table active");
    }

    let store = Arc::new(DynamoDbProgressStore::new(client, table_name.clone()));

    // 2. Test Basic Persistence
    let job_id = format!("test-job-{}", uuid::Uuid::new_v4());
    let user_id = "test-user-system".to_string();

    let mut job = Job::new(job_id.clone(), JobType::Backfill)
        .with_user(user_id.clone())
        .with_metadata(serde_json::json!({"test": "true"}));

    // Set initial status
    job.update_progress(10, "Started".to_string());

    store.save(&job).await.expect("Failed to save job");
    println!("Saved generic job {}", job_id);

    // Wait for eventual consistency
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Load back using list_by_user to avoid context-dependent load()
    let jobs = store
        .list_by_user(&user_id)
        .await
        .expect("Failed to list jobs");
    let loaded = jobs.iter().find(|j| j.id == job_id);

    assert!(loaded.is_some(), "Job should exist in user list");
    let loaded_job = loaded.unwrap();
    assert_eq!(loaded_job.id, job_id);
    assert_eq!(loaded_job.user_id, Some(user_id.clone()));
    assert_eq!(loaded_job.progress_percentage, 10);

    // 3. Test BackfillTracker Integration
    println!("Testing BackfillTracker integration...");

    let tracker = BackfillTracker::new(Some(store.clone()), "global".to_string());

    let backfill_hash = format!("bf-{}", uuid::Uuid::new_v4());
    let schema_name = "test_schema".to_string();
    let transform_id = "test_transform".to_string();

    // Start Backfill (Async and Persists)
    tracker
        .start_backfill_with_hash(backfill_hash.clone(), schema_name, transform_id.clone())
        .await;

    // Verify it is in store
    // BackfillTracker saves job with ID = backfill_hash in "global" partition
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let bf_jobs_global = store
        .list_by_user("global")
        .await
        .expect("Failed to list global jobs");
    let bf_job = bf_jobs_global.iter().find(|j| j.id == backfill_hash);

    assert!(
        bf_job.is_some(),
        "Backfill job should be persisted immediately"
    );
    let bf_job = bf_job.unwrap();
    assert_eq!(bf_job.status, fold_db::progress::JobStatus::Running); // start sets it to InProgress -> Running?
                                                                       // BackfillStatus::InProgress maps to JobStatus::Running.

    // Update Backfill
    tracker.set_mutations_expected(&backfill_hash, 10).await;

    // Complete Backfill
    tracker.force_complete(&backfill_hash).await;

    // Verify completion
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let bf_jobs_global_end = store
        .list_by_user("global")
        .await
        .expect("Failed to list global jobs");
    let bf_completed = bf_jobs_global_end.iter().find(|j| j.id == backfill_hash);

    assert!(bf_completed.is_some());
    let bf_completed = bf_completed.unwrap();
    assert_eq!(
        bf_completed.status,
        fold_db::progress::JobStatus::Completed
    );

    println!("Backfill integration verified successfully!");

    // Note: Jobs are not deleted - they will expire naturally via TTL or be overwritten
    // No cleanup needed
}
