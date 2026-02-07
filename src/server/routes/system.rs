use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::security::Ed25519KeyPair;
use crate::server::http_server::AppState;
use crate::server::routes::{handler_error_to_response, require_node, require_user_context};
use crate::storage::config::DatabaseConfig;
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

/// Get system status information
#[utoipa::path(
    get,
    path = "/api/system/status",
    tag = "system",
    responses(
        (status = 200, description = "System status", body = serde_json::Value)
    )
)]
pub async fn get_system_status(state: web::Data<AppState>) -> impl Responder {
    let (user_hash, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let node = node_arc.read().await;

    match crate::handlers::system::get_system_status(&user_hash, &node).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => handler_error_to_response(e),
    }
}

/// Get the node's private key
///
/// This endpoint returns the node's private key for use by the UI.
/// The private key is generated automatically when the node is created.
#[utoipa::path(
    get,
    path = "/api/system/private-key",
    tag = "system",
    responses(
        (status = 200, description = "Node private key", body = serde_json::Value)
    )
)]
pub async fn get_node_private_key(state: web::Data<AppState>) -> impl Responder {
    let (user_hash, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let node = node_arc.read().await;

    match crate::handlers::system::get_node_private_key(&user_hash, &node).await {
        Ok(response) => {
            log_feature!(
                LogFeature::HttpServer,
                info,
                "Node private key retrieved successfully"
            );
            HttpResponse::Ok().json(json!({
                "success": response.data.as_ref().map(|d| d.success).unwrap_or(false),
                "private_key": response.data.as_ref().map(|d| &d.key),
                "message": response.data.as_ref().map(|d| &d.message)
            }))
        }
        Err(e) => handler_error_to_response(e),
    }
}

/// Get the node's public key
///
/// This endpoint returns the node's public key for verification purposes.
/// The public key is generated automatically when the node is created.
#[utoipa::path(
    get,
    path = "/api/system/public-key",
    tag = "system",
    responses(
        (status = 200, description = "Node public key", body = serde_json::Value)
    )
)]
pub async fn get_node_public_key(state: web::Data<AppState>) -> impl Responder {
    let (user_hash, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let node = node_arc.read().await;

    match crate::handlers::system::get_node_public_key(&user_hash, &node).await {
        Ok(response) => {
            log_feature!(
                LogFeature::HttpServer,
                info,
                "Node public key retrieved successfully"
            );
            HttpResponse::Ok().json(json!({
                "success": response.data.as_ref().map(|d| d.success).unwrap_or(false),
                "public_key": response.data.as_ref().map(|d| &d.key),
                "message": response.data.as_ref().map(|d| &d.message)
            }))
        }
        Err(e) => handler_error_to_response(e),
    }
}

/// Request body for database reset
#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub struct ResetDatabaseRequest {
    pub confirm: bool,
}

