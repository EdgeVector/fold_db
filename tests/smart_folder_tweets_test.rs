//! Integration test: SmartFolder ingestion of Twitter export + post-ingestion query.
//!
//! Distinct from the other two Twitter/SmartFolder tests:
//!   - `ingest_tweets_test`: directly ingests tweets.js, no query phase
//!   - `smart_folder_ingestion_test`: SmartFolder pipeline on sample_data, no tweets
//!
//! This test exercises both together:
//!   1. `perform_smart_folder_scan` on a temp folder containing tweets.js
//!   2. Asserts tweets.js is recommended for ingestion
//!   3. Ingests via the library API (matching the HTTP route's internal path)
//!   4. Runs an AI agent query to verify the tweet records are queryable
//!
//! Requires: FOLD_OPENROUTER_API_KEY (or OPENROUTER_API_KEY)
//! Run with: `cargo test --test smart_folder_tweets_test -- --ignored --nocapture`

use fold_db::fold_node::llm_query::LlmQueryService;
use fold_db::fold_node::node::FoldNode;
use fold_db::fold_node::OperationProcessor;
use fold_db::ingestion::ingestion_service::IngestionService;
use fold_db::ingestion::smart_folder::{perform_smart_folder_scan, read_file_with_hash};
use fold_db::ingestion::{create_progress_tracker, IngestionConfig, IngestionRequest, ProgressService};
use fold_db::logging::core::run_with_user;
use fold_db::schema_service::server::{
    AddSchemaResponse, AvailableSchemasResponse, ConflictResponse, ErrorResponse, SchemaAddOutcome,
    SchemaServiceState, SchemasListResponse,
};
use fold_db::security::Ed25519KeyPair;
use fold_db::testing_utils::TestDatabaseFactory;

use actix_web::{web, App, HttpResponse, HttpServer};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::Path;
use tempfile::TempDir;

// ── Inline schema service handlers ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AddSchemaRequest {
    schema: fold_db::schema::types::Schema,
    mutation_mappers: HashMap<String, String>,
}

async fn handle_list_schemas(state: web::Data<SchemaServiceState>) -> HttpResponse {
    match state.get_schema_names() {
        Ok(names) => HttpResponse::Ok().json(SchemasListResponse { schemas: names }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to list schemas: {}", e),
        }),
    }
}

async fn handle_get_available_schemas(state: web::Data<SchemaServiceState>) -> HttpResponse {
    match state.get_all_schemas_cached() {
        Ok(schemas) => HttpResponse::Ok().json(AvailableSchemasResponse { schemas }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get schemas: {}", e),
        }),
    }
}

async fn handle_get_schema(
    path: web::Path<String>,
    state: web::Data<SchemaServiceState>,
) -> HttpResponse {
    let schema_name = path.into_inner();
    match state.get_schema_by_name(&schema_name) {
        Ok(Some(schema)) => HttpResponse::Ok().json(schema),
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: "Schema not found".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get schema: {}", e),
        }),
    }
}

async fn handle_add_schema(
    payload: web::Json<AddSchemaRequest>,
    state: web::Data<SchemaServiceState>,
) -> HttpResponse {
    let request = payload.into_inner();
    match state
        .add_schema(request.schema, request.mutation_mappers)
        .await
    {
        Ok(SchemaAddOutcome::Added(schema, mutation_mappers)) => {
            HttpResponse::Created().json(AddSchemaResponse {
                schema,
                mutation_mappers,
            })
        }
        Ok(SchemaAddOutcome::AlreadyExists(schema)) => {
            HttpResponse::Ok().json(AddSchemaResponse {
                schema,
                mutation_mappers: HashMap::new(),
            })
        }
        Ok(SchemaAddOutcome::TooSimilar(conflict)) => {
            HttpResponse::Conflict().json(ConflictResponse {
                error: "Schema too similar to existing schema".to_string(),
                similarity: conflict.similarity,
                closest_schema: conflict.closest_schema,
            })
        }
        Err(error) => HttpResponse::BadRequest().json(ErrorResponse {
            error: format!("Failed to add schema: {}", error),
        }),
    }
}

// ── Test helpers ─────────────────────────────────────────────────────────────

