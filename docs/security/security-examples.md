# Security Module Usage Examples

This document demonstrates how to use the DataFold security module for client key management, message signing, and encryption.

## Overview

The security module provides:
- Ed25519 key pair generation and management
- Message signing and verification
- AES-256-GCM encryption for data at rest
- Integration with HTTP endpoints

## Client-Side Usage

### 1. Generate Key Pair

```rust
use datafold::security::{ClientSecurity, Ed25519KeyPair, KeyRegistrationRequest};

// Generate a new key pair
let keypair = ClientSecurity::generate_client_keypair()?;

// Get the keys in different formats
let public_key_base64 = keypair.public_key_base64();
let secret_key_base64 = keypair.secret_key_base64();
let public_key_bytes = keypair.public_key_bytes();
let secret_key_bytes = keypair.secret_key_bytes();

println!("Public key: {}", public_key_base64);
// Store secret_key_base64 securely - never expose it!
```

### 2. Register Public Key with Server

```rust
use std::collections::HashMap;

// Create registration request
let registration_request = KeyRegistrationRequest {
    public_key: public_key_base64,
    owner_id: "alice@example.com".to_string(),
    permissions: vec!["read".to_string(), "write".to_string()],
    metadata: HashMap::new(),
    expires_at: None, // No expiration
};

// Send to server (pseudo-code)
// let response = http_client.post("/api/security/keys/register")
//     .json(&registration_request)
//     .send().await?;
// let key_response: KeyRegistrationResponse = response.json().await?;
// let public_key_id = key_response.public_key_id.unwrap();
```

### 3. Sign Messages

```rust
use datafold::security::MessageSigner;
use serde_json::json;

// Create a message signer (you got public_key_id from registration response)
let public_key_id = "ABC123...".to_string(); // From server registration response
let signer = ClientSecurity::create_signer(keypair, public_key_id);

// Create a message payload
let payload = json!({
    "action": "create_user",
    "data": {
        "username": "alice",
        "email": "alice@example.com"
    }
});

// Sign the message
let signed_message = ClientSecurity::sign_message(&signer, payload)?;

// Send signed message to server
// let response = http_client.post("/api/some-endpoint")
//     .json(&signed_message)
//     .send().await?;
```

## Server-Side Usage

### 1. Initialize Security Manager

```rust
use datafold::security::{SecurityManager, SecurityConfigBuilder};

// Create security configuration
let config = SecurityConfigBuilder::new()
    .require_signatures(true)        // All messages must be signed
    .require_tls(true)              // All connections must use TLS
    .enable_encryption()            // Enable at-rest encryption with auto-generated key
    .build();

// Create security manager
let security_manager = SecurityManager::new(config)?;
```

### 2. Handle Key Registration

```rust
use datafold::security::KeyRegistrationRequest;

async fn register_key(
    request: KeyRegistrationRequest,
    security_manager: &SecurityManager
) -> Result<String, SecurityError> {
    let response = security_manager.register_public_key(request)?;
    
    if response.success {
        Ok(response.public_key_id.unwrap())
    } else {
        Err(SecurityError::KeyRegistrationFailed(response.error.unwrap()))
    }
}
```

### 3. Verify Signed Messages

```rust
use datafold::security::{SignedMessage, SecurityMiddleware};

async fn handle_signed_request(
    signed_message: SignedMessage,
    security_manager: &SecurityManager
) -> Result<String, SecurityError> {
    // Verify the message signature
    let result = security_manager.verify_message(&signed_message)?;
    
    if !result.is_valid {
        return Err(SecurityError::SignatureVerificationFailed(
            result.error.unwrap_or("Invalid signature".to_string())
        ));
    }
    
    if !result.timestamp_valid {
        return Err(SecurityError::SignatureVerificationFailed(
            "Message timestamp is too old or in the future".to_string()
        ));
    }
    
    // Get the authenticated user
    let owner_id = result.public_key_info
        .map(|info| info.owner_id)
        .unwrap_or_else(|| "anonymous".to_string());
    
    // Process the verified request
    println!("Processing request from: {}", owner_id);
    println!("Request payload: {}", signed_message.payload);
    
    Ok(owner_id)
}
```

