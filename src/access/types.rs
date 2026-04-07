use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Context for evaluating access control decisions.
/// Built from the authenticated request — local owner gets distance 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessContext {
    /// Who is making the request (public key or user identifier)
    pub user_id: String,
    /// Legacy single trust distance. Used when `trust_distances` is empty.
    pub trust_distance: Option<u64>,
    /// Per-domain trust distances. Key = domain name, value = resolved distance.
    /// If a domain is missing, the caller has no trust path in that domain.
    #[serde(default)]
    pub trust_distances: HashMap<String, u64>,
    /// Caller's public keys (base64-encoded) for capability matching
    pub public_keys: Vec<String>,
    /// Schema names the caller has paid for
    pub paid_schemas: HashSet<String>,
    /// Caller's security clearance level (0 = lowest)
    pub clearance_level: u32,
}

impl AccessContext {
    /// Resolve trust distance for a specific domain.
    /// Owner contexts always return Some(0).
    /// Falls back to legacy `trust_distance` when `trust_distances` is empty.
    pub fn distance_for_domain(&self, domain: &str) -> Option<u64> {
        // Owner check: clearance_level == u32::MAX is the owner sentinel
        if self.clearance_level == u32::MAX && self.trust_distance == Some(0) {
            return Some(0);
        }
        if !self.trust_distances.is_empty() {
            self.trust_distances.get(domain).copied()
        } else {
            self.trust_distance
        }
    }

    /// Create an owner context (distance 0 in all domains, full access)
    pub fn owner(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            trust_distance: Some(0),
            trust_distances: HashMap::new(),
            public_keys: Vec::new(),
            paid_schemas: HashSet::new(),
            clearance_level: u32::MAX,
        }
    }

    /// Create a remote context with a specific trust distance (backwards compatible).
    /// Stores the distance in both the legacy field and the "personal" domain.
    pub fn remote(user_id: impl Into<String>, trust_distance: u64) -> Self {
        let mut trust_distances = HashMap::new();
        trust_distances.insert(DOMAIN_PERSONAL.to_string(), trust_distance);
        Self {
            user_id: user_id.into(),
            trust_distance: Some(trust_distance),
            trust_distances,
            public_keys: Vec::new(),
            paid_schemas: HashSet::new(),
            clearance_level: 0,
        }
    }

    /// Create a remote context with per-domain distances.
    pub fn remote_multi(user_id: impl Into<String>, trust_distances: HashMap<String, u64>) -> Self {
        Self {
            user_id: user_id.into(),
            trust_distance: None,
            trust_distances,
            public_keys: Vec::new(),
            paid_schemas: HashSet::new(),
            clearance_level: 0,
        }
    }
}

/// Result of an access control evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessDecision {
    Granted,
    Denied(AccessDenialReason),
}

impl AccessDecision {
    pub fn is_granted(&self) -> bool {
        matches!(self, AccessDecision::Granted)
    }

    pub fn is_denied(&self) -> bool {
        matches!(self, AccessDecision::Denied(_))
    }
}

/// Why access was denied — provides enough detail for the caller to understand what's missing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessDenialReason {
    TrustDistance {
        required: u64,
        actual: u64,
    },
    CapabilityMissing {
        kind: super::capability::CapabilityKind,
    },
    CapabilityExhausted {
        kind: super::capability::CapabilityKind,
    },
    SecurityLabel {
        source_level: u32,
        caller_level: u32,
    },
    PaymentRequired {
        cost: f64,
    },
    TrustDistanceUnresolvable,
}

impl fmt::Display for AccessDenialReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TrustDistance { required, actual } => {
                write!(
                    f,
                    "trust distance too high: required <= {}, actual {}",
                    required, actual
                )
            }
            Self::CapabilityMissing { kind } => {
                write!(f, "missing required {:?} capability", kind)
            }
            Self::CapabilityExhausted { kind } => {
                write!(f, "{:?} capability quota exhausted", kind)
            }
            Self::SecurityLabel {
                source_level,
                caller_level,
            } => {
                write!(
                    f,
                    "security label mismatch: field level {} > caller clearance {}",
                    source_level, caller_level
                )
            }
            Self::PaymentRequired { cost } => {
                write!(f, "payment required: {:.4}", cost)
            }
            Self::TrustDistanceUnresolvable => {
                write!(
                    f,
                    "trust distance could not be resolved (no path in trust graph)"
                )
            }
        }
    }
}

/// Per-field trust distance policy defining max read/write distances.
/// Default: owner-only (both 0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustDistancePolicy {
    /// Maximum trust distance for read access (u64::MAX = public)
    pub read_max: u64,
    /// Maximum trust distance for write access (0 = owner only)
    pub write_max: u64,
}

impl TrustDistancePolicy {
    pub fn new(read_max: u64, write_max: u64) -> Self {
        Self {
            read_max,
            write_max,
        }
    }

    /// Owner-only: only distance 0 can read or write
    pub fn owner_only() -> Self {
        Self {
            read_max: 0,
            write_max: 0,
        }
    }

    /// Public read, owner-only write
    pub fn public_read() -> Self {
        Self {
            read_max: u64::MAX,
            write_max: 0,
        }
    }

    pub fn can_read(&self, trust_distance: u64) -> bool {
        trust_distance <= self.read_max
    }

    pub fn can_write(&self, trust_distance: u64) -> bool {
        trust_distance <= self.write_max
    }
}

impl Default for TrustDistancePolicy {
    fn default() -> Self {
        Self::owner_only()
    }
}

