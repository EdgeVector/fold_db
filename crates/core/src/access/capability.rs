use serde::{Deserialize, Serialize};

use super::types::{AccessContext, AccessDecision, AccessDenialReason};

/// The kind of access a capability token grants.
#[cfg_attr(feature = "ts-bindings", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum CapabilityKind {
    /// RX_k(pk): grants read access; counter decrements with each read
    Read,
    /// WX_k(pk): grants write access; counter decrements with each write
    Write,
}

/// A cryptographic capability constraint on a field.
/// Binds a public key to a quota-limited access grant.
#[cfg_attr(feature = "ts-bindings", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CapabilityConstraint {
    /// Base64-encoded public key of the capability holder
    pub public_key: String,
    /// Remaining uses before the capability is exhausted (0 = exhausted)
    pub remaining_quota: u64,
    /// What kind of access this capability grants
    pub kind: CapabilityKind,
}

impl CapabilityConstraint {
    pub fn new(public_key: impl Into<String>, kind: CapabilityKind, quota: u64) -> Self {
        Self {
            public_key: public_key.into(),
            remaining_quota: quota,
            kind,
        }
    }

    /// Decrement the quota by 1. Returns false if already exhausted.
    pub fn decrement(&mut self) -> bool {
        if self.remaining_quota > 0 {
            self.remaining_quota -= 1;
            true
        } else {
            false
        }
    }

    pub fn is_exhausted(&self) -> bool {
        self.remaining_quota == 0
    }
}

/// Check capability constraints for a field access.
///
/// If no capabilities of the required kind exist on the field, access is granted
/// (no capability requirement). Otherwise the caller must hold at least one matching
/// capability with remaining quota > 0.
pub fn check_capabilities(
    capabilities: &[CapabilityConstraint],
    context: &AccessContext,
    is_write: bool,
) -> AccessDecision {
    let required_kind = if is_write {
        CapabilityKind::Write
    } else {
        CapabilityKind::Read
    };

    let relevant: Vec<_> = capabilities
        .iter()
        .filter(|c| c.kind == required_kind)
        .collect();

    // No capability constraints of this kind = pass
    if relevant.is_empty() {
        return AccessDecision::Granted;
    }

    // Caller must hold at least one matching capability with quota > 0
    for cap in &relevant {
        if context.public_keys.iter().any(|pk| pk == &cap.public_key) {
            if cap.remaining_quota > 0 {
                return AccessDecision::Granted;
            } else {
                return AccessDecision::Denied(AccessDenialReason::CapabilityExhausted {
                    kind: required_kind,
                });
            }
        }
    }

    AccessDecision::Denied(AccessDenialReason::CapabilityMissing {
        kind: required_kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_with_key(key: &str) -> AccessContext {
        AccessContext {
            user_id: "test".into(),
            is_owner: false,
            tiers: Default::default(),
            public_keys: vec![key.to_string()],
            paid_schemas: Default::default(),
        }
    }

    #[test]
    fn test_no_capabilities_grants_access() {
        let caps: Vec<CapabilityConstraint> = vec![];
        let ctx = ctx_with_key("pk1");
        assert!(check_capabilities(&caps, &ctx, false).is_granted());
        assert!(check_capabilities(&caps, &ctx, true).is_granted());
    }

    #[test]
    fn test_matching_capability_grants() {
        let caps = vec![CapabilityConstraint::new("pk1", CapabilityKind::Read, 10)];
        let ctx = ctx_with_key("pk1");
        assert!(check_capabilities(&caps, &ctx, false).is_granted());
    }

    #[test]
    fn test_wrong_key_denied() {
        let caps = vec![CapabilityConstraint::new("pk1", CapabilityKind::Read, 10)];
        let ctx = ctx_with_key("pk2");
        let result = check_capabilities(&caps, &ctx, false);
        assert!(result.is_denied());
    }

    #[test]
    fn test_exhausted_denied() {
        let caps = vec![CapabilityConstraint::new("pk1", CapabilityKind::Read, 0)];
        let ctx = ctx_with_key("pk1");
        let result = check_capabilities(&caps, &ctx, false);
        assert!(result.is_denied());
        if let AccessDecision::Denied(AccessDenialReason::CapabilityExhausted { kind }) = result {
            assert_eq!(kind, CapabilityKind::Read);
        } else {
            panic!("expected CapabilityExhausted");
        }
    }

    #[test]
    fn test_write_kind_mismatch() {
        // Only read capabilities — write check should pass (no write caps required)
        let caps = vec![CapabilityConstraint::new("pk1", CapabilityKind::Read, 10)];
        let ctx = ctx_with_key("pk1");
        assert!(check_capabilities(&caps, &ctx, true).is_granted());
    }

    #[test]
    fn test_decrement() {
        let mut cap = CapabilityConstraint::new("pk1", CapabilityKind::Read, 2);
        assert!(!cap.is_exhausted());
        assert!(cap.decrement());
        assert_eq!(cap.remaining_quota, 1);
        assert!(cap.decrement());
        assert_eq!(cap.remaining_quota, 0);
        assert!(cap.is_exhausted());
        assert!(!cap.decrement());
    }

    #[test]
    fn test_serialization() {
        let cap = CapabilityConstraint::new("pk1", CapabilityKind::Write, 100);
        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: CapabilityConstraint = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.public_key, "pk1");
        assert_eq!(deserialized.remaining_quota, 100);
        assert_eq!(deserialized.kind, CapabilityKind::Write);
    }
}
