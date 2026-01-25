#![cfg(feature = "lambda")]
use datafold::ingestion::progress::IngestionStep;
use datafold::lambda::{LambdaConfig, LambdaContext, LambdaLogging};
use datafold::storage::DatabaseConfig;
use serde_json::json;

use tokio::time::{sleep, Duration, Instant};

#[tokio::test]
async fn test_ingestion_performance_breakdown() {
    std::env::set_var("FOLDB_USER_ID", "test_user");
    println!("Starting Ingestion Performance Test (Local)...");

    // 1. Setup Lambda Context (Local)
    let temp_dir =
        std::env::temp_dir().join(format!("ingestion_perf_test_{}", uuid::Uuid::new_v4()));
    let storage_config = DatabaseConfig::Local {
        path: temp_dir.clone(),
    };

    // We use NoOp logger to avoid noise, or Stdout if we want to see logs
    let config = LambdaConfig::new(storage_config, LambdaLogging::Stdout)
        .with_schema_service_url("test://mock".to_string()); // Use mock schema service

    // Initialize (might fail if already initialized, but we assume clean runner for this test file)
    match LambdaContext::init(config).await {
        Ok(_) => println!("Context initialized."),
        Err(e) => {
            if e.to_string().contains("already initialized") {
                println!("Context already initialized, proceeding...");
            } else {
                panic!("Failed to init context: {}", e);
            }
        }
    }

    // 2. Prepare Sample Data (Similar to what might be causing issues)
    // The user mentioned "sample ingestion". I'll use a moderate size sample.
    let mut data = Vec::new();
    for i in 0..100 {
        data.push(json!({
            "id": format!("user_{}", i),
            "name": format!("User {}", i),
            "bio": format!("This is a bio for user {}. It has some length to it to make string processing non-trivial.", i),
            "stats": {
                "followers": i * 10,
                "following": i * 5
            },
            "tags": ["tag1", "tag2", "tag3"]
        }));
    }
    let json_data = json!(data);

    println!("Ingesting {} items...", data.len());

    // 3. Start Ingestion
    // auto_execute = true to measure execution time
    let start_total = Instant::now();
    let test_progress_id = uuid::Uuid::new_v4().to_string();
    let progress_id = LambdaContext::ingest_json(
        json_data,
        true, // auto_execute
        0,
        "test_key".to_string(),
        "test_user".to_string(),
        test_progress_id,
    )
    .await
    .expect("Failed to start ingestion");

    println!("Ingestion started. Progress ID: {}", progress_id);

    // 4. Poll and Measure Steps
    let mut last_step = IngestionStep::ValidatingConfig;
    let mut step_start_time = Instant::now();
    let mut step_durations = std::collections::HashMap::new();

    // Initial wait for progress to start
    let mut started = false;
    let timeout = Duration::from_secs(30);
    let start_wait = Instant::now();

    loop {
        if start_wait.elapsed() > timeout {
            panic!("Test timed out waiting for ingestion");
        }

        let progress;
        match LambdaContext::get_progress(&progress_id).await {
            Ok(p) => progress = p,
            Err(e) => {
                // It might take a moment for progress to be available
                sleep(Duration::from_millis(10)).await;
                continue;
            }
        }

        if let Some(p) = progress {
            if !started {
                // Start timing from the first received progress
                step_start_time = Instant::now();
                last_step = p.current_step.clone();
                started = true;
                println!("First progress update: {:?}", last_step);
            }

            if p.current_step != last_step {
                let duration = step_start_time.elapsed();
                step_durations.insert(format!("{:?}", last_step), duration);
                println!("Step {:?} took {:?}", last_step, duration);

                last_step = p.current_step.clone();
                step_start_time = Instant::now();
            }

            if p.is_complete || p.is_failed {
                let duration = step_start_time.elapsed();
                step_durations.insert(format!("{:?}", last_step), duration);
                println!("Step {:?} took {:?}", last_step, duration);

                if p.is_failed {
                    println!("Ingestion FAILED: {}", p.status_message);
                } else {
                    println!("Ingestion COMPLETED in {:?}", start_total.elapsed());
                }
                break;
            }
        }

        sleep(Duration::from_millis(10)).await;
    }

    // 5. Report Findings
    println!("\n=== PERFORMANCE REPORT ===");
    for (step, duration) in &step_durations {
        println!("{}: {:.2?}", step, duration);
    }

    // Check if mutation related steps are slow
    // IngestionSteps are: ValidatingConfig, PreparingSchemas, FlatteningData, GettingAIRecommendation, SettingUpSchema, GeneratingMutations, ExecutingMutations

    let gen_time = step_durations
        .get(&format!("{:?}", IngestionStep::GeneratingMutations))
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let exec_time = step_durations
        .get(&format!("{:?}", IngestionStep::ExecutingMutations))
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    println!("Generating Mutations Time: {:.4}s", gen_time);
    println!("Executing Mutations Time: {:.4}s", exec_time);

    // Cleanup
    std::fs::remove_dir_all(temp_dir).ok();
}
