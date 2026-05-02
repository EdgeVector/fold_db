use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Access tier — unified scale matching data sensitivity levels.
///
/// ```text
/// ┌───────┬───────────┬──────────────────┬─────────────────────────────────┐
/// │ Value │ Tier      │ Sensitivity      │ Example roles                   │
/// ├───────┼───────────┼──────────────────┼─────────────────────────────────┤
/// │   0   │ Public    │ Public           │ Anyone                          │
/// │   1   │ Outer     │ Internal         │ Acquaintance                    │
/// │   2   │ Trusted   │ Confidential     │ Friend, Trainer                 │
/// │   3   │ Inner     │ Restricted       │ Close friend, Family, Doctor    │
/// │   4   │ Owner     │ Highly Restricted│ Self (data owner)               │
/// └───────┴───────────┴──────────────────┴─────────────────────────────────┘
/// ```
///
/// Access check: `caller_tier >= field_min_tier`.
/// Higher tier = more access.
///
/// Name choice: "access" over "trust" because this type is an access-control
/// integer. The word "trust" is reserved for the informal concept (and for
/// trust-invite + org membership mechanisms, which are distinct from this).
/// See `docs/designs/platform_manifesto.md` for the full naming rationale.
#[cfg_attr(feature = "ts-bindings", derive(ts_rs::TS))]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
)]
#[repr(u8)]
pub enum AccessTier {
    Public = 0,
    Outer = 1,
    Trusted = 2,
    Inner = 3,
    Owner = 4,
}

impl AccessTier {
    /// Convert a sensitivity level (0-4) to the corresponding access tier.
    /// Panics if level > 4 (callers must validate via DataClassification).
    pub fn from_sensitivity(level: u8) -> Self {
        match level {
            0 => AccessTier::Public,
            1 => AccessTier::Outer,
            2 => AccessTier::Trusted,
            3 => AccessTier::Inner,
            4 => AccessTier::Owner,
            _ => panic!("invalid sensitivity level: {} (must be 0-4)", level),
        }
    }

    /// Return the numeric value (0-4).
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for AccessTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessTier::Public => write!(f, "Public"),
            AccessTier::Outer => write!(f, "Outer"),
            AccessTier::Trusted => write!(f, "Trusted"),
            AccessTier::Inner => write!(f, "Inner"),
            AccessTier::Owner => write!(f, "Owner"),
        }
    }
}

/// Flat access-graph map: public key → tier. One map per domain, stored in Sled.
pub type AccessMap = HashMap<String, AccessTier>;

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

/// Map a data classification domain to the trust domain that governs access.
///
/// Data domains are more granular — identity, location, behavioral all collapse
/// to the "personal" trust domain.
pub fn trust_domain_for_data_domain(data_domain: &str) -> &'static str {
    match data_domain {
        "medical" => DOMAIN_MEDICAL,
        "financial" => DOMAIN_FINANCIAL,
        "health" => DOMAIN_HEALTH,
        "family" => DOMAIN_FAMILY,
        "identity" | "location" | "behavioral" | "general" => DOMAIN_PERSONAL,
        _ => DOMAIN_PERSONAL,
    }
}

/// Context for evaluating access control decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessContext {
    /// Who is making the request (public key or user identifier)
    pub user_id: String,
    /// Whether this is the data owner (bypasses all tier checks)
    pub is_owner: bool,
    /// Per-domain trust tiers. Key = domain name, value = caller's tier.
    #[serde(default)]
    pub tiers: HashMap<String, AccessTier>,
    /// Caller's public keys (base64-encoded) for capability matching
    pub public_keys: Vec<String>,
    /// Schema names the caller has paid for
    pub paid_schemas: HashSet<String>,
}

impl AccessContext {
    /// Resolve trust tier for a specific domain.
    /// Owner always returns Owner. Otherwise looks up the tiers map.
    pub fn tier_for_domain(&self, domain: &str) -> Option<AccessTier> {
        if self.is_owner {
            return Some(AccessTier::Owner);
        }
        self.tiers.get(domain).copied()
    }

