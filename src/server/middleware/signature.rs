//! Signature verification middleware for Actix-web.
//!
//! Verifies Ed25519 signatures on write endpoints. POST/PUT/PATCH requests
//! to protected paths must include a `SignedMessage` envelope. The middleware
//! verifies the signature, then replaces the request body with the decoded
//! payload so downstream handlers receive the original JSON.

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::web::BytesMut;
use actix_web::{Error, HttpMessage, HttpResponse};
use futures_util::future::LocalBoxFuture;
use futures_util::StreamExt;
use std::future::{ready, Ready};
use std::rc::Rc;

use crate::security::SignedMessage;
use crate::server::http_server::AppState;

use base64::{engine::general_purpose, Engine as _};

/// Combined prefix rules: `(path_prefix, is_protected)`.
/// Exempt entries appear first so they win over any overlapping protected entry.
/// GET requests are excluded by the method check, so only POST/PUT/PATCH paths
/// that should be exempt need exempt entries here.
const PREFIX_RULES: &[(&str, bool)] = &[
    // Exempt (not protected)
    ("/api/system/auto-identity", false),
    ("/api/query", false),
    ("/api/system/private-key", false),
    ("/api/system/public-key", false),
    ("/api/system/status", false),
    ("/api/system/database-status", false),
    ("/api/system/complete-path", false),
    ("/api/security/", false),
    ("/api/openapi.json", false),
    // Protected (require signature)
    ("/api/mutation", true),
    ("/api/schemas/load", true),
    ("/api/schema/", true), // covers /api/schema/{name}/approve and /api/schema/{name}/block
    ("/api/ingestion/process", true),
    ("/api/ingestion/upload", true),
    ("/api/ingestion/config", true),
    ("/api/ingestion/batch-folder", true),
    ("/api/ingestion/smart-folder/", true),
    ("/api/system/reset-database", true),
    ("/api/system/setup", true),
    ("/api/system/database-config", true),
    ("/api/system/migrate-to-cloud", true),
    ("/api/llm-query/", true),
];

pub fn is_protected_write(method: &actix_web::http::Method, path: &str) -> bool {
    use actix_web::http::Method;

    // Only POST/PUT/PATCH can be write operations
    if method != Method::POST && method != Method::PUT && method != Method::PATCH {
        return false;
    }

    PREFIX_RULES
        .iter()
        .find(|(prefix, _)| path.starts_with(prefix))
        .is_some_and(|(_, protected)| *protected)
}

/// Middleware that verifies Ed25519 signatures on write endpoints.
/// Signatures are always required — there is no opt-out.
#[derive(Clone)]
pub struct SignatureVerificationMiddleware;

impl<S, B> Transform<S, ServiceRequest> for SignatureVerificationMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = SignatureVerificationService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SignatureVerificationService {
            service: Rc::new(service),
        }))
    }
}

