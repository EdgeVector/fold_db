use crate::handlers::system::NodeKeyResponse;
use crate::handlers::{ApiResponse, HandlerError};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use crate::server::routes::{handler_error_to_response, require_node_read};
use actix_web::{web, HttpResponse, Responder};
use serde_json::json;

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
}
