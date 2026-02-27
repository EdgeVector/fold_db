use crate::fold_node::config::NodeConfig;
use crate::handlers::system::NodeKeyResponse;
use crate::handlers::{ApiResponse, HandlerError};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::security::Ed25519KeyPair;
use crate::server::http_server::AppState;
use crate::server::node_manager::NodeManagerConfig;
use crate::server::routes::{handler_error_to_response, require_node_read, require_user_context};
use crate::storage::config::DatabaseConfig;
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match crate::handlers::system::get_system_status(&user_hash, &node).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => handler_error_to_response(e),
    }
}

/// Shared helper for key retrieval endpoints.
fn key_response(
    result: Result<ApiResponse<NodeKeyResponse>, HandlerError>,
    key_name: &str,
    log_msg: &str,
) -> HttpResponse {
    match result {
        Ok(response) => {
            log_feature!(LogFeature::HttpServer, info, "{}", log_msg);
            HttpResponse::Ok().json(json!({
                "success": response.data.as_ref().map(|d| d.success).unwrap_or(false),
                key_name: response.data.as_ref().map(|d| &d.key),
                "message": response.data.as_ref().map(|d| &d.message)
            }))
        }
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let result = crate::handlers::system::get_node_private_key(&user_hash, &node).await;
    key_response(result, "private_key", "Node private key retrieved successfully")
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let result = crate::handlers::system::get_node_public_key(&user_hash, &node).await;
    key_response(result, "public_key", "Node public key retrieved successfully")
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
            let processor = crate::fold_node::OperationProcessor::new(temp_processor_node);

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

/// Request body for migrating to cloud
#[derive(Deserialize, Serialize, utoipa::ToSchema, Debug, Clone)]
pub struct MigrateToCloudRequest {
    pub api_url: String,
    pub api_key: String,
}

/// Response for migrating to cloud (async job)
#[derive(Serialize, utoipa::ToSchema)]
pub struct MigrateToCloudResponse {
    pub success: bool,
    pub message: String,
    /// Job ID for tracking progress
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

/// Migrate data to Cloud (async background job)
///
/// This endpoint initiates a migration of all schemas and data to a remote XMEM cloud instance:
/// 1. Returns immediately with a job ID for progress tracking
/// 2. The background job reads local data and pushes it to the remote API
/// 3. Progress can be monitored via /api/ingestion/progress/{job_id}
#[utoipa::path(
    post,
    path = "/api/system/migrate-to-cloud",
    tag = "system",
    request_body = MigrateToCloudRequest,
    responses(
        (status = 202, description = "Migration job started", body = MigrateToCloudResponse),
        (status = 400, description = "Bad request", body = MigrateToCloudResponse),
        (status = 500, description = "Server error", body = MigrateToCloudResponse)
    )
)]
pub async fn migrate_to_cloud(
    state: web::Data<AppState>,
    progress_tracker: web::Data<crate::progress::ProgressTracker>,
    req: web::Json<MigrateToCloudRequest>,
) -> impl Responder {
    use crate::progress::{Job, JobType};

    // Get user ID from context
    let user_id = match require_user_context() {
        Ok(hash) => hash,
        Err(response) => return response,
    };

    let api_url = req.api_url.clone();
    let api_key = req.api_key.clone();

    if api_url.is_empty() || api_key.is_empty() {
        return HttpResponse::BadRequest().json(MigrateToCloudResponse {
            success: false,
            message: "api_url and api_key are required.".to_string(),
            job_id: None,
        });
    }

    // Generate a unique job ID
    let job_id = format!("migrate_{}", uuid::Uuid::new_v4());

    // Create the job entry
    let mut job = Job::new(
        job_id.clone(),
        JobType::Other("cloud_migration".to_string()),
    );
    job = job.with_user(user_id.clone());
    job.update_progress(5, format!("Initializing migration to {}...", api_url));

    // Save initial job state
    if let Err(e) = progress_tracker.save(&job).await {
        log_feature!(
            LogFeature::HttpServer,
            error,
            "Failed to create migration job: {}",
            e
        );
        return HttpResponse::InternalServerError().json(MigrateToCloudResponse {
            success: false,
            message: format!("Failed to create migration job: {}", e),
            job_id: None,
        });
    }

    let node_manager_clone = state.node_manager.clone();
    let tracker_clone = progress_tracker.clone();
    let job_id_clone = job_id.clone();
    let user_id_clone = user_id.clone();

    tokio::spawn(async move {
        // Set user context
        crate::logging::core::run_with_user(&user_id_clone.clone(), async move {
            if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                job.update_progress(10, "Fetching local node data...".to_string());
                let _ = tracker_clone.save(&job).await;
            }

            let node_arc = match node_manager_clone.get_node(&user_id_clone).await {
                Ok(n) => n,
                Err(e) => {
                    log_feature!(
                        LogFeature::HttpServer,
                        error,
                        "Failed to get node for migration: {}",
                        e
                    );
                    if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                        job.fail(format!("Failed to get node: {}", e));
                        let _ = tracker_clone.save(&job).await;
                    }
                    return;
                }
            };

            let processor =
                crate::fold_node::OperationProcessor::new(node_arc.read().await.clone());

            if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                job.update_progress(20, "Syncing schemas and documents...".to_string());
                let _ = tracker_clone.save(&job).await;
            }

            if let Err(e) = processor.migrate_to_cloud(&api_url, &api_key).await {
                log_feature!(
                    LogFeature::HttpServer,
                    error,
                    "Cloud migration failed: {}",
                    e
                );
                if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                    job.fail(format!("Cloud migration failed: {}", e));
                    let _ = tracker_clone.save(&job).await;
                }
                return;
            }

            log_feature!(
                LogFeature::HttpServer,
                info,
                "Cloud migration completed for user: {}",
                user_id_clone
            );

            if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                job.complete(Some(serde_json::json!({
                    "user_id": user_id_clone,
                    "message": "Migration completed successfully"
                })));
                let _ = tracker_clone.save(&job).await;
            }
        })
        .await;
    });

    HttpResponse::Accepted().json(MigrateToCloudResponse {
        success: true,
        message: "Cloud migration started. Monitor progress via /api/ingestion/progress endpoint."
            .to_string(),
        job_id: Some(job_id),
    })
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
    #[serde(rename = "exemem")]
    Exemem { api_url: String },
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
    pub idempotency: String,
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
    let config = state.node_manager.get_base_config().await;

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
                idempotency: config.tables.idempotency.clone(),
            },
        })),
        DatabaseConfig::Exemem { api_url, .. } => DatabaseConfigDto::Exemem {
            api_url: api_url.clone(),
        },
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
    let user_hash: String = hash
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()[..32]
        .to_string();

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

