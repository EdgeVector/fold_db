use serde::{Deserialize, Serialize};

use super::types::{AccessContext, AccessDecision, AccessDenialReason};

/// Payment gate for schema-level access pricing.
///
/// Fixed cost regardless of trust tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentGate {
    Fixed(f64),
}

impl PaymentGate {
    /// Return the fixed cost.
    pub fn cost(&self) -> f64 {
        match self {
            PaymentGate::Fixed(cost) => *cost,
        }
    }
}

/// Check if the caller has paid for access to the given schema.
pub fn check_payment(
    gate: &PaymentGate,
    context: &AccessContext,
    schema_name: &str,
) -> AccessDecision {
    if context.paid_schemas.contains(schema_name) {
        return AccessDecision::Granted;
    }

    let cost = gate.cost();

    if cost <= 0.0 {
        return AccessDecision::Granted;
    }

    AccessDecision::Denied(AccessDenialReason::PaymentRequired { cost })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_fixed_cost() {
        let gate = PaymentGate::Fixed(5.0);
        assert!((gate.cost() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_check_payment_paid() {
        let gate = PaymentGate::Fixed(10.0);
        let mut paid = HashSet::new();
        paid.insert("my_schema".to_string());
        let ctx = AccessContext {
            user_id: "bob".into(),
            is_owner: false,
            tiers: Default::default(),
            public_keys: vec![],
            paid_schemas: paid,
        };
        assert!(check_payment(&gate, &ctx, "my_schema").is_granted());
    }

    #[test]
    fn test_check_payment_not_paid() {
        let gate = PaymentGate::Fixed(10.0);
        let ctx = AccessContext::remote("bob", Default::default());
        let result = check_payment(&gate, &ctx, "my_schema");
        assert!(result.is_denied());
        if let AccessDecision::Denied(AccessDenialReason::PaymentRequired { cost }) = result {
            assert!((cost - 10.0).abs() < f64::EPSILON);
        } else {
            panic!("expected PaymentRequired");
        }
    }

    #[test]
    fn test_check_payment_zero_cost() {
        let gate = PaymentGate::Fixed(0.0);
        let ctx = AccessContext::remote("bob", Default::default());
        assert!(check_payment(&gate, &ctx, "my_schema").is_granted());
    }

    #[test]
    fn test_serialization() {
        let gate = PaymentGate::Fixed(2.0);
        let json = serde_json::to_string(&gate).unwrap();
        let deserialized: PaymentGate = serde_json::from_str(&json).unwrap();
        assert!((deserialized.cost() - gate.cost()).abs() < f64::EPSILON);
    }
}
