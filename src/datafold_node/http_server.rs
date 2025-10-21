use super::log_routes;
use super::llm_query;
use super::{query_routes, schema_routes, security_routes, system_routes};
use crate::datafold_node::DataFoldNode;
use crate::error::{FoldDbError, FoldDbResult};
use crate::error_handling::http_errors;
use crate::ingestion::routes as ingestion_routes;

use crate::log_feature;
use crate::logging::features::LogFeature;
use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer as ActixHttpServer};
use std::sync::Arc;
use tokio::sync::Mutex;

/// HTTP server for the DataFold node.
///
/// DataFoldHttpServer provides a web-based interface for external clients to interact
/// with a DataFold node. It handles HTTP requests and can serve the built React
/// UI,
/// and provides REST API endpoints for schemas, queries, and mutations.
///
/// # Features
///
/// * Static file serving for the UI
/// * REST API endpoints for schemas, queries, and mutations
/// * Sample data management
/// * One-click loading of sample data
pub struct DataFoldHttpServer {
    /// The DataFold node
    node: Arc<tokio::sync::Mutex<DataFoldNode>>,
    /// The HTTP server bind address
    bind_address: String,
}

/// Shared application state for the HTTP server.
pub struct AppState {
    /// The DataFold node
    pub(crate) node: Arc<tokio::sync::Mutex<DataFoldNode>>,
}

impl DataFoldHttpServer {
    /// Create a new HTTP server.
    ///
    /// This method creates a new HTTP server that listens on the specified address.
    /// It uses the provided DataFoldNode to process client requests.
    ///
    /// # Arguments
    ///
    /// * `node` - The DataFoldNode instance to use for processing requests
    /// * `bind_address` - The address to bind to (e.g., "127.0.0.1:9001")
    ///
    /// # Returns
    ///
    /// A `FoldDbResult` containing the new DataFoldHttpServer instance.
    ///
    /// # Errors
    ///
    /// Returns a `FoldDbError` if:
    /// * There is an error starting the HTTP server
    pub async fn new(node: DataFoldNode, bind_address: &str) -> FoldDbResult<Self> {
        // Initialize the enhanced logging system
        if let Err(e) = crate::logging::LoggingSystem::init_default().await {
            log_feature!(
                LogFeature::HttpServer,
                warn,
                "Failed to initialize enhanced logging system, falling back to web logger: {}",
                e
            );
            // Fall back to old web logger for backward compatibility
            crate::web_logger::init().ok();
        }

        Ok(Self {
            node: Arc::new(Mutex::new(node)),
            bind_address: bind_address.to_string(),
        })
    }