### 4. Middleware Integration

```rust
use datafold::security::SecurityMiddleware;
use std::sync::Arc;

// Create middleware
let middleware = SecurityMiddleware::new(Arc::new(security_manager));

// Validate request with required permissions
async fn protected_endpoint(
    signed_message: SignedMessage,
    middleware: &SecurityMiddleware
) -> Result<String, SecurityError> {
    let owner_id = middleware.validate_request(
        &signed_message,
        Some(&["write".to_string()]) // Require 'write' permission
    )?;
    
    println!("Authenticated user: {}", owner_id);
    Ok("Success".to_string())
}
```

## Encryption Examples

### 1. Encrypt Sensitive Data

```rust
use serde_json::json;

// Encrypt JSON data before storing
let sensitive_data = json!({
    "ssn": "123-45-6789",
    "credit_card": "1234-5678-9012-3456"
});

if let Some(encrypted) = security_manager.encrypt_json(&sensitive_data)? {
    println!("Data encrypted: {}", encrypted.algorithm);
    // Store encrypted data in database
} else {
    println!("Encryption disabled, storing plaintext");
    // Store plaintext data
}
```

### 2. Decrypt Data

```rust
// Retrieve encrypted data from database and decrypt
if let Some(encrypted_data) = get_encrypted_data_from_db() {
    let decrypted = security_manager.decrypt_json(&encrypted_data)?;
    println!("Decrypted data: {}", decrypted);
}
```

## HTTP API Examples

### 1. Register a Public Key

```bash
curl -X POST http://localhost:9001/api/security/keys/register \
  -H "Content-Type: application/json" \
  -d '{
    "public_key": "MCowBQYDK2VwAyEA...",
    "owner_id": "alice@example.com",
    "permissions": ["read", "write"],
    "metadata": {},
    "expires_at": null
  }'
```

### 2. Send a Signed Message

```bash
curl -X POST http://localhost:9001/api/security/protected \
  -H "Content-Type: application/json" \
  -d '{
    "payload": {
      "action": "get_data",
      "resource": "user_profile"
    },
    "signature": "base64_encoded_signature...",
    "public_key_id": "ABC123...",
    "timestamp": 1640995200,
    "nonce": null
  }'
```

### 3. Generate Demo Key Pair (Development Only)

```bash
curl -X POST http://localhost:9001/api/security/demo/keypair
```

**⚠️ Warning**: Never use the demo keypair endpoint in production!

## Best Practices

### Security
1. **Never expose secret keys** - Store them securely on the client side
2. **Use HTTPS/TLS** in production for all API communication
3. **Rotate keys regularly** - Implement key expiration and renewal
4. **Validate timestamps** - Reject messages that are too old
5. **Use strong passwords** for key derivation if storing encrypted keys

### Performance
1. **Cache public keys** on the server to avoid database lookups
2. **Batch verify** multiple signatures when possible
3. **Use connection pooling** for database operations
4. **Implement rate limiting** for key registration endpoints

### Monitoring
1. **Log all security events** - Key registrations, failed verifications, etc.
2. **Monitor for unusual patterns** - Multiple failed signatures from same key
3. **Set up alerts** for security violations
4. **Regular security audits** of registered keys and permissions

## Error Handling

```rust
use datafold::security::SecurityError;

match security_manager.verify_message(&signed_message) {
    Ok(result) if result.is_valid => {
        println!("Message verified successfully");
    },
    Ok(result) => {
        eprintln!("Verification failed: {:?}", result.error);
    },
    Err(SecurityError::KeyNotFound(key_id)) => {
        eprintln!("Public key not found: {}", key_id);
    },
    Err(SecurityError::SignatureVerificationFailed(msg)) => {
        eprintln!("Signature verification failed: {}", msg);
    },
    Err(e) => {
        eprintln!("Security error: {}", e);
    }
}
```

This comprehensive security module provides enterprise-grade cryptographic security for your DataFold deployment while maintaining ease of use and performance.