//! Integration test for signature verification.
//!
//! Tests the end-to-end sign-then-verify flow that occurs when a frontend
//! signs a request and the middleware verifies it.

use base64::Engine as _;
use fold_db::constants::SINGLE_PUBLIC_KEY_ID;
use fold_db::security::{Ed25519KeyPair, MessageSigner, MessageVerifier, PublicKeyInfo};
use serde_json::json;

/// Test that MessageSigner and MessageVerifier round-trip correctly
/// (the same flow that runs when a frontend signs and the middleware verifies).
#[tokio::test]
async fn test_sign_and_verify_round_trip() {
    let keypair = Ed25519KeyPair::generate().unwrap();
    let signer = MessageSigner::new(keypair);

    let verifier = MessageVerifier::new(300);
    let key_info = PublicKeyInfo::new(
        SINGLE_PUBLIC_KEY_ID.to_string(),
        signer.keypair_public_key_base64(),
        "system".to_string(),
        vec!["read".to_string(), "write".to_string()],
    );
    verifier
        .register_system_public_key(key_info)
        .await
        .unwrap();

    // Sign a mutation-like payload
    let payload = json!({
        "schema_name": "TestSchema",
        "fields_and_values": {"title": "Hello World"},
        "pub_key": "test_key"
    });

    let signed = signer.sign_message(payload.clone()).unwrap();
    let result = verifier.verify_message(&signed).unwrap();
    assert!(result.is_valid, "Signature should be valid");
    assert!(result.timestamp_valid, "Timestamp should be valid");

    // Verify the payload can be decoded back to the original
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(&signed.payload)
        .unwrap();
    let decoded: serde_json::Value = serde_json::from_slice(&decoded_bytes).unwrap();
    assert_eq!(decoded, payload);
}

/// Test that a tampered payload is rejected
#[tokio::test]
async fn test_tampered_payload_rejected() {
    let keypair = Ed25519KeyPair::generate().unwrap();
    let signer = MessageSigner::new(keypair);

    let verifier = MessageVerifier::new(300);
    let key_info = PublicKeyInfo::new(
        SINGLE_PUBLIC_KEY_ID.to_string(),
        signer.keypair_public_key_base64(),
        "system".to_string(),
        vec!["read".to_string(), "write".to_string()],
    );
    verifier
        .register_system_public_key(key_info)
        .await
        .unwrap();

    let payload = json!({"data": "original"});
    let mut signed = signer.sign_message(payload).unwrap();

    // Tamper with the payload (replace with different data)
    let tampered = json!({"data": "tampered"});
    let tampered_bytes = serde_json::to_vec(&tampered).unwrap();
    signed.payload = base64::engine::general_purpose::STANDARD.encode(&tampered_bytes);

    let result = verifier.verify_message(&signed).unwrap();
    assert!(!result.is_valid, "Tampered payload should fail verification");
}

/// Test that verification fails when no public key is registered
#[tokio::test]
async fn test_no_key_registered_rejected() {
    let keypair = Ed25519KeyPair::generate().unwrap();
    let signer = MessageSigner::new(keypair);

    // Verifier without any registered key
    let verifier = MessageVerifier::new(300);

    let payload = json!({"data": "test"});
    let signed = signer.sign_message(payload).unwrap();

    let result = verifier.verify_message(&signed).unwrap();
    assert!(
        !result.is_valid,
        "Should fail when no public key is registered"
    );
}

/// Test that a signature from a different key is rejected
#[tokio::test]
async fn test_wrong_key_rejected() {
    let keypair1 = Ed25519KeyPair::generate().unwrap();
    let keypair2 = Ed25519KeyPair::generate().unwrap();
    let signer = MessageSigner::new(keypair1);

    let verifier = MessageVerifier::new(300);
    // Register keypair2's public key, but sign with keypair1
    let key_info = PublicKeyInfo::new(
        SINGLE_PUBLIC_KEY_ID.to_string(),
        keypair2.public_key_base64(),
        "system".to_string(),
        vec!["read".to_string(), "write".to_string()],
    );
    verifier
        .register_system_public_key(key_info)
        .await
        .unwrap();

    let payload = json!({"data": "test"});
    let signed = signer.sign_message(payload).unwrap();

    let result = verifier.verify_message(&signed).unwrap();
    assert!(
        !result.is_valid,
        "Should fail when signed with wrong key"
    );
}

/// Test that write endpoints are always protected (no opt-out)
#[test]
fn test_write_endpoints_always_protected() {
    use fold_db::server::middleware::signature::is_protected_write;

    assert!(
        is_protected_write(&actix_web::http::Method::POST, "/api/mutation"),
        "POST /api/mutation must always require signature verification"
    );
}
