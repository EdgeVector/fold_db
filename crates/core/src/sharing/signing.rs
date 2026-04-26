//! Ed25519 signing helpers for [`ShareRule`].
//!
//! A share rule authorizes the writer to publish encrypted writes under
//! `share_prefix` that any holder of `share_e2e_secret` can decrypt. The
//! signature binds `rule_id`, `recipient_pubkey`, `share_prefix`,
//! `share_e2e_secret`, and `created_at` to `writer_pubkey` so an observer
//! (the recipient, or anyone verifying the rule later) can confirm the rule
//! was issued by the claimed writer.
//!
//! Verification is currently non-enforcing: the receiver-side of cross-user
//! sharing does NOT reject unsigned or invalid rules yet. Signing is added
//! first so the wire format is future-proof; enforcement lands in a follow-up.

use super::types::ShareRule;
use crate::security::{Ed25519KeyPair, Ed25519PublicKey, KeyUtils, SecurityError, SecurityResult};

/// Canonical byte serialization used for both signing and verification.
///
/// Format: fields joined by a single `0x00` separator in a fixed order:
/// `rule_id || 0x00 || recipient_pubkey || 0x00 || share_prefix || 0x00 ||
///  share_e2e_secret || 0x00 || created_at.to_be_bytes()`
///
/// The `signature`, `writer_pubkey`, `recipient_display_name`, `active`, and
/// `scope` fields are NOT included — display name / scope are mutable policy,
/// `active` toggles on deactivate, and signature/writer_pubkey are the
/// signature bindings themselves.
pub fn canonical_bytes(rule: &ShareRule) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        rule.rule_id.len()
            + rule.recipient_pubkey.len()
            + rule.share_prefix.len()
            + rule.share_e2e_secret.len()
            + 8
            + 4,
    );
    bytes.extend_from_slice(rule.rule_id.as_bytes());
    bytes.push(0x00);
    bytes.extend_from_slice(rule.recipient_pubkey.as_bytes());
    bytes.push(0x00);
    bytes.extend_from_slice(rule.share_prefix.as_bytes());
    bytes.push(0x00);
    bytes.extend_from_slice(&rule.share_e2e_secret);
    bytes.push(0x00);
    bytes.extend_from_slice(&rule.created_at.to_be_bytes());
    bytes
}

/// Sign `rule` with `keypair`. Returns the base64-encoded signature.
/// The caller is responsible for placing the signature on `rule.signature` and
/// ensuring `rule.writer_pubkey` matches `keypair`'s public key.
pub fn sign_share_rule(rule: &ShareRule, keypair: &Ed25519KeyPair) -> String {
    let bytes = canonical_bytes(rule);
    let sig = keypair.sign(&bytes);
    KeyUtils::signature_to_base64(&sig)
}

/// Verify the signature on `rule` against `rule.writer_pubkey`.
/// Returns `Ok(true)` on valid signature, `Ok(false)` on mismatch,
/// `Err` only when the public-key or signature encoding itself is malformed.
pub fn verify_share_rule(rule: &ShareRule) -> SecurityResult<bool> {
    if rule.signature.is_empty() {
        return Ok(false);
    }
    let pubkey = Ed25519PublicKey::from_base64(&rule.writer_pubkey)
        .map_err(|e| SecurityError::InvalidPublicKey(e.to_string()))?;
    let signature = KeyUtils::signature_from_base64(&rule.signature)
        .map_err(|e| SecurityError::InvalidSignature(e.to_string()))?;
    let bytes = canonical_bytes(rule);
    Ok(pubkey.verify(&bytes, &signature))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharing::types::ShareScope;

    fn make_rule(writer_pubkey: String) -> ShareRule {
        ShareRule {
            rule_id: "rule-1".to_string(),
            recipient_pubkey: "recipient-pk".to_string(),
            recipient_display_name: "Bob".to_string(),
            scope: ShareScope::AllSchemas,
            share_prefix: "share:alice:bob".to_string(),
            share_e2e_secret: vec![0x42u8; 32],
            active: true,
            created_at: 1_700_000_000,
            writer_pubkey,
            signature: String::new(),
        }
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let kp = Ed25519KeyPair::generate().unwrap();
        let mut rule = make_rule(kp.public_key_base64());
        rule.signature = sign_share_rule(&rule, &kp);
        assert!(verify_share_rule(&rule).unwrap());
    }

    #[test]
    fn tampered_prefix_fails_verify() {
        let kp = Ed25519KeyPair::generate().unwrap();
        let mut rule = make_rule(kp.public_key_base64());
        rule.signature = sign_share_rule(&rule, &kp);
        rule.share_prefix = "share:eve:bob".to_string();
        assert!(!verify_share_rule(&rule).unwrap());
    }

    #[test]
    fn tampered_secret_fails_verify() {
        let kp = Ed25519KeyPair::generate().unwrap();
        let mut rule = make_rule(kp.public_key_base64());
        rule.signature = sign_share_rule(&rule, &kp);
        rule.share_e2e_secret = vec![0x00u8; 32];
        assert!(!verify_share_rule(&rule).unwrap());
    }

    #[test]
    fn empty_signature_is_invalid() {
        let kp = Ed25519KeyPair::generate().unwrap();
        let rule = make_rule(kp.public_key_base64());
        assert!(!verify_share_rule(&rule).unwrap());
    }

    #[test]
    fn display_name_is_mutable_after_signing() {
        // scope + display name are not bound — so changing them does NOT
        // invalidate the signature (they're mutable local policy).
        let kp = Ed25519KeyPair::generate().unwrap();
        let mut rule = make_rule(kp.public_key_base64());
        rule.signature = sign_share_rule(&rule, &kp);
        rule.recipient_display_name = "Bob (renamed)".to_string();
        assert!(verify_share_rule(&rule).unwrap());
    }
}
