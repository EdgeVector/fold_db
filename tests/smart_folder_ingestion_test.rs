//! Integration test for smart folder ingestion + AI query.
//!
//! This test exercises the full smart folder pipeline:
//! 1. Scan sample_data/ with AI classification
//! 2. Ingest text-based files (json, csv, txt, md) via the library API
//! 3. Verify process results have correct keys (no empty keys)
//! 4. Run AI agent queries against the ingested data
//!
//! Requires:
//! - `FOLD_OPENROUTER_API_KEY` environment variable set
//!
//! Run with: `cargo test --test smart_folder_ingestion_test -- --ignored --nocapture`

use fold_db::fold_node::llm_query::LlmQueryService;
use fold_db::fold_node::node::FoldNode;
use fold_db::fold_node::OperationProcessor;
use fold_db::ingestion::ingestion_service::IngestionService;
use fold_db::ingestion::smart_folder::{perform_smart_folder_scan, read_file_with_hash};
use fold_db::ingestion::{create_progress_tracker, IngestionConfig, IngestionRequest, ProgressService};
use fold_db::logging::core::run_with_user;
use fold_db::schema_service::server::{
    AddSchemaResponse, ConflictResponse, ErrorResponse, SchemaAddOutcome, SchemaServiceState,
    SchemasListResponse,
};
use fold_db::testing_utils::TestDatabaseFactory;

use actix_web::{web, App, HttpResponse, HttpServer};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::Path;
use tempfile::TempDir;

// -- Inline schema service handlers (the real ones are module-private) --------

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
        Ok(schemas) => HttpResponse::Ok().json(serde_json::json!({ "schemas": schemas })),
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

// -- Test helpers ------------------------------------------------------------

/// Spawn a local schema service on a random port.
/// The TempDir must be kept alive for the duration of the test.
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
    let bound_address = listener
        .local_addr()
        .expect("failed to read bound address");

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

/// Text-based file extensions we can ingest without external converters.
const TEXT_EXTENSIONS: &[&str] = &["json", "csv", "txt", "md"];

fn is_text_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    TEXT_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

// -- Integration test --------------------------------------------------------