/// Response for database reset (async job)
#[derive(Serialize, utoipa::ToSchema)]
pub struct ResetDatabaseResponse {
    pub success: bool,
    pub message: String,
    /// Job ID for tracking progress (only present when async)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

/// Response for schema service reset
#[derive(Serialize, utoipa::ToSchema)]
pub struct ResetSchemaServiceResponse {
    pub success: bool,
    pub message: String,
}

/// Reset the database (async background job)
///
/// This endpoint initiates a database reset as a background job:
/// 1. Returns immediately with a job ID for progress tracking
/// 2. The background job clears all data for the current user
/// 3. Progress can be monitored via /api/ingestion/progress/{job_id}
///
/// This is a destructive operation that cannot be undone.
///
/// # Multi-Tenancy Support
///
/// This endpoint respects multi-tenancy by only clearing data for the
/// current user (identified via x-user-hash header). It uses the scan-free
/// DynamoDbResetManager to efficiently delete data partitioned by user.
#[utoipa::path(
    post,
    path = "/api/system/reset-database",
    tag = "system",
    request_body = ResetDatabaseRequest,
    responses(
        (status = 202, description = "Database reset job started", body = ResetDatabaseResponse),
        (status = 400, description = "Bad request", body = ResetDatabaseResponse),
        (status = 500, description = "Server error", body = ResetDatabaseResponse)
    )
)]
pub async fn reset_database(
    state: web::Data<AppState>,
    progress_tracker: web::Data<crate::progress::ProgressTracker>,
    req: web::Json<ResetDatabaseRequest>,
) -> impl Responder {
    use crate::progress::{Job, JobType};

    // Require explicit confirmation
    if !req.confirm {
        return HttpResponse::BadRequest().json(ResetDatabaseResponse {
            success: false,
            message: "Reset confirmation required. Set 'confirm' to true.".to_string(),
            job_id: None,
        });
    }

    // Get user ID from context (required for multi-tenancy)
    let user_id = match require_user_context() {
        Ok(hash) => hash,
        Err(response) => return response,
    };

    // Generate a unique job ID
    let job_id = format!("reset_{}", uuid::Uuid::new_v4());

    // Create the job entry
    let mut job = Job::new(job_id.clone(), JobType::Other("database_reset".to_string()));
    job = job.with_user(user_id.clone());
    job.update_progress(5, "Initializing database reset...".to_string());

    // Save initial job state
    if let Err(e) = progress_tracker.save(&job).await {
        log_feature!(
            LogFeature::HttpServer,
            error,
            "Failed to create reset job: {}",
            e
        );
        return HttpResponse::InternalServerError().json(ResetDatabaseResponse {
            success: false,
            message: format!("Failed to create reset job: {}", e),
            job_id: None,
        });
    }

    // Clone dependencies for the background task
    let node_manager_clone = state.node_manager.clone();
    let tracker_clone = progress_tracker.clone();
    let job_id_clone = job_id.clone();
    let user_id_clone = user_id.clone();

    // Spawn the background reset task
    tokio::spawn(async move {
        // Set user context for the background task
        crate::logging::core::run_with_user(&user_id_clone.clone(), async move {
            // Update progress: Clearing DynamoDB tables
            if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                job.update_progress(10, "Clearing user data from storage...".to_string());
                let _ = tracker_clone.save(&job).await;
            }

            // Get node from NodeManager for this user
            let node_arc = match node_manager_clone.get_node(&user_id_clone).await {
                Ok(n) => n,
                Err(e) => {
                    log_feature!(
                        LogFeature::HttpServer,
                        error,
                        "Failed to get node for reset: {}",
                        e
                    );
                    if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                        job.fail(format!("Failed to get node: {}", e));
                        let _ = tracker_clone.save(&job).await;
                    }
                    return;
                }
            };

            // Create processor
            let temp_processor_node = node_arc.read().await.clone();
            let processor = crate::datafold_node::OperationProcessor::new(temp_processor_node);

            // Step 2: Perform the storage reset
            if let Err(e) = processor.perform_database_reset(Some(&user_id_clone)).await {
                log_feature!(
                    LogFeature::HttpServer,
                    error,
                    "Database reset failed: {}",
                    e
                );
                if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                    job.fail(format!("Database reset failed: {}", e));
                    let _ = tracker_clone.save(&job).await;
                }
                return;
            }

            // Step 3: Invalidate the cached node so it gets re-created on next access
            node_manager_clone.invalidate_node(&user_id_clone).await;

            log_feature!(
                LogFeature::HttpServer,
                info,
                "Database reset completed successfully for user: {}",
                user_id_clone
            );

            // Mark job as complete
            if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                job.complete(Some(serde_json::json!({
                    "user_id": user_id_clone,
                    "message": "Database reset successfully. All data has been cleared."
                })));
                let _ = tracker_clone.save(&job).await;
            }
        })
        .await;
    });

    // Return immediately with accepted status and job ID
    HttpResponse::Accepted().json(ResetDatabaseResponse {
        success: true,
        message: "Database reset started. Monitor progress via /api/ingestion/progress endpoint."
            .to_string(),
        job_id: Some(job_id),
    })
}

