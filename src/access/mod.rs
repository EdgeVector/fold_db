//! Access control system implementing the four-layer model from the FoldDB whitepaper:
//!
//! 1. **Trust distance** — graph-based proximity check (per-field read_max / write_max)
//! 2. **Capability tokens** — cryptographic RX/WX tokens with bounded quotas
//! 3. **Security labels** — information-flow lattice preventing downclassification
//! 4. **Payment gates** — distance-based pricing formulas
//!
//! All four layers are conjunctive: every applicable check must pass.

pub mod audit;
pub mod capability;
pub mod payment;
pub mod security_label;
pub mod trust;
pub mod types;

pub use audit::{AuditAction, AuditEvent, AuditLog};
pub use capability::{CapabilityConstraint, CapabilityKind};
pub use payment::PaymentGate;
pub use security_label::SecurityLabel;
pub use trust::TrustGraph;
pub use types::{
    org_domain, AccessContext, AccessDecision, AccessDenialReason, FieldAccessPolicy,
    TrustDistancePolicy, DOMAIN_FAMILY, DOMAIN_FINANCIAL, DOMAIN_HEALTH, DOMAIN_MEDICAL,
    DOMAIN_PERSONAL,
};

/// Check all four access control layers for a **read** operation on a single field.
///
/// If `policy` is `None`, the field has no access control (legacy behavior) and access is granted.
/// Payment gates are checked at the schema level (passed separately).
pub fn check_read_access(
    policy: Option<&FieldAccessPolicy>,
    context: &AccessContext,
    schema_name: &str,
    payment_gate: Option<&PaymentGate>,
) -> AccessDecision {
    // No policy = legacy behavior, grant access
    let policy = match policy {
        Some(p) => p,
        None => return AccessDecision::Granted,
    };

    let trust_distance = match context.trust_distance {
        Some(d) => d,
        None => return AccessDecision::Denied(AccessDenialReason::TrustDistanceUnresolvable),
    };

    // Layer 1: Trust distance
    if !policy.trust_distance.can_read(trust_distance) {
        return AccessDecision::Denied(AccessDenialReason::TrustDistance {
            required: policy.trust_distance.read_max,
            actual: trust_distance,
        });
    }

    // Layer 2: Capability tokens
    match capability::check_capabilities(&policy.capabilities, context, false) {
        AccessDecision::Granted => {}
        denied => return denied,
    }

    // Layer 3: Security labels
    if let Some(label) = &policy.security_label {
        if !label.allows_read(context.clearance_level) {
            return AccessDecision::Denied(AccessDenialReason::SecurityLabel {
                source_level: label.level,
                caller_level: context.clearance_level,
            });
        }
    }

    // Layer 4: Payment gate (schema-level)
    if let Some(gate) = payment_gate {
        match payment::check_payment(gate, context, schema_name) {
            AccessDecision::Granted => {}
            denied => return denied,
        }
    }

    AccessDecision::Granted
}

