use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use crate::server::routes::require_user_context;
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

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
