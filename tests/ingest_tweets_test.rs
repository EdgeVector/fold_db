//! Integration test for ingesting a real Twitter export (tweets.js).
//!
//! This test exercises the full ingestion pipeline:
//! 1. Parse tweets.js (Twitter export format with JS prefix)
//! 2. AI-powered schema recommendation
//! 3. Schema submission to local schema service
//! 4. Mutation generation and execution
//!
//! Requires:
//! - `FOLD_OPENROUTER_API_KEY` environment variable set
//! - tests/fixtures/tweets.js file present
//!
//! Run with: `cargo test --test ingest_tweets_test -- --ignored --nocapture`

use fold_db::fold_node::node::FoldNode;
use fold_db::ingestion::ingestion_service::IngestionService;
use fold_db::ingestion::smart_folder::read_file_as_json;
use fold_db::ingestion::{create_progress_tracker, IngestionRequest, ProgressService};
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

/// Spawn a local schema service on a random port, returning (base_url, server_handle).
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
    let bound_address = listener.local_addr().expect("failed to read bound address");

    let state_clone = state_data.clone();
    let server = HttpServer::new(move || {
        App::new().app_data(state_clone.clone()).service(
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
    // Give the server a moment to start
    actix_web::rt::time::sleep(std::time::Duration::from_millis(100)).await;

    let base_url = format!("http://127.0.0.1:{}", bound_address.port());
    (base_url, handle, temp_dir)
}

// -- Integration test --------------------------------------------------------

#[actix_web::test]
#[ignore] // Requires FOLD_OPENROUTER_API_KEY and AI calls
async fn test_ingest_tweets_js() {
    // Skip if no API key
    if std::env::var("FOLD_OPENROUTER_API_KEY").is_err()
        && std::env::var("OPENROUTER_API_KEY").is_err()
    {
        eprintln!("Skipping: FOLD_OPENROUTER_API_KEY not set");
        return;
    }

    // Check that the fixture file exists
    let tweets_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tweets.js");
    if !tweets_path.exists() {
        eprintln!(
            "Skipping: tweets.js fixture not found at {}",
            tweets_path.display()
        );
        return;
    }

    // 1. Spin up a local schema service
    let (schema_url, schema_handle, _schema_temp_dir) = spawn_local_schema_service().await;
    eprintln!("Local schema service running at {}", schema_url);

    // 2. Create FoldNode with the local schema service
    let mut config = TestDatabaseFactory::create_test_node_config();
    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let user_id = keypair.public_key_base64();
    config = config
        .with_identity(&user_id, &keypair.secret_key_base64())
        .with_schema_service_url(&schema_url);
    let node = FoldNode::new(config).await.unwrap();

    // 3. Parse tweets.js
    let tweet_data = read_file_as_json(&tweets_path).expect("Failed to parse tweets.js");
    eprintln!(
        "Parsed tweets.js: {} top-level items",
        tweet_data.as_array().map(|a| a.len()).unwrap_or(1)
    );

    // 4. Create IngestionService from environment
    let ingestion_service =
        IngestionService::from_env().expect("Failed to create ingestion service");

    // 5. Create progress tracking
    let progress_tracker = create_progress_tracker(None).await;
    let progress_service = ProgressService::new(progress_tracker);

    // 6. Build ingestion request
    let request = IngestionRequest {
        data: tweet_data,
        auto_execute: true,
        trust_distance: 0,
        pub_key: user_id.clone(),
        source_file_name: Some("tweets.js".to_string()),
        progress_id: Some("test-tweets-ingestion".to_string()),
        file_hash: None,
        source_folder: None,
        image_descriptive_name: None,
    };

    // 7. Run ingestion within user context
    let response = run_with_user(&user_id, async {
        ingestion_service
            .process_json_with_node_and_progress(
                request,
                &node,
                &progress_service,
                "test-tweets-ingestion".to_string(),
            )
            .await
    })
    .await;

    match &response {
        Ok(resp) => {
            eprintln!("Ingestion response:");
            eprintln!("  success: {}", resp.success);
            eprintln!("  schema_used: {:?}", resp.schema_used);
            eprintln!("  new_schema_created: {}", resp.new_schema_created);
            eprintln!("  mutations_generated: {}", resp.mutations_generated);
            eprintln!("  mutations_executed: {}", resp.mutations_executed);
            eprintln!("  errors: {:?}", resp.errors);
        }
        Err(e) => {
            eprintln!("Ingestion failed: {}", e);
        }
    }

    let response = response.expect("Ingestion should succeed");

    // 8. Assert results
    assert!(response.success, "Ingestion should succeed");
    assert!(
        response.mutations_generated > 0,
        "Should generate at least one mutation"
    );
    assert!(
        response.mutations_executed > 0,
        "Should execute at least one mutation"
    );
    assert!(response.schema_used.is_some(), "Should have used a schema");

    eprintln!(
        "Successfully ingested tweets.js: {} mutations generated, {} executed",
        response.mutations_generated, response.mutations_executed
    );

    // Cleanup
    schema_handle.stop(true).await;
}
