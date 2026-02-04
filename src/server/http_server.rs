use super::middleware::auth::UserContextMiddleware;
use super::node_manager::NodeManager;
use super::routes::log as log_routes;
use super::routes::{
    query as query_routes, schema as schema_routes, security as security_routes,
    system as system_routes,
};
use crate::datafold_node::llm_query;
use crate::datafold_node::DataFoldNode;
use crate::error::{FoldDbError, FoldDbResult};
use crate::ingestion::routes as ingestion_routes;
use crate::utils::http_errors;

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
/// UI, and provides REST API endpoints for schemas, queries, and mutations.
///
/// # Architecture
///
/// The server now uses a lazy per-user node initialization pattern:
/// - On startup: Only configuration is loaded, no DynamoDB access
/// - On first request for a user: Node is created with user context
/// - Subsequent requests: Node is cached and reused
///
/// This aligns with Lambda's multi-tenant architecture.
///
/// # Features
///
/// * Static file serving for the UI
/// * REST API endpoints for schemas, queries, and mutations
/// * Sample data management
/// * One-click loading of sample data
pub struct DataFoldHttpServer {
    /// The node manager for lazy per-user node creation
    node_manager: Arc<NodeManager>,
    /// The HTTP server bind address
    bind_address: String,
}

/// Shared application state for the HTTP server.
pub struct AppState {
    /// The node manager for getting per-user nodes
    pub(crate) node_manager: Arc<NodeManager>,
}

impl DataFoldHttpServer {
    /// Create a new HTTP server.
    ///
    /// This method creates a new HTTP server that listens on the specified address.
    /// It uses the provided NodeManager to create per-user nodes lazily.
    ///
    /// # Arguments
    ///
    /// * `node_manager` - The NodeManager instance for creating per-user nodes
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
    pub async fn new(node_manager: NodeManager, bind_address: &str) -> FoldDbResult<Self> {
        // Extract DynamoDB logs config from base config if using DynamoDB backend
        let logs_config = {
            match &node_manager.get_base_config().database {
                #[cfg(feature = "aws-backend")]
                crate::datafold_node::config::DatabaseConfig::Cloud(d) => {
                    // Note: user_id is NOT set here - it comes from per-request headers
                    Some((d.tables.logs.clone(), d.region.clone(), None))
                }
                _ => None,
            }
        };

        // Initialize the enhanced logging system with Cloud config if available
        crate::logging::LoggingSystem::init_with_fallback(logs_config).await;

        Ok(Self {
            node_manager: Arc::new(node_manager),
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
        // Load schemas from schema service if configured
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

        // Create shared application state
        let app_state = web::Data::new(AppState {
            node_manager: self.node_manager.clone(),
        });

        // Create upload storage data
        let upload_storage_data = web::Data::new(upload_storage.clone());

        // Create LLM query state (gracefully handles missing configuration)
        let llm_query_state = web::Data::new(llm_query::LlmQueryState::new());

        // Create progress tracker based on database config
        let progress_tracker = {
            #[cfg(feature = "aws-backend")]
            {
                if let crate::datafold_node::config::DatabaseConfig::Cloud(cloud_config) =
                    &self.node_manager.get_base_config().database
                {
                    crate::progress::create_tracker(Some((
                        cloud_config.tables.process.clone(),
                        cloud_config.region.clone(),
                    )))
                    .await
                } else {
                    crate::progress::create_tracker(None).await
                }
            }
            #[cfg(not(feature = "aws-backend"))]
            {
                crate::progress::create_tracker(None).await
            }
        };
        let progress_tracker_data = web::Data::new(progress_tracker);

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
                .wrap(UserContextMiddleware)
                .app_data(app_state.clone())
                .app_data(llm_query_state.clone())
                .app_data(upload_storage_data.clone())
                .app_data(progress_tracker_data.clone())
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
        let schema_service_url = self
            .node_manager
            .get_base_config()
            .schema_service_url
            .clone();

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

                // For schema loading, we need a temporary node
                // Schemas are global, so we use a system context
                let client = crate::datafold_node::SchemaServiceClient::new(&url);

                match client.list_schemas().await {
                    Ok(schemas) => {
                        log_feature!(
                            LogFeature::Database,
                            info,
                            "Loaded {} schemas from schema service",
                            schemas.len()
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
                "/transforms/backfills/{id}",
                web::get().to(query_routes::get_backfill),
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
            "/ingestion/config",
            web::get().to(ingestion_routes::get_ingestion_config),
        )
        .route(
            "/ingestion/config",
            web::post().to(ingestion_routes::save_ingestion_config),
        )
        .route(
            "/ingestion/progress",
            web::get().to(ingestion_routes::get_all_progress),
        )
        .route(
            "/ingestion/progress/{id}",
            web::get().to(ingestion_routes::get_progress),
        )
        .route(
            "/ingestion/batch-folder",
            web::post().to(ingestion_routes::batch_folder_ingest),
        );
    }

    fn configure_log_routes(cfg: &mut web::ServiceConfig) {
        cfg.route("/logs", web::get().to(log_routes::list_logs))
            .route("/logs/stream", web::get().to(log_routes::stream_logs));
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
            "/system/database-config",
            web::get().to(system_routes::get_database_config),
        )
        .route(
            "/system/database-config",
            web::post().to(system_routes::update_database_config),
        );
    }

    fn configure_llm_query_routes(cfg: &mut web::ServiceConfig) {
        cfg.route(
            "/llm-query/native-index",
            web::post().to(llm_query::ai_native_index_query),
        )
        .route("/llm-query/run", web::post().to(llm_query::run_query))
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
            "/llm-query/backfill/{hash}",
            web::get().to(llm_query::get_backfill_status),
        )
        .route(
            "/llm-query/agent",
            web::post().to(llm_query::agent_query),
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

// Helper function to get a node for a request
// This is used by route handlers
pub async fn get_node_for_request(
    app_state: &web::Data<AppState>,
    user_id: &str,
) -> Result<Arc<Mutex<DataFoldNode>>, FoldDbError> {
    app_state
        .node_manager
        .get_node(user_id)
        .await
        .map_err(|e| FoldDbError::Config(format!("Failed to get node for user: {}", e)))
}