    /// Create an owner context (full access in all domains)
    pub fn owner(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            is_owner: true,
            tiers: HashMap::new(),
            public_keys: Vec::new(),
            paid_schemas: HashSet::new(),
        }
    }

    /// Create a remote context with per-domain tiers.
    pub fn remote(user_id: impl Into<String>, tiers: HashMap<String, AccessTier>) -> Self {
        Self {
            user_id: user_id.into(),
            is_owner: false,
            tiers,
            public_keys: Vec::new(),
            paid_schemas: HashSet::new(),
        }
    }

    /// Create a remote context with a single domain tier (convenience).
    pub fn remote_single(user_id: impl Into<String>, domain: &str, tier: AccessTier) -> Self {
        let mut tiers = HashMap::new();
        tiers.insert(domain.to_string(), tier);
        Self::remote(user_id, tiers)
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

/// Why access was denied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessDenialReason {
    InsufficientTrust {
        domain: String,
        required: AccessTier,
        actual: AccessTier,
    },
    NoDomainTrust {
        domain: String,
    },
    CapabilityMissing {
        kind: super::capability::CapabilityKind,
    },
    CapabilityExhausted {
        kind: super::capability::CapabilityKind,
    },
    PaymentRequired {
        cost: f64,
    },
}

impl fmt::Display for AccessDenialReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientTrust {
                domain,
                required,
                actual,
            } => {
                write!(
                    f,
                    "insufficient trust in domain '{}': required {} ({}), actual {} ({})",
                    domain,
                    required.as_u8(),
                    required,
                    actual.as_u8(),
                    actual
                )
            }
            Self::NoDomainTrust { domain } => {
                write!(f, "no trust relationship in domain '{}'", domain)
            }
            Self::CapabilityMissing { kind } => {
                write!(f, "missing required {:?} capability", kind)
            }
            Self::CapabilityExhausted { kind } => {
                write!(f, "{:?} capability quota exhausted", kind)
            }
            Self::PaymentRequired { cost } => {
                write!(f, "payment required: {:.4}", cost)
            }
        }
    }
}

/// Per-field access policy combining trust tier and capability checks.
/// Attached to `FieldCommon`. If `None`, field uses default (owner-only).
#[cfg_attr(feature = "ts-bindings", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FieldAccessPolicy {
    /// Which trust domain governs this field's access.
    /// Default: "personal".
    #[serde(default = "default_trust_domain")]
    pub trust_domain: String,
    /// Minimum trust tier required to read this field.
    /// Default: Owner (only the data owner can read).
    #[serde(default = "default_tier")]
    pub min_read_tier: AccessTier,
    /// Minimum trust tier required to write this field.
    /// Default: Owner (only the data owner can write).
    #[serde(default = "default_tier")]
    pub min_write_tier: AccessTier,
    /// Capability tokens required for access
    pub capabilities: Vec<super::capability::CapabilityConstraint>,
}

fn default_trust_domain() -> String {
    DOMAIN_PERSONAL.to_string()
}

fn default_tier() -> AccessTier {
    AccessTier::Owner
}

