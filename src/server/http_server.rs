use super::routes::log as log_routes;
use super::routes::{
    query as query_routes, schema as schema_routes, security as security_routes,
    system as system_routes,
};
use crate::datafold_node::llm_query;
use crate::datafold_node::DataFoldNode;
use crate::error::{FoldDbError, FoldDbResult};
use crate::ingestion::create_progress_tracker;
use crate::ingestion::routes as ingestion_routes;
use crate::utils::http_errors;

use crate::log_feature;
use crate::logging::features::LogFeature;
use actix_cors::Cors;
use actix_files::Files;

use actix_web::{web, App, HttpResponse, HttpServer as ActixHttpServer};
use std::sync::Arc;
use tokio::sync::RwLock;

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
    node: Arc<tokio::sync::RwLock<DataFoldNode>>,
    /// The HTTP server bind address
    bind_address: String,
}



/// Shared application state for the HTTP server.
pub struct AppState {
    /// The DataFold node
    pub(crate) node: Arc<tokio::sync::RwLock<DataFoldNode>>,
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
        // Extract DynamoDB logs config from node if using DynamoDB backend
        let logs_config = {
            match &node.config.database {
                #[cfg(feature = "aws-backend")]
                crate::datafold_node::config::DatabaseConfig::DynamoDb(d) => {
                    let user_id = d
                        .user_id
                        .clone()
                        .unwrap_or_else(|| node.get_node_public_key().to_string());
                    Some((d.tables.logs.clone(), d.region.clone(), Some(user_id)))
                }
                _ => None,
            }
        };

        // Initialize the enhanced logging system with DynamoDB config if available
        match crate::logging::LoggingSystem::init_with_dynamodb(logs_config).await {
            Ok(_) => {}
            Err(crate::logging::LoggingError::AlreadyInitialized) => {
                // Logging system already initialized, which is expected if running from binary
            }
            Err(e) => {
                log_feature!(
                    LogFeature::HttpServer,
                    warn,
                    "Failed to initialize enhanced logging system, falling back to web logger: {}",
                    e
                );
                // Fall back to default logging
                crate::logging::LoggingSystem::init_default().await.ok();
            }
        }