/// Check all four access control layers for a **write** operation on a single field.
///
/// Same as `check_read_access` but uses `write_max` and `CapabilityKind::Write`.
pub fn check_write_access(
    policy: Option<&FieldAccessPolicy>,
    context: &AccessContext,
    schema_name: &str,
    payment_gate: Option<&PaymentGate>,
) -> AccessDecision {
    let policy = match policy {
        Some(p) => p,
        None => return AccessDecision::Granted,
    };

    let trust_distance = match context.trust_distance {
        Some(d) => d,
        None => return AccessDecision::Denied(AccessDenialReason::TrustDistanceUnresolvable),
    };

    // Layer 1: Trust distance
    if !policy.trust_distance.can_write(trust_distance) {
        return AccessDecision::Denied(AccessDenialReason::TrustDistance {
            required: policy.trust_distance.write_max,
            actual: trust_distance,
        });
    }

    // Layer 2: Capability tokens
    match capability::check_capabilities(&policy.capabilities, context, true) {
        AccessDecision::Granted => {}
        denied => return denied,
    }

    // Layer 3: Security labels
    if let Some(label) = &policy.security_label {
        if !label.allows_read(context.clearance_level) {
            return AccessDecision::Denied(AccessDenialReason::SecurityLabel {
                source_level: label.level,
                caller_level: context.clearance_level,
            });
        }
    }

    // Layer 4: Payment gate (schema-level)
    if let Some(gate) = payment_gate {
        match payment::check_payment(gate, context, schema_name) {
            AccessDecision::Granted => {}
            denied => return denied,
        }
    }

    AccessDecision::Granted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy_public_read() -> FieldAccessPolicy {
        FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::new(u64::MAX, 0),
            ..Default::default()
        }
    }

    fn policy_owner_only() -> FieldAccessPolicy {
        FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::owner_only(),
            ..Default::default()
        }
    }

    #[test]
    fn test_no_policy_grants_access() {
        let ctx = AccessContext::remote("bob", 100);
        assert!(check_read_access(None, &ctx, "schema", None).is_granted());
        assert!(check_write_access(None, &ctx, "schema", None).is_granted());
    }

    #[test]
    fn test_owner_always_has_access() {
        let ctx = AccessContext::owner("alice");
        let policy = policy_owner_only();
        assert!(check_read_access(Some(&policy), &ctx, "schema", None).is_granted());
        assert!(check_write_access(Some(&policy), &ctx, "schema", None).is_granted());
    }

    #[test]
    fn test_public_read_allows_remote() {
        let ctx = AccessContext::remote("bob", 100);
        let policy = policy_public_read();
        assert!(check_read_access(Some(&policy), &ctx, "schema", None).is_granted());
    }

    #[test]
    fn test_owner_only_blocks_remote_read() {
        let ctx = AccessContext::remote("bob", 1);
        let policy = policy_owner_only();
        let result = check_read_access(Some(&policy), &ctx, "schema", None);
        assert!(result.is_denied());
    }

    #[test]
    fn test_write_blocked_by_trust_distance() {
        let ctx = AccessContext::remote("bob", 3);
        let policy = FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::new(10, 2),
            ..Default::default()
        };
        // Read should pass (distance 3 <= read_max 10)
        assert!(check_read_access(Some(&policy), &ctx, "schema", None).is_granted());
        // Write should fail (distance 3 > write_max 2)
        assert!(check_write_access(Some(&policy), &ctx, "schema", None).is_denied());
    }

    #[test]
    fn test_security_label_blocks_read() {
        let mut ctx = AccessContext::remote("bob", 0);
        ctx.clearance_level = 1;
        let policy = FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::public_read(),
            security_label: Some(SecurityLabel::new(3, "secret")),
            ..Default::default()
        };
        let result = check_read_access(Some(&policy), &ctx, "schema", None);
        assert!(result.is_denied());
        if let AccessDecision::Denied(AccessDenialReason::SecurityLabel {
            source_level,
            caller_level,
        }) = result
        {
            assert_eq!(source_level, 3);
            assert_eq!(caller_level, 1);
        } else {
            panic!("expected SecurityLabel denial");
        }
    }

    #[test]
    fn test_payment_gate_blocks_unpaid() {
        let ctx = AccessContext::remote("bob", 1);
        let policy = policy_public_read();
        let gate = PaymentGate::Fixed(5.0);
        let result = check_read_access(Some(&policy), &ctx, "paid_schema", Some(&gate));
        assert!(result.is_denied());
    }

    #[test]
    fn test_payment_gate_allows_paid() {
        let mut ctx = AccessContext::remote("bob", 1);
        ctx.paid_schemas.insert("paid_schema".to_string());
        let policy = policy_public_read();
        let gate = PaymentGate::Fixed(5.0);
        assert!(check_read_access(Some(&policy), &ctx, "paid_schema", Some(&gate)).is_granted());
    }

    #[test]
    fn test_unresolvable_trust_distance() {
        let ctx = AccessContext {
            user_id: "bob".into(),
            trust_distance: None,
            public_keys: vec![],
            paid_schemas: Default::default(),
            clearance_level: 0,
        };
        let policy = policy_public_read();
        let result = check_read_access(Some(&policy), &ctx, "schema", None);
        assert!(result.is_denied());
    }

    #[test]
    fn test_all_layers_combined() {
        // A field that requires: distance <= 5, RX capability, clearance >= 2, payment
        let policy = FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::new(5, 0),
            capabilities: vec![CapabilityConstraint::new(
                "pk_bob",
                CapabilityKind::Read,
                10,
            )],
            security_label: Some(SecurityLabel::new(2, "sensitive")),
            ..Default::default()
        };
        let gate = PaymentGate::Fixed(1.0);

        let mut ctx = AccessContext::remote("bob", 3);
        ctx.public_keys = vec!["pk_bob".to_string()];
        ctx.clearance_level = 5;
        ctx.paid_schemas.insert("schema".to_string());

        // All layers pass
        assert!(check_read_access(Some(&policy), &ctx, "schema", Some(&gate)).is_granted());

        // Remove payment → denied
        ctx.paid_schemas.clear();
        assert!(check_read_access(Some(&policy), &ctx, "schema", Some(&gate)).is_denied());
    }
}