/// Reset the schema service database
///
/// This endpoint resets the schema service database by calling its reset endpoint.
/// This is useful when schemas need to be recreated with updated topology inference.
#[utoipa::path(
    post,
    path = "/api/system/reset-schema-service",
    tag = "system",
    request_body = ResetDatabaseRequest,
    responses(
        (status = 200, description = "Schema service reset result", body = ResetSchemaServiceResponse),
        (status = 400, description = "Bad request", body = ResetSchemaServiceResponse),
        (status = 500, description = "Server error", body = ResetSchemaServiceResponse)
    )
)]
pub async fn reset_schema_service(
    state: web::Data<AppState>,
    req: web::Json<ResetDatabaseRequest>,
) -> impl Responder {
    // Require explicit confirmation
    if !req.confirm {
        return HttpResponse::BadRequest().json(ResetSchemaServiceResponse {
            success: false,
            message: "Reset confirmation required. Set 'confirm' to true.".to_string(),
        });
    }

    // Get the schema service client from the node via NodeManager
    let (_user_hash, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let node = node_arc.read().await;
    let schema_client = node.get_schema_client();

    // Call the schema service reset endpoint
    match schema_client.reset_schema_service().await {
        Ok(()) => {
            log_feature!(
                LogFeature::HttpServer,
                info,
                "Schema service database reset completed successfully"
            );
            HttpResponse::Ok().json(ResetSchemaServiceResponse {
                success: true,
                message:
                    "Schema service database reset successfully. All schemas have been cleared."
                        .to_string(),
            })
        }
        Err(e) => {
            log_feature!(
                LogFeature::HttpServer,
                error,
                "Schema service reset failed: {}",
                e
            );
            HttpResponse::InternalServerError().json(ResetSchemaServiceResponse {
                success: false,
                message: format!("Schema service reset failed: {}", e),
            })
        }
    }
}

/// Database configuration request/response types
#[derive(Deserialize, Serialize, utoipa::ToSchema, Debug, Clone)]
pub struct DatabaseConfigRequest {
    pub database: DatabaseConfigDto,
}

#[derive(Deserialize, Serialize, utoipa::ToSchema, Debug, Clone)]
#[serde(tag = "type")]
pub enum DatabaseConfigDto {
    #[serde(rename = "local")]
    Local { path: String },
    #[cfg(feature = "aws-backend")]
    #[serde(rename = "cloud", alias = "dynamodb")]
    Cloud(Box<CloudConfigDto>),
}

/// DTO for ExplicitTables
#[derive(Deserialize, Serialize, utoipa::ToSchema, Debug, Clone, Default)]
pub struct ExplicitTablesDto {
    pub main: String,
    pub metadata: String,
    pub permissions: String,
    pub transforms: String,
    pub orchestrator: String,
    pub schema_states: String,
    pub schemas: String,
    pub public_keys: String,
    pub transform_queue: String,
    pub native_index: String,
    pub process: String,
    pub logs: String,
}

/// DTO for CloudConfig (formerly DynamoDbConfig)
#[cfg(feature = "aws-backend")]
#[derive(Deserialize, Serialize, utoipa::ToSchema, Debug, Clone)]
pub struct CloudConfigDto {
    pub region: String,
    /// Explicit table names for all required namespaces
    pub tables: ExplicitTablesDto,
    pub auto_create: bool,
    pub user_id: Option<String>,
    pub file_storage_bucket: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DatabaseConfigResponse {
    pub success: bool,
    pub message: String,
    pub requires_restart: bool,
}

/// Get current database configuration
#[utoipa::path(
    get,
    path = "/api/system/database-config",
    tag = "system",
    responses(
        (status = 200, description = "Database configuration", body = DatabaseConfigDto)
    )
)]
pub async fn get_database_config(state: web::Data<AppState>) -> impl Responder {
    // Get the base configuration from NodeManager (not per-user)
    let config = state.node_manager.get_base_config();

    let db_config = match &config.database {
        DatabaseConfig::Local { path } => DatabaseConfigDto::Local {
            path: path.to_string_lossy().to_string(),
        },
        #[cfg(feature = "aws-backend")]
        DatabaseConfig::Cloud(config) => DatabaseConfigDto::Cloud(Box::new(CloudConfigDto {
            region: config.region.clone(),
            auto_create: config.auto_create,
            user_id: config.user_id.clone(),
            file_storage_bucket: config.file_storage_bucket.clone(),
            tables: ExplicitTablesDto {
                main: config.tables.main.clone(),
                metadata: config.tables.metadata.clone(),
                permissions: config.tables.permissions.clone(),
                transforms: config.tables.transforms.clone(),
                orchestrator: config.tables.orchestrator.clone(),
                schema_states: config.tables.schema_states.clone(),
                schemas: config.tables.schemas.clone(),
                public_keys: config.tables.public_keys.clone(),
                transform_queue: config.tables.transform_queue.clone(),
                native_index: config.tables.native_index.clone(),
                process: config.tables.process.clone(),
                logs: config.tables.logs.clone(),
            },
        })),
    };

    HttpResponse::Ok().json(db_config)
}