/// Request body for system setup (matches CLI setup wizard)
#[derive(Deserialize, Serialize, utoipa::ToSchema, Debug, Clone)]
pub struct SetupRequest {
    /// Storage configuration (optional: only update if provided)
    #[serde(default)]
    pub storage: Option<StorageSetup>,
    /// Schema service URL (optional: only update if provided)
    #[serde(default)]
    pub schema_service_url: Option<String>,
}

/// Storage setup options matching CLI wizard
#[derive(Deserialize, Serialize, utoipa::ToSchema, Debug, Clone)]
#[serde(tag = "type")]
pub enum StorageSetup {
    /// Local Sled storage
    #[serde(rename = "local")]
    Local { path: String },
    /// Exemem cloud storage
    #[serde(rename = "exemem")]
    Exemem { api_url: String, api_key: String },
}

/// Response for setup endpoint
#[derive(Serialize, utoipa::ToSchema)]
pub struct SetupResponse {
    pub success: bool,
    pub message: String,
}

/// Persist a NodeConfig to disk (same path the server loaded from)
fn persist_node_config(config: &NodeConfig) -> Result<(), String> {
    let config_path =
        std::env::var("NODE_CONFIG").unwrap_or_else(|_| "config/node_config.json".to_string());

    // Ensure config directory exists
    if let Some(parent) = std::path::Path::new(&config_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let config_json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_path, config_json)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Apply setup configuration (storage and/or schema service URL)
///
/// This endpoint allows the UI wizard to configure the same settings as the CLI
/// setup wizard. It updates the config, persists it to disk, and invalidates
/// cached nodes so the next request uses the new configuration.
#[utoipa::path(
    post,
    path = "/api/system/setup",
    tag = "system",
    request_body = SetupRequest,
    responses(
        (status = 200, description = "Setup applied successfully", body = SetupResponse),
        (status = 400, description = "Bad request", body = SetupResponse),
        (status = 500, description = "Server error", body = SetupResponse)
    )
)]
pub async fn apply_setup(
    state: web::Data<AppState>,
    req: web::Json<SetupRequest>,
) -> impl Responder {
    // Read current config
    let mut config = state.node_manager.get_base_config().await;

    let mut changes = Vec::new();

    // Apply storage override if provided
    if let Some(ref storage) = req.storage {
        match storage {
            StorageSetup::Local { path } => {
                config.database = DatabaseConfig::Local {
                    path: std::path::PathBuf::from(path),
                };
                changes.push("storage (local)");
            }
            StorageSetup::Exemem { api_url, api_key } => {
                config.database = DatabaseConfig::Exemem {
                    api_url: api_url.clone(),
                    api_key: api_key.clone(),
                };
                changes.push("storage (exemem)");
            }
        }
    }

    // Apply schema_service_url override if provided
    if let Some(ref url) = req.schema_service_url {
        config.schema_service_url = Some(url.clone());
        changes.push("schema service URL");
    }

    if changes.is_empty() {
        return HttpResponse::BadRequest().json(SetupResponse {
            success: false,
            message: "No configuration changes provided".to_string(),
        });
    }

    // Persist to disk
    if let Err(e) = persist_node_config(&config) {
        log_feature!(
            LogFeature::HttpServer,
            error,
            "Failed to persist setup config: {}",
            e
        );
        return HttpResponse::InternalServerError().json(SetupResponse {
            success: false,
            message: format!("Failed to save configuration: {}", e),
        });
    }

    // Update NodeManager config and invalidate all cached nodes
    let new_manager_config = NodeManagerConfig {
        base_config: config,
    };
    state.node_manager.update_config(new_manager_config).await;

    let message = format!("Setup applied: {}", changes.join(", "));
    log_feature!(LogFeature::HttpServer, info, "{}", message);

    HttpResponse::Ok().json(SetupResponse {
        success: true,
        message,
    })
}

