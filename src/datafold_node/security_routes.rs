//! Security-related HTTP routes for key management and authentication

use super::http_server::AppState;
use crate::security::{
    SecurityManager, KeyRegistrationRequest,
    SignedMessage, SecurityMiddleware, SecurityError,
    ClientSecurity,
};
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde_json::json;
use std::sync::Arc;
use crate::log_feature;
use crate::logging::features::LogFeature;

/// Get the security manager from the node
async fn get_security_manager(data: &web::Data<AppState>) -> Arc<SecurityManager> {
    let node_guard = data.node.lock().await;
    node_guard.get_security_manager().clone()
}

/// Register the system-wide public key
pub async fn register_system_public_key(
    request: web::Json<KeyRegistrationRequest>,
    data: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let security_manager = get_security_manager(&data).await;
    
    match security_manager.register_system_public_key(request.into_inner()) {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => {
            log_feature!(LogFeature::HttpServer, error, "Failed to register public key: {}", e);
            // Let the framework handle the error response
            Err(actix_web::error::ErrorBadRequest(e.to_string()))
        }
    }
}

/// Remove the system public key
pub async fn remove_system_public_key(data: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let security_manager = get_security_manager(&data).await;
    
    match security_manager.remove_system_public_key() {
        Ok(_) => Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "message": "System key removed successfully"
        }))),
        Err(e) => Ok(HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": e.to_string()
        }))),
    }
}

/// Get the system public key information
pub async fn get_system_public_key(data: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let security_manager = get_security_manager(&data).await;
    
    match security_manager.get_system_public_key() {
        Ok(Some(key_info)) => Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "key": key_info
        }))),
        Ok(None) => Ok(HttpResponse::NotFound().json(json!({
            "success": false,
            "error": "System key not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(json!({
            "success": false,
            "error": e.to_string()
        }))),
    }
}

/// Verify a signed message (for testing purposes)
pub async fn verify_message(
    message: web::Json<SignedMessage>,
    data: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let security_manager = get_security_manager(&data).await;
    
    match security_manager.verify_message(&message.into_inner()) {
        Ok(result) => Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "verification_result": {
                "is_valid": result.is_valid,
                "timestamp_valid": result.timestamp_valid,
                "owner_id": result.public_key_info.as_ref().map(|info| &info.owner_id),
                "permissions": result.public_key_info.as_ref().map(|info| &info.permissions),
                "error": result.error
            }
        }))),
        Err(e) => Ok(HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": e.to_string()
        }))),
    }
}

/// Generate a demo key pair (for development/testing purposes only)
pub async fn generate_demo_keypair(_data: web::Data<AppState>) -> ActixResult<HttpResponse> {
    match ClientSecurity::generate_client_keypair() {
        Ok(keypair) => {
            let public_key_base64 = keypair.public_key_base64();
            let secret_key_base64 = keypair.secret_key_base64();
            
            Ok(HttpResponse::Ok().json(json!({
                "success": true,
                "warning": "This is for development/testing only. Never expose secret keys in production!",
                "keypair": {
                    "public_key": public_key_base64,
                    "secret_key": secret_key_base64
                }
            })))
        },
        Err(e) => Ok(HttpResponse::InternalServerError().json(json!({
            "success": false,
            "error": e.to_string()
        }))),
    }
}

/// Get security configuration status
pub async fn get_security_status(data: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let security_manager = get_security_manager(&data).await;
    let key_present = security_manager.get_system_public_key().unwrap_or(None).is_some();
    
    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "status": {
            "signatures_required": security_manager.config.require_signatures,
            "tls_required": security_manager.config.require_tls,
            "encryption_enabled": security_manager.is_encryption_enabled(),
            "system_key_present": key_present,
        }
    })))
}