pub struct SignatureVerificationService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for SignatureVerificationService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();

        Box::pin(async move {
            // Only verify protected write endpoints
            if !is_protected_write(req.method(), req.path()) {
                return svc
                    .call(req)
                    .await
                    .map(|res| res.map_into_left_body());
            }

            // Skip multipart uploads (file uploads have their own handling)
            if let Some(ct) = req.headers().get("content-type") {
                if let Ok(ct_str) = ct.to_str() {
                    if ct_str.starts_with("multipart/") {
                        return svc
                            .call(req)
                            .await
                            .map(|res| res.map_into_left_body());
                    }
                }
            }

            // Read the request body
            let mut body = BytesMut::new();
            let mut payload = req.take_payload();
            while let Some(chunk) = payload.next().await {
                let chunk = chunk?;
                body.extend_from_slice(&chunk);
            }

            // Try to parse as SignedMessage
            let signed_message: SignedMessage = match serde_json::from_slice(&body) {
                Ok(msg) => msg,
                Err(_) => {
                    // Not a SignedMessage envelope — reject when signatures are required
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({
                            "error": "Request must be signed. Expected a SignedMessage envelope."
                        }));
                    return Ok(req.into_response(response).map_into_right_body());
                }
            };

            // Get the node manager from app state to access the verifier
            let app_state = match req.app_data::<actix_web::web::Data<AppState>>() {
                Some(state) => state.clone(),
                None => {
                    let response = HttpResponse::InternalServerError()
                        .json(serde_json::json!({
                            "error": "Server configuration error: missing app state"
                        }));
                    return Ok(req.into_response(response).map_into_right_body());
                }
            };

            // We need a node to get the verifier. Use "system" as the user for verification.
            // In local mode this returns the shared node; in cloud mode we need any node.
            // We'll try to get a node from the user context header first.
            let user_id = req
                .headers()
                .get("x-user-hash")
                .or_else(|| req.headers().get("x-user-id"))
                .and_then(|v| v.to_str().ok())
                .unwrap_or("system");

            let node_arc = match app_state.node_manager.get_node(user_id).await {
                Ok(n) => n,
                Err(e) => {
                    let response = HttpResponse::InternalServerError()
                        .json(serde_json::json!({
                            "error": format!("Failed to get node for verification: {}", e)
                        }));
                    return Ok(req.into_response(response).map_into_right_body());
                }
            };

            let node = node_arc.read().await;
            let verifier = &node.get_security_manager().verifier;

            // Verify the signature
            match verifier.verify_message(&signed_message) {
                Ok(result) if result.is_valid => {
                    // Signature valid — decode the payload and replace the body
                    let payload_bytes = match general_purpose::STANDARD
                        .decode(&signed_message.payload)
                    {
                        Ok(bytes) => bytes,
                        Err(_) => {
                            let response = HttpResponse::BadRequest()
                                .json(serde_json::json!({
                                    "error": "Invalid base64 payload in signed message"
                                }));
                            return Ok(req.into_response(response).map_into_right_body());
                        }
                    };

                    // Drop the node lock before calling the downstream service
                    drop(node);

                    // Replace the request payload with the decoded original JSON bytes
                    req.set_payload(actix_http::Payload::from(
                        actix_web::web::Bytes::from(payload_bytes),
                    ));

                    svc.call(req).await.map(|res| res.map_into_left_body())
                }
                Ok(result) => {
                    let error_msg = result
                        .error
                        .unwrap_or_else(|| "Signature verification failed".to_string());
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({ "error": error_msg }));
                    Ok(req.into_response(response).map_into_right_body())
                }
                Err(e) => {
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({
                            "error": format!("Signature verification error: {}", e)
                        }));
                    Ok(req.into_response(response).map_into_right_body())
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_protected_write_post_mutation() {
        assert!(is_protected_write(
            &actix_web::http::Method::POST,
            "/api/mutation"
        ));
    }

    #[test]
    fn test_is_protected_write_get_not_protected() {
        assert!(!is_protected_write(
            &actix_web::http::Method::GET,
            "/api/mutation"
        ));
    }

    #[test]
    fn test_is_protected_write_query_exempt() {
        assert!(!is_protected_write(
            &actix_web::http::Method::POST,
            "/api/query"
        ));
    }

    #[test]
    fn test_is_protected_write_schema_approve() {
        assert!(is_protected_write(
            &actix_web::http::Method::POST,
            "/api/schema/my_schema/approve"
        ));
    }

    #[test]
    fn test_is_protected_write_llm_query() {
        assert!(is_protected_write(
            &actix_web::http::Method::POST,
            "/api/llm-query/chat"
        ));
    }

    #[test]
    fn test_is_protected_write_auto_identity_exempt() {
        assert!(!is_protected_write(
            &actix_web::http::Method::POST,
            "/api/system/auto-identity"
        ));
    }

    #[test]
    fn test_is_protected_write_schemas_load() {
        assert!(is_protected_write(
            &actix_web::http::Method::POST,
            "/api/schemas/load"
        ));
    }

    #[test]
    fn test_is_protected_write_ingestion_process() {
        assert!(is_protected_write(
            &actix_web::http::Method::POST,
            "/api/ingestion/process"
        ));
    }

    #[test]
    fn test_is_protected_write_system_setup() {
        assert!(is_protected_write(
            &actix_web::http::Method::POST,
            "/api/system/setup"
        ));
    }

    #[test]
    fn test_is_protected_write_get_schemas_not_protected() {
        assert!(!is_protected_write(
            &actix_web::http::Method::GET,
            "/api/schemas"
        ));
    }

    #[test]
    fn test_is_protected_write_ingestion_status_exempt() {
        assert!(!is_protected_write(
            &actix_web::http::Method::POST,
            "/api/ingestion/status"
        ));
    }
}