#[actix_web::test]
#[ignore] // Requires FOLD_OPENROUTER_API_KEY and AI calls
async fn test_smart_folder_ingest_and_query() {
    // ── Phase 1: Setup ──────────────────────────────────────────────────

    // Guard: skip if no API key
    if std::env::var("FOLD_OPENROUTER_API_KEY").is_err()
        && std::env::var("OPENROUTER_API_KEY").is_err()
    {
        eprintln!("Skipping: FOLD_OPENROUTER_API_KEY not set");
        return;
    }

    // Spin up local schema service
    let (schema_url, schema_handle, _schema_temp_dir) = spawn_local_schema_service().await;
    eprintln!("Local schema service running at {}", schema_url);

    // Create FoldNode
    let mut config = TestDatabaseFactory::create_test_node_config();
    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let user_id = keypair.public_key_base64();
    config = config
        .with_identity(&user_id, &keypair.secret_key_base64())
        .with_schema_service_url(&schema_url);
    let node = FoldNode::new(config).await.unwrap();

    // Create IngestionService + ProgressService
    let ingestion_service =
        IngestionService::from_env().expect("Failed to create ingestion service");
    let progress_tracker = create_progress_tracker(None).await;
    let progress_service = ProgressService::new(progress_tracker);

    // ── Phase 2: Scan ───────────────────────────────────────────────────

    let sample_data_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("sample_data");
    assert!(
        sample_data_path.exists(),
        "sample_data/ directory not found at {}",
        sample_data_path.display()
    );

    eprintln!("Scanning: {}", sample_data_path.display());
    let scan = perform_smart_folder_scan(
        &sample_data_path,
        5,
        500,
        Some(&ingestion_service),
        Some(&node),
    )
    .await
    .expect("Smart folder scan failed");

    eprintln!("Scan results:");
    eprintln!("  success: {}", scan.success);
    eprintln!("  total_files: {}", scan.total_files);
    eprintln!("  recommended_files: {}", scan.recommended_files.len());
    eprintln!("  skipped_files: {}", scan.skipped_files.len());

    assert!(scan.success, "Scan should succeed");
    assert!(scan.total_files > 0, "Should find at least one file");
    assert!(
        !scan.recommended_files.is_empty(),
        "Should recommend at least one file"
    );

    // Filter to text-only files
    let text_files: Vec<_> = scan
        .recommended_files
        .iter()
        .filter(|f| is_text_file(&f.path))
        .collect();

    eprintln!(
        "Text files to ingest: {} (of {} recommended)",
        text_files.len(),
        scan.recommended_files.len()
    );
    for f in &text_files {
        eprintln!("  - {} ({})", f.path, f.category);
    }

    assert!(
        !text_files.is_empty(),
        "Should have at least one text file to ingest"
    );

    // ── Phase 3: Ingest ─────────────────────────────────────────────────

    struct IngestedFile {
        file_name: String,
        progress_id: String,
        success: bool,
    }

    let mut ingested: Vec<IngestedFile> = Vec::new();

    for (idx, rec) in text_files.iter().enumerate() {
        let full_path = sample_data_path.join(&rec.path);
        let file_name = rec.path.clone();
        let progress_id = format!("test-smart-{}", idx);

        eprintln!("\nIngesting [{}/{}]: {}", idx + 1, text_files.len(), file_name);

        // Read file
        let (json_data, file_hash, _raw_bytes) = match read_file_with_hash(&full_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("  SKIP (read error): {}", e);
                ingested.push(IngestedFile {
                    file_name,
                    progress_id,
                    success: false,
                });
                continue;
            }
        };

        // Build request
        let request = IngestionRequest {
            data: json_data,
            auto_execute: true,
            trust_distance: 0,
            pub_key: user_id.clone(),
            source_file_name: Some(file_name.clone()),
            progress_id: Some(progress_id.clone()),
            file_hash: Some(file_hash),
            source_folder: Some(sample_data_path.to_string_lossy().to_string()),
            image_descriptive_name: None,
        };

        // Run ingestion
        let pid = progress_id.clone();
        let result = run_with_user(&user_id, async {
            ingestion_service
                .process_json_with_node_and_progress(request, &node, &progress_service, pid)
                .await
        })
        .await;

        match &result {
            Ok(resp) => {
                eprintln!(
                    "  OK: schema={:?} mutations_gen={} mutations_exec={} new_schema={}",
                    resp.schema_used,
                    resp.mutations_generated,
                    resp.mutations_executed,
                    resp.new_schema_created
                );
                if !resp.errors.is_empty() {
                    eprintln!("  warnings: {:?}", resp.errors);
                }
                ingested.push(IngestedFile {
                    file_name,
                    progress_id,
                    success: resp.success && resp.mutations_executed > 0,
                });
            }
            Err(e) => {
                eprintln!("  FAIL: {}", e);
                ingested.push(IngestedFile {
                    file_name,
                    progress_id,
                    success: false,
                });
            }
        }
    }

    let success_count = ingested.iter().filter(|f| f.success).count();
    let total_attempted = ingested.len();
    let success_rate = if total_attempted > 0 {
        (success_count as f64) / (total_attempted as f64)
    } else {
        0.0
    };
    eprintln!(
        "\nIngestion summary: {}/{} succeeded ({:.0}%)",
        success_count,
        total_attempted,
        success_rate * 100.0
    );

    assert!(
        success_rate >= 0.5,
        "At least 50% of text files should ingest successfully, got {:.0}% ({}/{})",
        success_rate * 100.0,
        success_count,
        total_attempted
    );

    // ── Phase 4: Verify Process Results ─────────────────────────────────

    // Wait for async ProcessResultsSubscriber to flush
    actix_web::rt::time::sleep(std::time::Duration::from_millis(500)).await;

    let mut total_outcomes = 0usize;
    let mut empty_key_count = 0usize;

    for file in ingested.iter().filter(|f| f.success) {
        let outcomes = node
            .get_process_results(&file.progress_id)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "get_process_results('{}') failed for {}: {}",
                    file.progress_id, file.file_name, e
                );
            });

        eprintln!(
            "Process results for '{}': {} outcomes",
            file.file_name,
            outcomes.len()
        );

        for outcome in &outcomes {
            let has_key = outcome.key_value.hash.is_some() || outcome.key_value.range.is_some();
            eprintln!(
                "  mutation={} schema={} key={}",
                &outcome.mutation_id[..8.min(outcome.mutation_id.len())],
                outcome.schema_name,
                outcome.key_value
            );

            assert!(
                !outcome.schema_name.is_empty(),
                "schema_name must not be empty for mutation {} in {}",
                outcome.mutation_id,
                file.file_name
            );

            if !has_key {
                empty_key_count += 1;
                eprintln!(
                    "  WARNING: empty key_value for mutation {} in {}",
                    outcome.mutation_id, file.file_name
                );
            }
        }

        total_outcomes += outcomes.len();
    }

    eprintln!(
        "\nProcess results summary: {} total outcomes, {} with empty keys",
        total_outcomes, empty_key_count
    );

    assert!(
        total_outcomes > 0,
        "Should have at least one process result outcome across all ingested files"
    );

    assert_eq!(
        empty_key_count, 0,
        "No process results should have empty keys (found {})",
        empty_key_count
    );

    // ── Phase 5: AI Agent Queries ───────────────────────────────────────

    let ingestion_config = IngestionConfig::from_env().expect("IngestionConfig::from_env failed");
    let query_service =
        LlmQueryService::new(ingestion_config).expect("Failed to create LlmQueryService");

    let processor = OperationProcessor::new(node.clone());
    let schemas = processor
        .list_schemas()
        .await
        .expect("Failed to list schemas");

    eprintln!(
        "\nSchemas available for querying: {}",
        schemas
            .iter()
            .map(|s| s.name())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Define queries with soft-check keywords
    let queries: Vec<(&str, Vec<&str>)> = vec![
        (
            "What medications are in the database?",
            vec!["medication", "lisinopril", "vitamin", "metformin"],
        ),
        (
            "What are the bank transactions or financial records?",
            vec!["bank", "transaction", "finance", "payment", "deposit"],
        ),
        (
            "Who are the contacts in the address book?",
            vec!["contact", "mom", "dad", "sarah", "address", "phone"],
        ),
    ];

    let mut queries_succeeded = 0usize;
    let mut queries_failed = 0usize;

    for (query_text, keywords) in &queries {
        eprintln!("\nAI Query: \"{}\"", query_text);

        let result = query_service
            .run_agent_query(query_text, &schemas, &node, &user_id, 10)
            .await;

        match result {
            Ok((answer, tool_calls)) => {
                eprintln!("  Tool calls: {}", tool_calls.len());
                for tc in &tool_calls {
                    eprintln!("    - {} -> {}", tc.tool, tc.result);
                }
                eprintln!("  Answer: {}", &answer[..200.min(answer.len())]);

                // Hard assert: answer is non-empty
                assert!(
                    !answer.trim().is_empty(),
                    "AI query '{}' returned empty answer",
                    query_text
                );

                // Soft check: answer contains at least one expected keyword
                let answer_lower = answer.to_lowercase();
                let matched = keywords.iter().any(|kw| answer_lower.contains(kw));
                if !matched {
                    eprintln!(
                        "  WARNING: Answer for '{}' did not contain any of {:?}",
                        query_text, keywords
                    );
                } else {
                    eprintln!("  Keyword match: PASS");
                }

                queries_succeeded += 1;
            }
            Err(e) => {
                // AI agent responses are non-deterministic; malformed LLM output
                // (e.g., wrong JSON structure) should not fail the whole test.
                eprintln!("  WARNING (non-deterministic): {}", e);
                queries_failed += 1;
            }
        }
    }

    eprintln!(
        "\nAI query summary: {}/{} succeeded, {} failed",
        queries_succeeded,
        queries.len(),
        queries_failed
    );

    assert!(
        queries_succeeded >= 1,
        "At least 1 AI query should succeed, but all {} failed",
        queries.len()
    );

    // ── Cleanup ─────────────────────────────────────────────────────────
    schema_handle.stop(true).await;
    eprintln!("\nTest complete.");
}