    /// Run the HTTP server.
    ///
    /// This method starts the HTTP server and begins accepting client connections.
    /// It can serve the compiled React UI and provides REST API endpoints for
    /// schemas, queries, and mutations.
    ///
    /// # Returns
    ///
    /// A `FoldDbResult` indicating success or failure.
    ///
    /// # Errors
    ///
    /// Returns a `FoldDbError` if:
    /// * There is an error binding to the specified address
    /// * There is an error starting the server
    pub async fn run(&self) -> FoldDbResult<()> {
        log_feature!(
            LogFeature::HttpServer,
            info,
            "HTTP server running on {}",
            self.bind_address
        );

        // Load schemas from schema service if configured
        let schema_service_url = {
            let node_guard = self.node.lock().await;
            node_guard.config.schema_service_url.clone()
        };
        
        if let Some(url) = schema_service_url {
            // Skip loading for mock/test schema services
            if url.starts_with("test://") || url.starts_with("mock://") {
                log_feature!(
                    LogFeature::Database,
                    info,
                    "Mock schema service detected ({}). Skipping automatic schema loading. Schemas must be loaded manually in tests.",
                    url
                );
            } else {
                log_feature!(
                    LogFeature::Database,
                    info,
                    "Loading schemas from schema service at {}...",
                    url
                );
                
                let schema_manager = {
                    let node_guard = self.node.lock().await;
                    let db_guard = node_guard.get_fold_db()?;
                    let manager = db_guard.schema_manager.clone();
                    drop(db_guard);
                    drop(node_guard);
                    manager
                };
                
                let client = crate::datafold_node::SchemaServiceClient::new(&url);
                
                match client.load_all_schemas(&schema_manager).await {
                    Ok(loaded_count) => {
                        log_feature!(
                            LogFeature::Database,
                            info,
                            "Loaded {} schemas from schema service",
                            loaded_count
                        );
                    }
                    Err(e) => {
                        log_feature!(
                            LogFeature::Database,
                            error,
                            "Failed to load schemas from schema service: {}. Server will start but no schemas will be available.",
                            e
                        );
                    }
                }
            }
        }

        // Create shared application state
        let app_state = web::Data::new(AppState {
            node: self.node.clone(),
        });

        // Create LLM query state (gracefully handles missing configuration)
        let llm_query_state = web::Data::new(llm_query::LlmQueryState::new());

        // Start the HTTP server
        let server = ActixHttpServer::new(move || {
            // Create CORS middleware
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600);

            // Configure custom JSON error handler
            let json_config =
                web::JsonConfig::default().error_handler(http_errors::json_error_handler);

            App::new()
                .wrap(cors)
                .app_data(app_state.clone())
                .app_data(llm_query_state.clone())
                .app_data(json_config)
                .service(
                    web::scope("/api")
                        // OpenAPI spec endpoint
                        .route(
                            "/openapi.json",
                            web::get().to(|| async move {
                                let doc = crate::datafold_node::openapi::build_openapi();
                                HttpResponse::Ok().content_type("application/json").body(doc)
                            }),
                        )
                        // Schema endpoints
                        .route("/schemas", web::get().to(schema_routes::list_schemas))
                        .route("/schemas/load", web::post().to(schema_routes::load_schemas))
                        .route("/schema/{name}", web::get().to(schema_routes::get_schema))
                        .route(
                            "/schema/{name}/approve",
                            web::post().to(schema_routes::approve_schema),
                        )
                        .route(
                            "/schema/{name}/block",
                            web::post().to(schema_routes::block_schema),
                        )
                        // Backfill endpoints
                        .route("/backfill/{hash}", web::get().to(schema_routes::get_backfill_status))
                        .route("/query", web::post().to(query_routes::execute_query))
                        .route("/mutation", web::post().to(query_routes::execute_mutation))
                        // Ingestion endpoints
                        .route(
                            "/ingestion/process",
                            web::post().to(ingestion_routes::process_json),
                        )
                        .route(
                            "/ingestion/status",
                            web::get().to(ingestion_routes::get_status),
                        )
                        .route(
                            "/ingestion/health",
                            web::get().to(ingestion_routes::health_check),
                        )
                        .route(
                            "/ingestion/config",
                            web::get().to(ingestion_routes::get_ingestion_config),
                        )
                        .route(
                            "/ingestion/config",
                            web::post().to(ingestion_routes::save_ingestion_config),
                        )
                        .route(
                            "/ingestion/validate",
                            web::post().to(ingestion_routes::validate_json),
                        )
                        // Transform endpoints
                        .route("/transforms", web::get().to(query_routes::list_transforms))
                        .route(
                            "/transforms/queue",
                            web::get().to(query_routes::get_transform_queue),
                        )
                        .route(
                            "/transforms/queue/{id}",
                            web::post().to(query_routes::add_to_transform_queue),
                        )
                        // Backfill monitoring endpoints
                        .route(
                            "/transforms/backfills",
                            web::get().to(query_routes::get_all_backfills),
                        )
                        .route(
                            "/transforms/backfills/active",
                            web::get().to(query_routes::get_active_backfills),
                        )
                        .route(
                            "/transforms/backfills/statistics",
                            web::get().to(query_routes::get_backfill_statistics),
                        )
                        .route(
                            "/transforms/backfills/{id}",
                            web::get().to(query_routes::get_backfill),
                        )
                        .route(
                            "/transforms/statistics",
                            web::get().to(query_routes::get_transform_statistics),
                        )
                        // Log endpoints
                        .route("/logs", web::get().to(log_routes::list_logs))
                        .route("/logs/stream", web::get().to(log_routes::stream_logs))
                        .route("/logs/config", web::get().to(log_routes::get_config))
                        .route(
                            "/logs/config/reload",
                            web::post().to(log_routes::reload_config),
                        )
                        .route("/logs/features", web::get().to(log_routes::get_features))
                        .route(
                            "/logs/level",
                            web::put().to(log_routes::update_feature_level),
                        )
                        // System endpoints
                        .route(
                            "/system/status",
                            web::get().to(system_routes::get_system_status),
                        )
                        .route(
                            "/system/private-key",
                            web::get().to(system_routes::get_node_private_key),
                        )
                        .route(
                            "/system/public-key",
                            web::get().to(system_routes::get_node_public_key),
                        )
                        .route(
                            "/system/reset-database",
                            web::post().to(system_routes::reset_database),
                        )
                        // LLM Query endpoints
                        .route(
                            "/llm-query/run",
                            web::post().to(llm_query::run_query),
                        )
                        .route(
                            "/llm-query/analyze",
                            web::post().to(llm_query::analyze_query),
                        )
                        .route(
                            "/llm-query/execute",
                            web::post().to(llm_query::execute_query_plan),
                        )
                        .route(
                            "/llm-query/chat",
                            web::post().to(llm_query::chat),
                        )
                        .route(
                            "/llm-query/backfill/{hash}",
                            web::get().to(llm_query::get_backfill_status),
                        )
                        // Security endpoints
                        .service(
                            web::scope("/security")
                                .service(
                                    web::resource("/system-key")
                                        .route(
                                            web::get().to(security_routes::get_system_public_key),
                                        ),
                                ),
                        )
                )
                // Serve the built React UI if it exists
                .service(
                    Files::new("/", "src/datafold_node/static-react/dist").index_file("index.html"),
                )
        })
        .bind(&self.bind_address)
        .map_err(|e| FoldDbError::Config(format!("Failed to bind HTTP server: {}", e)))?
        .run();

        // Run the server
        server
            .await
            .map_err(|e| FoldDbError::Config(format!("HTTP server error: {}", e)))?;

        Ok(())
    }
}