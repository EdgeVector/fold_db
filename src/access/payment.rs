use serde::{Deserialize, Serialize};

use super::types::{AccessContext, AccessDecision, AccessDenialReason};

/// Payment gate formula for schema-level access pricing.
///
/// Cost is a function of trust distance τ:
/// - `Linear`: C(τ) = base + per_distance × τ
/// - `Exponential`: C(τ) = base × e^(growth × τ)
/// - `Fixed`: C(τ) = cost (constant regardless of distance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentGate {
    Linear { base: f64, per_distance: f64 },
    Exponential { base: f64, growth: f64 },
    Fixed(f64),
}

impl PaymentGate {
    /// Calculate the cost for a given trust distance.
    pub fn cost(&self, trust_distance: u64) -> f64 {
        let tau = trust_distance as f64;
        match self {
            PaymentGate::Linear { base, per_distance } => base + per_distance * tau,
            PaymentGate::Exponential { base, growth } => base * (growth * tau).exp(),
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

    let trust_distance = context.trust_distance.unwrap_or(u64::MAX);
    let cost = gate.cost(trust_distance);

    // Owner (distance 0) with zero-cost gate passes for free
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
        assert!((gate.cost(0) - 5.0).abs() < f64::EPSILON);
        assert!((gate.cost(100) - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_linear_cost() {
        let gate = PaymentGate::Linear {
            base: 1.0,
            per_distance: 0.5,
        };
        assert!((gate.cost(0) - 1.0).abs() < f64::EPSILON);
        assert!((gate.cost(2) - 2.0).abs() < f64::EPSILON);
        assert!((gate.cost(10) - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_exponential_cost() {
        let gate = PaymentGate::Exponential {
            base: 1.0,
            growth: 1.0,
        };
        // C(0) = 1.0 * e^0 = 1.0
        assert!((gate.cost(0) - 1.0).abs() < 0.01);
        // C(1) = 1.0 * e^1 ≈ 2.718
        assert!((gate.cost(1) - std::f64::consts::E).abs() < 0.01);
    }

    #[test]
    fn test_check_payment_paid() {
        let gate = PaymentGate::Fixed(10.0);
        let mut paid = HashSet::new();
        paid.insert("my_schema".to_string());
        let ctx = AccessContext {
            user_id: "bob".into(),
            trust_distance: Some(5),
            trust_distances: Default::default(),
            public_keys: vec![],
            paid_schemas: paid,
            clearance_level: 0,
        };
        assert!(check_payment(&gate, &ctx, "my_schema").is_granted());
    }

    #[test]
    fn test_check_payment_not_paid() {
        let gate = PaymentGate::Fixed(10.0);
        let ctx = AccessContext::remote("bob", 5);
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
        let gate = PaymentGate::Linear {
            base: 0.0,
            per_distance: 0.0,
        };
        let ctx = AccessContext::remote("bob", 5);
        assert!(check_payment(&gate, &ctx, "my_schema").is_granted());
    }

    #[test]
    fn test_serialization() {
        let gate = PaymentGate::Exponential {
            base: 2.0,
            growth: 0.5,
        };
        let json = serde_json::to_string(&gate).unwrap();
        let deserialized: PaymentGate = serde_json::from_str(&json).unwrap();
        assert!((deserialized.cost(3) - gate.cost(3)).abs() < f64::EPSILON);
    }
}