/// Well-known trust domain names.
pub const DOMAIN_PERSONAL: &str = "personal";
pub const DOMAIN_FAMILY: &str = "family";
pub const DOMAIN_FINANCIAL: &str = "financial";
pub const DOMAIN_HEALTH: &str = "health";
pub const DOMAIN_MEDICAL: &str = "medical";

/// Construct an org trust domain name from an org hash.
pub fn org_domain(org_hash: &str) -> String {
    format!("org:{}", org_hash)
}

/// Per-field access policy combining all four access control layers.
/// Attached to `FieldCommon`. If `None`, field uses legacy behavior (no checks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldAccessPolicy {
    /// Which trust domain governs this field's access.
    /// Default: "personal". Each domain has its own independent TrustGraph.
    #[serde(default = "default_trust_domain")]
    pub trust_domain: String,
    /// Trust distance bounds for read/write
    pub trust_distance: TrustDistancePolicy,
    /// Capability tokens required for access
    pub capabilities: Vec<super::capability::CapabilityConstraint>,
    /// Information flow security label
    pub security_label: Option<super::security_label::SecurityLabel>,
}

fn default_trust_domain() -> String {
    DOMAIN_PERSONAL.to_string()
}

impl Default for FieldAccessPolicy {
    fn default() -> Self {
        Self {
            trust_domain: default_trust_domain(),
            trust_distance: TrustDistancePolicy::default(),
            capabilities: Vec::new(),
            security_label: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owner_context() {
        let ctx = AccessContext::owner("alice");
        assert_eq!(ctx.user_id, "alice");
        assert_eq!(ctx.trust_distance, Some(0));
        assert_eq!(ctx.clearance_level, u32::MAX);
        // Owner has distance 0 in any domain
        assert_eq!(ctx.distance_for_domain("personal"), Some(0));
        assert_eq!(ctx.distance_for_domain("health"), Some(0));
        assert_eq!(ctx.distance_for_domain("financial"), Some(0));
    }

    #[test]
    fn test_remote_context() {
        let ctx = AccessContext::remote("bob", 3);
        assert_eq!(ctx.trust_distance, Some(3));
        assert_eq!(ctx.clearance_level, 0);
        // remote() stores in "personal" domain
        assert_eq!(ctx.distance_for_domain("personal"), Some(3));
        // Other domains: not present
        assert_eq!(ctx.distance_for_domain("health"), None);
    }

    #[test]
    fn test_remote_multi_context() {
        let mut distances = HashMap::new();
        distances.insert("personal".to_string(), 2);
        distances.insert("health".to_string(), 1);
        let ctx = AccessContext::remote_multi("bob", distances);
        assert_eq!(ctx.distance_for_domain("personal"), Some(2));
        assert_eq!(ctx.distance_for_domain("health"), Some(1));
        assert_eq!(ctx.distance_for_domain("financial"), None);
    }

    #[test]
    fn test_distance_for_domain_legacy_fallback() {
        // Empty trust_distances → falls back to trust_distance
        let ctx = AccessContext {
            user_id: "bob".into(),
            trust_distance: Some(5),
            trust_distances: HashMap::new(),
            public_keys: vec![],
            paid_schemas: HashSet::new(),
            clearance_level: 0,
        };
        assert_eq!(ctx.distance_for_domain("personal"), Some(5));
        assert_eq!(ctx.distance_for_domain("health"), Some(5));
    }

    #[test]
    fn test_trust_distance_policy_owner_only() {
        let p = TrustDistancePolicy::owner_only();
        assert!(p.can_read(0));
        assert!(!p.can_read(1));
        assert!(p.can_write(0));
        assert!(!p.can_write(1));
    }

    #[test]
    fn test_trust_distance_policy_public_read() {
        let p = TrustDistancePolicy::public_read();
        assert!(p.can_read(u64::MAX));
        assert!(p.can_write(0));
        assert!(!p.can_write(1));
    }

    #[test]
    fn test_trust_distance_policy_custom() {
        let p = TrustDistancePolicy::new(5, 2);
        assert!(p.can_read(5));
        assert!(!p.can_read(6));
        assert!(p.can_write(2));
        assert!(!p.can_write(3));
    }

    #[test]
    fn test_access_decision_helpers() {
        assert!(AccessDecision::Granted.is_granted());
        assert!(!AccessDecision::Granted.is_denied());
        let denied = AccessDecision::Denied(AccessDenialReason::TrustDistanceUnresolvable);
        assert!(denied.is_denied());
        assert!(!denied.is_granted());
    }

    #[test]
    fn test_denial_reason_display() {
        let reason = AccessDenialReason::TrustDistance {
            required: 3,
            actual: 5,
        };
        let msg = format!("{}", reason);
        assert!(msg.contains("required <= 3"));
        assert!(msg.contains("actual 5"));
    }

    #[test]
    fn test_field_access_policy_default() {
        let policy = FieldAccessPolicy::default();
        assert_eq!(policy.trust_distance, TrustDistancePolicy::owner_only());
        assert!(policy.capabilities.is_empty());
        assert!(policy.security_label.is_none());
    }

    #[test]
    fn test_field_access_policy_serialization() {
        let policy = FieldAccessPolicy {
            trust_distance: TrustDistancePolicy::new(10, 2),
            ..Default::default()
        };
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: FieldAccessPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.trust_distance.read_max, 10);
        assert_eq!(deserialized.trust_distance.write_max, 2);
    }
}