/// Get client integration examples
pub async fn get_client_examples(_data: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let examples = json!({
        "rust_example": ClientSecurity::generate_client_example(),
        "javascript_example": r#"
// JavaScript client example
class DataFoldSecurityClient {
    constructor(secretKey, publicKeyId) {
        this.secretKey = secretKey;
        this.publicKeyId = publicKeyId;
    }
    
    async signMessage(payload) {
        // This would use a JavaScript Ed25519 library like tweetnacl
        const timestamp = Math.floor(Date.now() / 1000);
        const messageToSign = JSON.stringify(payload) + timestamp + this.publicKeyId;
        
        // Sign with Ed25519 (implementation depends on crypto library)
        const signature = await ed25519.sign(messageToSign, this.secretKey);
        
        return {
            payload: payload,
            signature: base64Encode(signature),
            public_key_id: this.publicKeyId,
            timestamp: timestamp
        };
    }
    
    async sendSignedRequest(endpoint, payload) {
        const signedMessage = await this.signMessage(payload);
        
        const response = await fetch(endpoint, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(signedMessage)
        });
        
        return response.json();
    }
}
"#,
        "python_example": r#"
# Python client example
import ed25519
import base64
import json
import time
import requests

class DataFoldSecurityClient:
    def __init__(self, secret_key_base64, public_key_id):
        self.secret_key = ed25519.SigningKey(base64.b64decode(secret_key_base64))
        self.public_key_id = public_key_id
    
    def sign_message(self, payload):
        timestamp = int(time.time())
        message_to_sign = json.dumps(payload, sort_keys=True) + str(timestamp) + self.public_key_id
        
        signature = self.secret_key.sign(message_to_sign.encode())
        
        return {
            "payload": payload,
            "signature": base64.b64encode(signature).decode(),
            "public_key_id": self.public_key_id,
            "timestamp": timestamp
        }
    
    def send_signed_request(self, endpoint, payload):
        signed_message = self.sign_message(payload)
        
        response = requests.post(endpoint, json=signed_message)
        return response.json()
"#
    });
    
    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "examples": examples
    })))
}

/// Middleware to verify signed messages on protected endpoints
pub async fn verify_signed_request(
    message: web::Json<SignedMessage>,
    data: web::Data<AppState>,
    required_permissions: Option<Vec<String>>,
) -> Result<String, SecurityError> {
    let security_manager = get_security_manager(&data).await;
    let middleware = SecurityMiddleware::new(security_manager);
    
    let permissions: Option<&[String]> = required_permissions.as_deref();
    middleware.validate_request(&message.into_inner(), permissions)
}

/// Example of a protected endpoint that requires signature verification
pub async fn protected_endpoint(
    message: web::Json<SignedMessage>,
    data: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    // Verify the message signature and require 'read' permission
    match verify_signed_request(message, data, Some(vec!["read".to_string()])).await {
        Ok(owner_id) => {
            // Message is valid, process the request
            Ok(HttpResponse::Ok().json(json!({
                "success": true,
                "message": "Access granted",
                "authenticated_user": owner_id,
                "data": "This is protected data"
            })))
        },
        Err(e) => {
            Ok(HttpResponse::Unauthorized().json(json!({
                "success": false,
                "error": e.to_string()
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{ClientSecurity, Ed25519PublicKey};
    
    #[test]
    fn test_security_integration() {
        // Generate a test keypair
        let keypair = ClientSecurity::generate_client_keypair().unwrap();
        let public_key = Ed25519PublicKey::from_bytes(&keypair.public_key_bytes()).unwrap();
        
        // Create registration request
        let registration_request = KeyRegistrationRequest {
            public_key: public_key.to_base64(),
            owner_id: "test_user".to_string(),
            permissions: vec!["read".to_string(), "write".to_string()],
            metadata: std::collections::HashMap::new(),
            expires_at: None,
        };
        
        // Just test that the request structure is valid
        assert_eq!(registration_request.owner_id, "test_user");
        assert_eq!(registration_request.permissions.len(), 2);
        assert!(!registration_request.public_key.is_empty());
    }
}