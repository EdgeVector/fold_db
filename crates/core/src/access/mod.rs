//! Access control system implementing the three-layer model:
//!
//! 1. **Trust tier** — per-domain tier check (caller_tier >= field_min_tier)
//! 2. **Capability tokens** — cryptographic RX/WX tokens with bounded quotas
//! 3. **Payment gates** — fixed pricing for schema access
//!
//! All three layers are conjunctive: every applicable check must pass.

pub mod audit;
pub mod capability;
pub mod payment;
pub mod types;

pub use audit::{AuditAction, AuditEvent, AuditLog};
pub use capability::{CapabilityConstraint, CapabilityKind};
pub use payment::PaymentGate;
pub use types::{
    org_domain, trust_domain_for_data_domain, AccessContext, AccessDecision, AccessDenialReason,
    AccessMap, AccessTier, FieldAccessPolicy, DOMAIN_FAMILY, DOMAIN_FINANCIAL, DOMAIN_HEALTH,
    DOMAIN_MEDICAL, DOMAIN_PERSONAL,
};

/// Check all three access control layers for a single field.
///
/// If `policy` is `None`, the field defaults to owner-only access.
/// Payment gates are checked at the schema level (passed separately).
/// Set `is_write` to check write access instead of read.
pub fn check_access(
    policy: Option<&FieldAccessPolicy>,
    context: &AccessContext,
    schema_name: &str,
    payment_gate: Option<&PaymentGate>,
    is_write: bool,
) -> AccessDecision {
    let default_policy = FieldAccessPolicy::default();
    let policy = policy.unwrap_or(&default_policy);

    // Layer 1: Trust tier
    let caller_tier = match context.tier_for_domain(&policy.trust_domain) {
        Some(t) => t,
        None => {
            return AccessDecision::Denied(AccessDenialReason::NoDomainTrust {
                domain: policy.trust_domain.clone(),
            })
        }
    };

    let min_tier = if is_write {
        policy.min_write_tier
    } else {
        policy.min_read_tier
    };

    if caller_tier < min_tier {
        return AccessDecision::Denied(AccessDenialReason::InsufficientTrust {
            domain: policy.trust_domain.clone(),
            required: min_tier,
            actual: caller_tier,
        });
    }

    // Layer 2: Capability tokens
    match capability::check_capabilities(&policy.capabilities, context, is_write) {
        AccessDecision::Granted => {}
        denied => return denied,
    }

    // Layer 3: Payment gate (schema-level)
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
    use std::collections::HashMap;

    fn policy_public_read() -> FieldAccessPolicy {
        FieldAccessPolicy {
            min_read_tier: AccessTier::Public,
            min_write_tier: AccessTier::Owner,
            ..Default::default()
        }
    }

    fn policy_owner_only() -> FieldAccessPolicy {
        FieldAccessPolicy::default()
    }

    #[test]
    fn test_no_policy_defaults_to_owner_only() {
        let ctx = AccessContext::remote_single("bob", "personal", AccessTier::Trusted);
        assert!(check_access(None, &ctx, "schema", None, false).is_denied());
        assert!(check_access(None, &ctx, "schema", None, true).is_denied());

        let owner_ctx = AccessContext::owner("alice");
        assert!(check_access(None, &owner_ctx, "schema", None, false).is_granted());
        assert!(check_access(None, &owner_ctx, "schema", None, true).is_granted());
    }

    #[test]
    fn test_owner_always_has_access() {
        let ctx = AccessContext::owner("alice");
        let policy = policy_owner_only();
        assert!(check_access(Some(&policy), &ctx, "schema", None, false).is_granted());
        assert!(check_access(Some(&policy), &ctx, "schema", None, true).is_granted());
    }

    #[test]
    fn test_public_read_allows_any_tier() {
        let ctx = AccessContext::remote_single("bob", "personal", AccessTier::Public);
        let policy = policy_public_read();
        assert!(check_access(Some(&policy), &ctx, "schema", None, false).is_granted());
    }

    #[test]
    fn test_owner_only_blocks_remote_read() {
        let ctx = AccessContext::remote_single("bob", "personal", AccessTier::Inner);
        let policy = policy_owner_only();
        assert!(check_access(Some(&policy), &ctx, "schema", None, false).is_denied());
    }

    #[test]
    fn test_write_requires_higher_tier() {
        let ctx = AccessContext::remote_single("bob", "personal", AccessTier::Trusted);
        let policy = FieldAccessPolicy {
            min_read_tier: AccessTier::Outer,
            min_write_tier: AccessTier::Inner,
            ..Default::default()
        };
        // Trusted (2) >= Outer (1) → read granted
        assert!(check_access(Some(&policy), &ctx, "schema", None, false).is_granted());
        // Trusted (2) < Inner (3) → write denied
        assert!(check_access(Some(&policy), &ctx, "schema", None, true).is_denied());
    }

    #[test]
    fn test_no_domain_trust_denied() {
        let ctx = AccessContext::remote(
            "bob",
            HashMap::new(), // no domains
        );
        let policy = policy_public_read();
        let result = check_access(Some(&policy), &ctx, "schema", None, false);
        assert!(result.is_denied());
        if let AccessDecision::Denied(AccessDenialReason::NoDomainTrust { domain }) = result {
            assert_eq!(domain, "personal");
        } else {
            panic!("expected NoDomainTrust denial");
        }
    }

    #[test]
    fn test_payment_gate_blocks_unpaid() {
        let ctx = AccessContext::remote_single("bob", "personal", AccessTier::Inner);
        let policy = policy_public_read();
        let gate = PaymentGate::Fixed(5.0);
        assert!(check_access(Some(&policy), &ctx, "paid_schema", Some(&gate), false).is_denied());
    }

    #[test]
    fn test_payment_gate_allows_paid() {
        let mut ctx = AccessContext::remote_single("bob", "personal", AccessTier::Inner);
        ctx.paid_schemas.insert("paid_schema".to_string());
        let policy = policy_public_read();
        let gate = PaymentGate::Fixed(5.0);
        assert!(check_access(Some(&policy), &ctx, "paid_schema", Some(&gate), false).is_granted());
    }

    #[test]
    fn test_domain_aware_access_check() {
        let mut tiers = HashMap::new();
        tiers.insert("health".to_string(), AccessTier::Inner);
        tiers.insert("personal".to_string(), AccessTier::Trusted);
        let ctx = AccessContext::remote("bob", tiers);

        // Health field with min Trusted → Bob at Inner(3) → granted
        let health_policy = FieldAccessPolicy {
            trust_domain: "health".to_string(),
            min_read_tier: AccessTier::Trusted,
            min_write_tier: AccessTier::Owner,
            capabilities: vec![],
        };
        assert!(check_access(Some(&health_policy), &ctx, "schema", None, false).is_granted());

        // Personal field with min Inner → Bob at Trusted(2) → denied
        let personal_policy = FieldAccessPolicy {
            trust_domain: "personal".to_string(),
            min_read_tier: AccessTier::Inner,
            min_write_tier: AccessTier::Owner,
            capabilities: vec![],
        };
        assert!(check_access(Some(&personal_policy), &ctx, "schema", None, false).is_denied());

        // Financial field → Bob not in financial domain → denied
        let financial_policy = FieldAccessPolicy {
            trust_domain: "financial".to_string(),
            min_read_tier: AccessTier::Outer,
            min_write_tier: AccessTier::Owner,
            capabilities: vec![],
        };
        assert!(check_access(Some(&financial_policy), &ctx, "schema", None, false).is_denied());
    }

    #[test]
    fn test_all_layers_combined() {
        let policy = FieldAccessPolicy {
            min_read_tier: AccessTier::Outer,
            min_write_tier: AccessTier::Owner,
            capabilities: vec![CapabilityConstraint::new(
                "pk_bob",
                CapabilityKind::Read,
                10,
            )],
            ..Default::default()
        };
        let gate = PaymentGate::Fixed(1.0);

        let mut ctx = AccessContext::remote_single("bob", "personal", AccessTier::Inner);
        ctx.public_keys = vec!["pk_bob".to_string()];
        ctx.paid_schemas.insert("schema".to_string());

        // All layers pass
        assert!(check_access(Some(&policy), &ctx, "schema", Some(&gate), false).is_granted());

        // Remove payment → denied
        ctx.paid_schemas.clear();
        assert!(check_access(Some(&policy), &ctx, "schema", Some(&gate), false).is_denied());
    }
}