async fn spawn_local_schema_service() -> (String, actix_web::dev::ServerHandle, TempDir) {
    let temp_dir = TempDir::new().expect("failed to create tempdir for schema service");
    let db_path = temp_dir
        .path()
        .join("test_schema_db")
        .to_string_lossy()
        .to_string();

    let state = SchemaServiceState::new(db_path).expect("failed to create schema service state");
    let state_data = web::Data::new(state);

    let listener =
        TcpListener::bind(("127.0.0.1", 0)).expect("failed to bind schema service listener");
    let bound_address = listener.local_addr().expect("failed to read bound address");

    let state_clone = state_data.clone();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(state_clone.clone())
            .service(
                web::scope("/api")
                    .route("/schemas", web::get().to(handle_list_schemas))
                    .route("/schemas", web::post().to(handle_add_schema))
                    .route(
                        "/schemas/available",
                        web::get().to(handle_get_available_schemas),
                    )
                    .route("/schema/{name}", web::get().to(handle_get_schema)),
            )
    })
    .listen(listener)
    .expect("failed to listen")
    .run();

    let handle = server.handle();
    actix_web::rt::spawn(server);
    actix_web::rt::time::sleep(std::time::Duration::from_millis(100)).await;

    let base_url = format!("http://127.0.0.1:{}", bound_address.port());
    (base_url, handle, temp_dir)
}

// ── Integration test ─────────────────────────────────────────────────────────