/// Request body for filesystem path completion
#[derive(Deserialize)]
pub struct PathCompleteRequest {
    pub partial_path: String,
}

/// Complete a partial filesystem path with matching directories
///
/// This endpoint provides directory-only path completion for the folder picker UI.
/// It lists directories matching a partial path prefix, hiding dotfiles.
pub async fn complete_path(body: web::Json<PathCompleteRequest>) -> impl Responder {
    let partial = &body.partial_path;

    let (parent, prefix) = if partial.ends_with('/') || partial.ends_with('\\') {
        (PathBuf::from(partial), String::new())
    } else {
        let path = PathBuf::from(partial);
        let parent = path.parent().unwrap_or(Path::new("/")).to_path_buf();
        let prefix = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        (parent, prefix)
    };

    let entries = match std::fs::read_dir(&parent) {
        Ok(entries) => entries,
        Err(_) => return HttpResponse::Ok().json(json!({ "completions": Vec::<String>::new() })),
    };

    let prefix_lower = prefix.to_lowercase();
    let mut completions: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| {
            if prefix.is_empty() {
                return true;
            }
            e.file_name()
                .to_string_lossy()
                .to_lowercase()
                .starts_with(&prefix_lower)
        })
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();

    completions.sort();
    completions.truncate(20);

    HttpResponse::Ok().json(json!({ "completions": completions }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_node::{FoldNode, NodeConfig};
    use crate::server::node_manager::{NodeManager, NodeManagerConfig};
    use actix_web::test;
    use std::sync::Arc;
    use tempfile::tempdir;

    async fn create_test_state(temp_dir: &tempfile::TempDir) -> web::Data<AppState> {
        let keypair = crate::security::Ed25519KeyPair::generate().unwrap();
        let config = NodeConfig::new(temp_dir.path().to_path_buf())
            .with_schema_service_url("test://mock")
            .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());
        let node = FoldNode::new(config.clone()).await.unwrap();

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
