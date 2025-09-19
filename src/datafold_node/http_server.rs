use super::log_routes;
use super::{network_routes, query_routes, schema_routes, security_routes, system_routes};
use crate::datafold_node::DataFoldNode;
use crate::error::{FoldDbError, FoldDbResult};
use crate::error_handling::http_errors;
use crate::ingestion::routes as ingestion_routes;

use crate::log_feature;
use crate::logging::features::LogFeature;
use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpServer as ActixHttpServer};
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

        // Create shared application state
        let app_state = web::Data::new(AppState {
            node: self.node.clone(),
        });

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
                .app_data(json_config)
                .service(
                    web::scope("/api")
                        // Schema endpoints
                        .route("/schemas", web::get().to(schema_routes::list_schemas))
                        .route(
                            "/schemas/status",
                            web::get().to(schema_routes::get_schema_status),
                        )
                        .route(
                            "/schemas/refresh",
                            web::post().to(schema_routes::refresh_schemas),
                        )
                        .route(
                            "/schemas/available",
                            web::get().to(schema_routes::list_available_schemas),
                        )
                        .route(
                            "/schemas/available/add",
                            web::post().to(schema_routes::add_schema_to_available),
                        )
                        .route(
                            "/schemas/by-state/{state}",
                            web::get().to(schema_routes::list_schemas_by_state),
                        )
                        .route("/schema/{name}", web::get().to(schema_routes::get_schema))
                        .route("/schema", web::post().to(schema_routes::create_schema))
                        .route(
                            "/schema/{name}",
                            web::delete().to(schema_routes::unload_schema_route),
                        )
                        .route(
                            "/schema/{name}/load",
                            web::post().to(schema_routes::load_schema_route),
                        )
                        .route(
                            "/schema/{name}/approve",
                            web::post().to(schema_routes::approve_schema),
                        )
                        .route(
                            "/schema/{name}/block",
                            web::post().to(schema_routes::block_schema),
                        )
                        .route(
                            "/schema/{name}/state",
                            web::get().to(schema_routes::get_schema_state),
                        )
                        // Operation endpoints
                        .route("/execute", web::post().to(query_routes::execute_operation))
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
                            web::get().to(ingestion_routes::get_config),
                        )
                        .route(
                            "/ingestion/validate",
                            web::post().to(ingestion_routes::validate_json),
                        )
                        .route(
                            "/ingestion/openrouter-config",
                            web::get().to(ingestion_routes::get_openrouter_config),
                        )
                        .route(
                            "/ingestion/openrouter-config",
                            web::post().to(ingestion_routes::save_openrouter_config),
                        )
                        // Transform endpoints
                        .route("/transforms", web::get().to(query_routes::list_transforms))
                        .route(
                            "/transform/{id}/run",
                            web::post().to(query_routes::run_transform),
                        )
                        .route(
                            "/transforms/queue",
                            web::get().to(query_routes::get_transform_queue),
                        )
                        .route(
                            "/transforms/queue/{id}",
                            web::post().to(query_routes::add_to_transform_queue),
                        )
                        .route(
                            "/transforms/reload",
                            web::post().to(query_routes::reload_transforms),
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
                        // Security endpoints
                        .service(
                            web::scope("/security")
                                .service(
                                    web::resource("/system-key")
                                        .route(
                                            web::post()
                                                .to(security_routes::register_system_public_key),
                                        )
                                        .route(
                                            web::get().to(security_routes::get_system_public_key),
                                        )
                                        .route(
                                            web::delete()
                                                .to(security_routes::remove_system_public_key),
                                        ),
                                )
                                .route("/verify", web::post().to(security_routes::verify_message))
                                .route(
                                    "/status",
                                    web::get().to(security_routes::get_security_status),
                                )
                                .route(
                                    "/examples",
                                    web::get().to(security_routes::get_client_examples),
                                )
                                .route(
                                    "/demo-keypair",
                                    web::get().to(security_routes::generate_demo_keypair),
                                )
                                .route(
                                    "/protected",
                                    web::post().to(security_routes::protected_endpoint),
                                ),
                        )
                        // Network endpoints
                        .service(
                            web::scope("/network")
                                .route("/init", web::post().to(network_routes::init_network))
                                .route("/start", web::post().to(network_routes::start_network))
                                .route("/stop", web::post().to(network_routes::stop_network))
                                .route("/status", web::get().to(network_routes::get_network_status))
                                .route("/connect", web::post().to(network_routes::connect_to_node))
                                .route("/discover", web::post().to(network_routes::discover_nodes))
                                .route("/nodes", web::get().to(network_routes::list_nodes)),
                        ),
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

#[cfg(test)]
mod tests {
    use super::DataFoldHttpServer;
    use crate::datafold_node::{DataFoldNode, NodeConfig};
    use std::net::TcpListener;
    use tempfile::tempdir;

    /// Test the new unified schema status endpoint
    #[tokio::test]
    async fn test_unified_schema_status() {
        let temp_dir = tempdir().unwrap();
        let config = NodeConfig::new(temp_dir.path().to_path_buf());
        let node = DataFoldNode::load(config).await.unwrap();

        // pick an available port
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let bind_addr = format!("127.0.0.1:{}", addr.port());

        let server = DataFoldHttpServer::new(node, &bind_addr)
            .await
            .expect("server init");

        let handle = tokio::spawn(async move { server.run().await.unwrap() });

        // Wait for server to start
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Test new unified schema status endpoint
        let client = reqwest::Client::new();
        let url = format!("http://{}/api/schemas/status", bind_addr);

        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .expect("Failed to connect to server");

        assert!(response.status().is_success());

        let json_value: serde_json::Value = response
            .json()
            .await
            .expect("Failed to parse JSON response");

        // Verify response structure
        assert!(json_value.get("data").is_some());

        handle.abort();
        let _ = handle.await;
    }

    /// Verify that logs endpoint returns data
    #[tokio::test]
    async fn logs_endpoint_returns_lines() {
        let temp_dir = tempdir().unwrap();
        let config = NodeConfig::new(temp_dir.path().to_path_buf());
        let node = DataFoldNode::new(config).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let bind_addr = format!("127.0.0.1:{}", addr.port());

        let server = DataFoldHttpServer::new(node, &bind_addr)
            .await
            .expect("server init");

        let handle = tokio::spawn(async move { server.run().await.unwrap() });

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let client = reqwest::Client::new();
        let url = format!("http://{}/api/logs", bind_addr);

        let response: serde_json::Value = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .expect("request failed")
            .json()
            .await
            .expect("invalid json");

        // Verify the response structure
        assert!(response.is_object());
        assert!(response.get("logs").is_some());
        assert!(response.get("count").is_some());
        assert!(response.get("timestamp").is_some());

        // Check that logs field is an array (may be empty initially)
        let logs_array = response.get("logs").unwrap().as_array().unwrap();

        // Verify count field matches array length
        let count = response.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count as usize, logs_array.len());

        handle.abort();
        let _ = handle.await;
    }
}