#[actix_web::test]
#[ignore] // Requires FOLD_OPENROUTER_API_KEY and AI calls
async fn test_smart_folder_tweets_ingest_and_query() {
    // Guard: fail loudly if no API key (test is #[ignore] — must be opted in explicitly)
    std::env::var("FOLD_OPENROUTER_API_KEY")
        .or_else(|_| std::env::var("OPENROUTER_API_KEY"))
        .expect("FOLD_OPENROUTER_API_KEY or OPENROUTER_API_KEY must be set to run this test");

    // Verify the tweets.js fixture is present
    let tweets_fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tweets.js");
    assert!(
        tweets_fixture.exists(),
        "tweets.js fixture not found at {}",
        tweets_fixture.display()
    );

    // ── Setup ────────────────────────────────────────────────────────────────

    let (schema_url, schema_handle, _schema_temp_dir) = spawn_local_schema_service().await;
    eprintln!("Schema service: {}", schema_url);

    let mut config = TestDatabaseFactory::create_test_node_config();
    let keypair = Ed25519KeyPair::generate().unwrap();
    let user_id = keypair.public_key_base64();
    config = config
        .with_identity(&user_id, &keypair.secret_key_base64())
        .with_schema_service_url(&schema_url);
    let node = FoldNode::new(config).await.unwrap();

    let ingestion_service =
        IngestionService::from_env().expect("Failed to create ingestion service");
    let progress_tracker = create_progress_tracker(None).await;
    let progress_service = ProgressService::new(progress_tracker);

    // ── Phase 1: SmartFolder Scan ────────────────────────────────────────────
    //
    // Copy tweets.js into a dedicated temp folder so the scan sees exactly
    // one file and the recommendation is unambiguous.

    let scan_dir = TempDir::new().expect("Failed to create scan temp dir");
    let tweets_in_scan_dir = scan_dir.path().join("tweets.js");
    std::fs::copy(&tweets_fixture, &tweets_in_scan_dir)
        .expect("Failed to copy tweets.js to scan dir");

    eprintln!("Scanning: {}", scan_dir.path().display());
    let scan = perform_smart_folder_scan(
        scan_dir.path(),
        2,   // max_depth
        10,  // max_files
        Some(&ingestion_service),
        Some(&node),
    )
    .await
    .expect("Smart folder scan failed");

    eprintln!(
        "Scan: total={} recommended={} skipped={}",
        scan.total_files,
        scan.recommended_files.len(),
        scan.skipped_files.len()
    );
    for f in &scan.recommended_files {
        eprintln!("  recommend: {} ({})", f.path, f.category);
    }
    for f in &scan.skipped_files {
        eprintln!("  skip: {} ({})", f.path, f.category);
    }

    assert!(scan.success, "Scan should succeed");
    assert!(
        scan.total_files >= 1,
        "Scan should find tweets.js"
    );

    let tweets_rec = scan
        .recommended_files
        .iter()
        .find(|f| f.path.contains("tweets.js"))
        .expect("tweets.js must appear in recommended_files — the LLM should classify personal Twitter exports as worth ingesting");

    eprintln!(
        "tweets.js: category={} should_ingest={} estimated_cost={:.4}",
        tweets_rec.category, tweets_rec.should_ingest, tweets_rec.estimated_cost
    );
    assert!(
        tweets_rec.should_ingest,
        "tweets.js should be marked for ingestion (category={})",
        tweets_rec.category
    );

    // ── Phase 2: Ingest tweets.js ────────────────────────────────────────────

    let progress_id = "test-smart-tweets".to_string();

    let (json_data, file_hash, _raw_bytes) =
        read_file_with_hash(&tweets_in_scan_dir).expect("Failed to read tweets.js");

    let item_count = json_data.as_array().map(|a| a.len()).unwrap_or(1);
    eprintln!("tweets.js parsed: {} top-level items", item_count);

    let request = IngestionRequest {
        data: json_data,
        auto_execute: true,
        pub_key: user_id.clone(),
        source_file_name: Some("tweets.js".to_string()),
        progress_id: Some(progress_id.clone()),
        file_hash: Some(file_hash),
        source_folder: Some(scan_dir.path().to_string_lossy().to_string()),
        image_descriptive_name: None,
    };

    let pid = progress_id.clone();
    let result = run_with_user(&user_id, async {
        ingestion_service
            .process_json_with_node_and_progress(request, &node, &progress_service, pid)
            .await
    })
    .await;

    let response = result.expect("Ingestion should succeed");
    eprintln!(
        "Ingestion: success={} schema={:?} mutations_generated={} mutations_executed={}",
        response.success,
        response.schema_used,
        response.mutations_generated,
        response.mutations_executed
    );
    if !response.errors.is_empty() {
        eprintln!("  warnings: {:?}", response.errors);
    }

    assert!(response.success, "Ingestion should succeed");
    assert!(
        response.mutations_generated > 0,
        "Should generate at least one mutation from tweets"
    );
    assert!(
        response.mutations_executed > 0,
        "Should execute at least one mutation"
    );
    assert!(
        response.schema_used.is_some(),
        "Should have created or used a schema"
    );

    // ── Phase 3: Query ingested tweet data ───────────────────────────────────

    let ingestion_config =
        IngestionConfig::from_env().expect("IngestionConfig::from_env failed");
    let query_service =
        LlmQueryService::new(ingestion_config).expect("Failed to create LlmQueryService");

    let processor = OperationProcessor::new(node.clone());
    let schemas = processor
        .list_schemas()
        .await
        .expect("Failed to list schemas");

    eprintln!(
        "Schemas available: {}",
        schemas.iter().map(|s| s.name()).collect::<Vec<_>>().join(", ")
    );
    assert!(!schemas.is_empty(), "At least one schema should exist after ingestion");

    let (answer, tool_calls) = query_service
        .run_agent_query(
            "What are my tweets about? Give me a summary of the topics.",
            &schemas,
            &node,
            &user_id,
            10,
        )
        .await
        .expect("AI query should succeed");

    eprintln!("Tool calls: {}", tool_calls.len());
    for tc in &tool_calls {
        let result_str = tc.result.to_string();
        let result_preview: String = result_str.chars().take(120).collect();
        eprintln!("  {} -> {}", tc.tool, result_preview);
    }
    let answer_preview: String = answer.chars().take(400).collect();
    eprintln!("Answer (first 400 chars): {}", answer_preview);

    assert!(
        !answer.trim().is_empty(),
        "Query answer should not be empty"
    );

    // The agent must have called at least one query tool to read the data
    assert!(
        !tool_calls.is_empty(),
        "Agent should have issued at least one tool call to read tweet records"
    );

    // Soft keyword check: answer should reference tweet-related concepts
    let answer_lower = answer.to_lowercase();
    let tweet_keywords = [
        "tweet", "twitter", "post", "retweet", "mention", "hashtag",
        "social", "message", "thread",
    ];
    let matched = tweet_keywords.iter().any(|kw| answer_lower.contains(kw));
    assert!(
        matched,
        "Query answer should reference tweet-related content; keywords {:?} not found in: {}",
        tweet_keywords,
        answer.chars().take(400).collect::<String>()
    );

    // ── Cleanup ──────────────────────────────────────────────────────────────
    schema_handle.stop(true).await;
    eprintln!("Test complete.");
}