        Ok(Self {
            node: Arc::new(RwLock::new(node)),
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
        self.load_schemas_if_configured().await?;

        // Initialize upload storage from environment config
        let upload_storage_config =
            crate::storage::config::UploadStorageConfig::from_env().unwrap_or_default();

        let upload_storage = match upload_storage_config {
            crate::storage::config::UploadStorageConfig::Local { path } => {
                crate::storage::UploadStorage::local(path)
            }
        };

        log_feature!(
            LogFeature::HttpServer,
            info,
            "Upload storage initialized: {}",
            if upload_storage.is_local() {
                "Local"
            } else {
                "S3"
            }
        );

        // Create individual dependencies for ingestion routes
        let node = web::Data::new(self.node.clone());

        // Extract DynamoDB config for progress tracker if available
        let dynamo_config = {
            let node_guard = self.node.read().await;
            match &node_guard.config.database {
                #[cfg(feature = "aws-backend")]
                crate::datafold_node::config::DatabaseConfig::DynamoDb(d) => {
                    Some((d.tables.process.clone(), d.region.clone()))
                }
                _ => None,
            }
        };

        let progress_tracker_data = web::Data::new(create_progress_tracker(dynamo_config).await);
        let upload_storage_data = web::Data::new(upload_storage.clone());

        // Create shared application state for routes that still need it
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
                .app_data(node.clone())
                .app_data(progress_tracker_data.clone())
                .app_data(upload_storage_data.clone())
                .app_data(json_config)
                .configure(Self::configure_api)
                // Serve static files from the React app build directory
                // This must be last to allow API routes to take precedence
                .service(Files::new("/", "./src/server/static-react/dist").index_file("index.html"))
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

    async fn load_schemas_if_configured(&self) -> FoldDbResult<()> {
        // Load schemas from schema service if configured
        let schema_service_url = {
            let node_guard = self.node.read().await;
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
                    let node_guard = self.node.read().await;
                    let db_guard = node_guard.get_fold_db().await?;
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
        Ok(())
    }

    fn configure_api(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/api")
                .configure(Self::configure_openapi_routes)
                .configure(Self::configure_schema_routes)
                .configure(Self::configure_query_routes)
                .configure(Self::configure_ingestion_routes)
                .configure(Self::configure_log_routes)
                .configure(Self::configure_system_routes)
                .configure(Self::configure_llm_query_routes)
                .configure(Self::configure_security_routes),
        );
    }

    fn configure_openapi_routes(cfg: &mut web::ServiceConfig) {
        cfg.route(
            "/openapi.json",
            web::get().to(|| async move {
                let doc = crate::server::openapi::build_openapi();
                HttpResponse::Ok()
                    .content_type("application/json")
                    .body(doc)
            }),
        );
    }

    fn configure_schema_routes(cfg: &mut web::ServiceConfig) {
        cfg.route("/schemas", web::get().to(schema_routes::list_schemas))
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
            .route(
                "/backfill/{hash}",
                web::get().to(schema_routes::get_backfill_status),
            );
    }

    fn configure_query_routes(cfg: &mut web::ServiceConfig) {
        cfg.route("/query", web::post().to(query_routes::execute_query))
            .route("/mutation", web::post().to(query_routes::execute_mutation))
            .route(
                "/mutations/batch",
                web::post().to(query_routes::execute_mutations_batch),
            )
            .route("/transforms", web::get().to(query_routes::list_transforms))
            .route(
                "/transforms/queue",
                web::get().to(query_routes::get_transform_queue),
            )
            .route(
                "/transforms/queue/{id}",
                web::post().to(query_routes::add_to_transform_queue),
            )
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
            .route(
                "/native-index/search",
                web::get().to(query_routes::native_index_search),
            )
            .route(
                "/indexing/status",
                web::get().to(query_routes::get_indexing_status),
            );
    }

    fn configure_ingestion_routes(cfg: &mut web::ServiceConfig) {
        cfg.route(
            "/ingestion/process",
            web::post().to(ingestion_routes::process_json),
        )
        .route(
            "/ingestion/upload",
            web::post().to(crate::ingestion::file_upload::upload_file),
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
        .route(
            "/ingestion/progress",
            web::get().to(ingestion_routes::get_all_progress),
        )
        .route(
            "/ingestion/progress/{id}",
            web::get().to(ingestion_routes::get_progress),
        );
    }

    fn configure_log_routes(cfg: &mut web::ServiceConfig) {
        cfg.route("/logs", web::get().to(log_routes::list_logs))
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
            );
    }

    fn configure_system_routes(cfg: &mut web::ServiceConfig) {
        cfg.route(
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
        .route(
            "/system/reset-schema-service",
            web::post().to(system_routes::reset_schema_service),
        )
        .route(
            "/system/database-config",
            web::get().to(system_routes::get_database_config),
        )
        .route(
            "/system/database-config",
            web::post().to(system_routes::update_database_config),
        );
    }

    fn configure_llm_query_routes(cfg: &mut web::ServiceConfig) {
        cfg.route("/llm-query/run", web::post().to(llm_query::run_query))
            .route(
                "/llm-query/analyze",
                web::post().to(llm_query::analyze_query),
            )
            .route(
                "/llm-query/execute",
                web::post().to(llm_query::execute_query_plan),
            )
            .route("/llm-query/chat", web::post().to(llm_query::chat))
            .route(
                "/llm-query/analyze-followup",
                web::post().to(llm_query::analyze_followup),
            )
            .route(
                "/llm-query/native-index",
                web::post().to(llm_query::ai_native_index_query),
            )
            .route(
                "/llm-query/backfill/{hash}",
                web::get().to(llm_query::get_backfill_status),
            );
    }

    fn configure_security_routes(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/security").service(
                web::resource("/system-key")
                    .route(web::get().to(security_routes::get_system_public_key)),
            ),
        );
    }
}