/// Get a default auto-generated identity for local development.
///
/// This endpoint returns a deterministic identity derived from the node's
/// public key. It does NOT require authentication, allowing the frontend
/// to auto-authenticate without a login step.
#[utoipa::path(
    get,
    path = "/api/system/auto-identity",
    tag = "system",
    responses(
        (status = 200, description = "Default identity for auto-login", body = serde_json::Value)
    )
)]
pub async fn auto_identity() -> impl Responder {
    // Generate a deterministic keypair from a fixed seed
    let seed = Sha256::digest(b"local_default_user");
    let keypair = match Ed25519KeyPair::from_secret_key(seed.as_slice()) {
        Ok(kp) => kp,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "ok": false,
                "error": format!("Failed to generate identity: {}", e)
            }));
        }
    };

    let public_key = keypair.public_key_base64();

    // Derive user_hash = SHA256(public_key)[0:32] (same algorithm as frontend)
    let hash = Sha256::digest(public_key.as_bytes());
    let user_hash: String = hash.iter().map(|b| format!("{:02x}", b)).collect::<String>()[..32].to_string();

    HttpResponse::Ok().json(json!({
        "user_id": public_key,
        "user_hash": user_hash,
        "public_key": public_key,
    }))
}

/// Update database configuration
///
/// This endpoint updates the database configuration in the node config file.
/// The server must be restarted for the changes to take effect.
#[utoipa::path(
    post,
    path = "/api/system/database-config",
    tag = "system",
    request_body = DatabaseConfigRequest,
    responses(
        (status = 200, description = "Configuration updated", body = DatabaseConfigResponse),
        (status = 400, description = "Bad request", body = DatabaseConfigResponse),
        (status = 500, description = "Server error", body = DatabaseConfigResponse)
    )
)]
pub async fn update_database_config(
    _state: web::Data<AppState>,
    _req: web::Json<DatabaseConfigRequest>,
) -> impl Responder {
    // NOTE: Dynamic database config updates are not supported in multi-tenant mode
    // The database configuration is set at startup and affects all users.
    // To change the database configuration, update the config file and restart the server.
    HttpResponse::BadRequest().json(DatabaseConfigResponse {
        success: false,
        message: "Dynamic database configuration updates are not supported. Please update the configuration file and restart the server.".to_string(),
        requires_restart: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datafold_node::{DataFoldNode, NodeConfig};
    use crate::server::node_manager::{NodeManager, NodeManagerConfig};
    use actix_web::test;
    use std::sync::Arc;
    use tempfile::tempdir;

    async fn create_test_state(temp_dir: &tempfile::TempDir) -> web::Data<AppState> {
        let keypair = crate::security::Ed25519KeyPair::generate().unwrap();
        let config = NodeConfig::new(temp_dir.path().to_path_buf())
            .with_schema_service_url("test://mock")
            .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());
        let node = DataFoldNode::new(config.clone()).await.unwrap();

        // Create NodeManager and pre-populate with test node
        let node_manager_config = NodeManagerConfig {
            base_config: config,
        };
        let node_manager = NodeManager::new(node_manager_config);
        node_manager.set_node("test_user", node).await;

        web::Data::new(AppState {
            node_manager: Arc::new(node_manager),
        })
    }

    #[tokio::test]
    async fn test_system_status() {
        let temp_dir = tempdir().unwrap();
        let state = create_test_state(&temp_dir).await;

        // Need to run with user context since routes now require authentication
        crate::logging::core::run_with_user("test_user", async move {
            let req = test::TestRequest::get().to_http_request();
            let resp = get_system_status(state).await.respond_to(&req);
            assert_eq!(resp.status(), 200);
        })
        .await;
    }

    #[tokio::test]
    async fn test_get_node_private_key() {
        let temp_dir = tempdir().unwrap();
        let state = create_test_state(&temp_dir).await;

        crate::logging::core::run_with_user("test_user", async move {
            let req = test::TestRequest::get().to_http_request();
            let resp = get_node_private_key(state).await.respond_to(&req);
            assert_eq!(resp.status(), 200);

            // Parse the response to verify it contains the private key
            let body = resp.into_body();
            let bytes = actix_web::body::to_bytes(body).await.unwrap_or_default();
            let response: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();

            assert!(response["success"].as_bool().unwrap_or(false));
            assert!(response["private_key"].as_str().is_some());
            assert!(!response["private_key"].as_str().unwrap_or("").is_empty());
        })
        .await;
    }

    #[tokio::test]
    async fn test_get_node_public_key() {
        let temp_dir = tempdir().unwrap();
        let state = create_test_state(&temp_dir).await;

        crate::logging::core::run_with_user("test_user", async move {
            let req = test::TestRequest::get().to_http_request();
            let resp = get_node_public_key(state).await.respond_to(&req);
            assert_eq!(resp.status(), 200);

            // Parse the response to verify it contains the public key
            let body = resp.into_body();
            let bytes = actix_web::body::to_bytes(body).await.unwrap_or_default();
            let response: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();

            assert!(response["success"].as_bool().unwrap_or(false));
            assert!(response["public_key"].as_str().is_some());
            assert!(!response["public_key"].as_str().unwrap_or("").is_empty());
        })
        .await;
    }

    #[tokio::test]
    async fn test_private_and_public_keys_are_different() {
        let temp_dir = tempdir().unwrap();
        let state = create_test_state(&temp_dir).await;

        crate::logging::core::run_with_user("test_user", async move {
            // Get private key
            let req1 = test::TestRequest::get().to_http_request();
            let resp1 = get_node_private_key(state.clone()).await.respond_to(&req1);
            let body1 = resp1.into_body();
            let bytes1 = actix_web::body::to_bytes(body1).await.unwrap_or_default();
            let response1: serde_json::Value = serde_json::from_slice(&bytes1).unwrap_or_default();
            let private_key = response1["private_key"].as_str().unwrap_or("").to_string();

            // Get public key
            let req2 = test::TestRequest::get().to_http_request();
            let resp2 = get_node_public_key(state).await.respond_to(&req2);
            let body2 = resp2.into_body();
            let bytes2 = actix_web::body::to_bytes(body2).await.unwrap_or_default();
            let response2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap_or_default();
            let public_key = response2["public_key"].as_str().unwrap_or("").to_string();

            // Verify they are different
            assert_ne!(private_key, public_key);
            assert!(!private_key.is_empty());
            assert!(!public_key.is_empty());
        })
        .await;
    }

    #[tokio::test]
    async fn test_reset_database_without_confirmation() {
        let temp_dir = tempdir().unwrap();
        let state = create_test_state(&temp_dir).await;
        let progress_tracker = web::Data::new(crate::progress::create_tracker(None).await);

        let req_body = ResetDatabaseRequest { confirm: false };
        let req = test::TestRequest::post()
            .set_json(&req_body)
            .to_http_request();

        let resp = reset_database(state, progress_tracker, web::Json(req_body))
            .await
            .respond_to(&req);
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn test_reset_database_with_confirmation() {
        let temp_dir = tempdir().unwrap();
        let state = create_test_state(&temp_dir).await;
        let progress_tracker = web::Data::new(crate::progress::create_tracker(None).await);

        crate::logging::core::run_with_user("test_user", async move {
            let req_body = ResetDatabaseRequest { confirm: true };
            let req = test::TestRequest::post()
                .set_json(&req_body)
                .to_http_request();

            let resp = reset_database(state, progress_tracker, web::Json(req_body))
                .await
                .respond_to(&req);
            // The response should be 202 (Accepted) for async job started, or 500 for internal error
            assert!(resp.status() == 202 || resp.status() == 500);
        })
        .await;
    }
}