impl Default for FieldAccessPolicy {
    fn default() -> Self {
        Self {
            trust_domain: default_trust_domain(),
            min_read_tier: AccessTier::Owner,
            min_write_tier: AccessTier::Owner,
            capabilities: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_tier_ordering() {
        assert!(AccessTier::Public < AccessTier::Outer);
        assert!(AccessTier::Outer < AccessTier::Trusted);
        assert!(AccessTier::Trusted < AccessTier::Inner);
        assert!(AccessTier::Inner < AccessTier::Owner);
    }

    #[test]
    fn test_trust_tier_from_sensitivity() {
        assert_eq!(AccessTier::from_sensitivity(0), AccessTier::Public);
        assert_eq!(AccessTier::from_sensitivity(1), AccessTier::Outer);
        assert_eq!(AccessTier::from_sensitivity(2), AccessTier::Trusted);
        assert_eq!(AccessTier::from_sensitivity(3), AccessTier::Inner);
        assert_eq!(AccessTier::from_sensitivity(4), AccessTier::Owner);
    }

    #[test]
    #[should_panic(expected = "invalid sensitivity level")]
    fn test_trust_tier_from_invalid_sensitivity() {
        AccessTier::from_sensitivity(5);
    }

    #[test]
    fn test_trust_tier_as_u8() {
        assert_eq!(AccessTier::Public.as_u8(), 0);
        assert_eq!(AccessTier::Owner.as_u8(), 4);
    }

    #[test]
    fn test_trust_domain_for_data_domain() {
        assert_eq!(trust_domain_for_data_domain("medical"), "medical");
        assert_eq!(trust_domain_for_data_domain("financial"), "financial");
        assert_eq!(trust_domain_for_data_domain("health"), "health");
        assert_eq!(trust_domain_for_data_domain("family"), "family");
        assert_eq!(trust_domain_for_data_domain("identity"), "personal");
        assert_eq!(trust_domain_for_data_domain("location"), "personal");
        assert_eq!(trust_domain_for_data_domain("behavioral"), "personal");
        assert_eq!(trust_domain_for_data_domain("general"), "personal");
        assert_eq!(trust_domain_for_data_domain("unknown"), "personal");
    }

    #[test]
    fn test_owner_context() {
        let ctx = AccessContext::owner("alice");
        assert_eq!(ctx.user_id, "alice");
        assert!(ctx.is_owner);
        assert_eq!(ctx.tier_for_domain("personal"), Some(AccessTier::Owner));
        assert_eq!(ctx.tier_for_domain("health"), Some(AccessTier::Owner));
        assert_eq!(ctx.tier_for_domain("anything"), Some(AccessTier::Owner));
    }

    #[test]
    fn test_remote_context() {
        let mut tiers = HashMap::new();
        tiers.insert("personal".to_string(), AccessTier::Trusted);
        tiers.insert("medical".to_string(), AccessTier::Inner);
        let ctx = AccessContext::remote("bob", tiers);
        assert!(!ctx.is_owner);
        assert_eq!(ctx.tier_for_domain("personal"), Some(AccessTier::Trusted));
        assert_eq!(ctx.tier_for_domain("medical"), Some(AccessTier::Inner));
        assert_eq!(ctx.tier_for_domain("financial"), None);
    }

    #[test]
    fn test_remote_single_context() {
        let ctx = AccessContext::remote_single("bob", "health", AccessTier::Trusted);
        assert_eq!(ctx.tier_for_domain("health"), Some(AccessTier::Trusted));
        assert_eq!(ctx.tier_for_domain("personal"), None);
    }

    #[test]
    fn test_access_decision_helpers() {
        assert!(AccessDecision::Granted.is_granted());
        assert!(!AccessDecision::Granted.is_denied());
        let denied = AccessDecision::Denied(AccessDenialReason::NoDomainTrust {
            domain: "medical".to_string(),
        });
        assert!(denied.is_denied());
        assert!(!denied.is_granted());
    }

    #[test]
    fn test_denial_reason_display() {
        let reason = AccessDenialReason::InsufficientTrust {
            domain: "health".to_string(),
            required: AccessTier::Inner,
            actual: AccessTier::Outer,
        };
        let msg = format!("{}", reason);
        assert!(msg.contains("health"));
        assert!(msg.contains("required 3"));
        assert!(msg.contains("actual 1"));
    }

    #[test]
    fn test_field_access_policy_default() {
        let policy = FieldAccessPolicy::default();
        assert_eq!(policy.trust_domain, "personal");
        assert_eq!(policy.min_read_tier, AccessTier::Owner);
        assert_eq!(policy.min_write_tier, AccessTier::Owner);
        assert!(policy.capabilities.is_empty());
    }

    #[test]
    fn test_field_access_policy_serialization() {
        let policy = FieldAccessPolicy {
            min_read_tier: AccessTier::Trusted,
            min_write_tier: AccessTier::Inner,
            ..Default::default()
        };
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: FieldAccessPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.min_read_tier, AccessTier::Trusted);
        assert_eq!(deserialized.min_write_tier, AccessTier::Inner);
    }

    #[test]
    fn test_org_domain() {
        assert_eq!(org_domain("abc123"), "org:abc123");
    }

    #[test]
    fn test_trust_tier_access_check_logic() {
        assert!(AccessTier::Owner >= AccessTier::Owner);
        assert!(AccessTier::Owner >= AccessTier::Public);
        assert!(AccessTier::Inner >= AccessTier::Inner);
        assert!(AccessTier::Inner >= AccessTier::Trusted);
        assert!(AccessTier::Outer < AccessTier::Inner);
        assert!(AccessTier::Public < AccessTier::Outer);
    }
}
