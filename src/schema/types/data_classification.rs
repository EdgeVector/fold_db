use crate::access::{trust_domain_for_data_domain, TrustTier};
use serde::Serialize;

/// Standard sensitivity levels for data classification.
/// Higher values indicate greater sensitivity.
pub const PUBLIC: u8 = 0;
pub const INTERNAL: u8 = 1;
pub const CONFIDENTIAL: u8 = 2;
pub const RESTRICTED: u8 = 3;
pub const HIGHLY_RESTRICTED: u8 = 4;

/// Maximum allowed sensitivity level.
pub const MAX_SENSITIVITY_LEVEL: u8 = 4;

/// Two-dimensional data classification label for a field.
///
/// Every field carries a classification with two independent dimensions:
/// - **sensitivity_level**: How sensitive the data is (0=Public … 4=Highly Restricted)
/// - **data_domain**: What kind of data it is (e.g. "financial", "medical", "identity")
///
/// Labels are partially ordered by sensitivity within the same domain.
/// Labels with different domains are incomparable — cross-domain information
/// flow requires explicit authorization.
///
/// Validated on construction AND deserialization — sensitivity_level > 4
/// or empty data_domain will produce an error in both paths.
///
/// ```text
/// ┌───────┬───────────────────┬──────────────────────────────────────────────┐
/// │ Level │ Name              │ Description                                  │
/// ├───────┼───────────────────┼──────────────────────────────────────────────┤
/// │   0   │ Public            │ Freely distributable. No access restrictions. │
/// │   1   │ Internal          │ Not sensitive but not for public release.     │
/// │   2   │ Confidential      │ Business-sensitive. Competitive value.        │
/// │   3   │ Restricted        │ Personally identifiable or attributable.      │
/// │   4   │ Highly Restricted │ Regulated data (HIPAA, financial, biometric). │
/// └───────┴───────────────────┴──────────────────────────────────────────────┘
/// ```
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DataClassification {
    /// Sensitivity level: 0 (Public) through 4 (Highly Restricted).
    pub sensitivity_level: u8,
    /// Data domain tag identifying the type of data (e.g. "financial", "medical",
    /// "identity", "behavioral", "location", "general").
    pub data_domain: String,
}

// Custom Deserialize that validates sensitivity_level and data_domain,
// preventing invalid values from entering the system via JSON.
impl<'de> serde::Deserialize<'de> for DataClassification {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Helper {
            sensitivity_level: u8,
            data_domain: String,
        }
        let h = Helper::deserialize(deserializer)?;
        DataClassification::new(h.sensitivity_level, h.data_domain)
            .map_err(serde::de::Error::custom)
    }
}

impl DataClassification {
    /// Create a new DataClassification with validated sensitivity level.
    ///
    /// Returns `Err` if `sensitivity_level` exceeds `MAX_SENSITIVITY_LEVEL` (4)
    /// or if `data_domain` is empty.
    pub fn new(sensitivity_level: u8, data_domain: impl Into<String>) -> Result<Self, String> {
        if sensitivity_level > MAX_SENSITIVITY_LEVEL {
            return Err(format!(
                "sensitivity_level {} exceeds maximum {} (Highly Restricted)",
                sensitivity_level, MAX_SENSITIVITY_LEVEL
            ));
        }
        let data_domain = data_domain.into();
        if data_domain.trim().is_empty() {
            return Err("data_domain must not be empty".to_string());
        }
        Ok(Self {
            sensitivity_level,
            data_domain,
        })
    }

    /// Returns the human-readable name for this sensitivity level.
    pub fn sensitivity_name(&self) -> &'static str {
        match self.sensitivity_level {
            0 => "Public",
            1 => "Internal",
            2 => "Confidential",
            3 => "Restricted",
            4 => "Highly Restricted",
            _ => "Unknown",
        }
    }

    /// Convenience constructor for Low (Public, general domain).
    pub fn low() -> Self {
        Self {
            sensitivity_level: PUBLIC,
            data_domain: "general".to_string(),
        }
    }

    /// Convenience constructor for Medium (Confidential, general domain).
    pub fn medium() -> Self {
        Self {
            sensitivity_level: CONFIDENTIAL,
            data_domain: "general".to_string(),
        }
    }

    /// Convenience constructor for High (Highly Restricted, general domain).
    pub fn high() -> Self {
        Self {
            sensitivity_level: HIGHLY_RESTRICTED,
            data_domain: "general".to_string(),
        }
    }

    /// Returns the default trust tier for this classification's sensitivity level.
    pub fn default_trust_tier(&self) -> TrustTier {
        TrustTier::from_sensitivity(self.sensitivity_level)
    }

    /// Returns the trust domain that governs access for this classification's data domain.
    pub fn default_trust_domain(&self) -> &'static str {
        trust_domain_for_data_domain(&self.data_domain)
    }

    /// Information flow check: this classification can flow to `other` iff
    /// its sensitivity is less than or equal to `other`'s sensitivity.
    pub fn can_flow_to(&self, other: &DataClassification) -> bool {
        self.sensitivity_level <= other.sensitivity_level
    }
}

impl PartialOrd for DataClassification {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DataClassification {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sensitivity_level.cmp(&other.sensitivity_level)
    }
}

impl std::fmt::Display for DataClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.sensitivity_name(), self.data_domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_classification() {
        let c = DataClassification::new(0, "general").unwrap();
        assert_eq!(c.sensitivity_level, 0);
        assert_eq!(c.data_domain, "general");
        assert_eq!(c.sensitivity_name(), "Public");
    }

    #[test]
    fn test_max_level() {
        let c = DataClassification::new(4, "financial").unwrap();
        assert_eq!(c.sensitivity_level, 4);
        assert_eq!(c.sensitivity_name(), "Highly Restricted");
    }

    #[test]
    fn test_invalid_level() {
        let err = DataClassification::new(5, "general").unwrap_err();
        assert!(err.contains("exceeds maximum"));
    }

    #[test]
    fn test_invalid_level_large() {
        let err = DataClassification::new(255, "general").unwrap_err();
        assert!(err.contains("exceeds maximum"));
    }

    #[test]
    fn test_empty_domain() {
        let err = DataClassification::new(0, "").unwrap_err();
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn test_whitespace_domain() {
        let err = DataClassification::new(0, "   ").unwrap_err();
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn test_serialize_deserialize() {
        let c = DataClassification::new(3, "medical").unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let deserialized: DataClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(c, deserialized);
    }

    #[test]
    fn test_deserialize_rejects_invalid_level() {
        let json = r#"{"sensitivity_level": 99, "data_domain": "general"}"#;
        let result = serde_json::from_str::<DataClassification>(json);
        assert!(
            result.is_err(),
            "should reject sensitivity_level > 4 during deserialization"
        );
        let err = result.unwrap_err().to_string();
        assert!(err.contains("exceeds maximum"), "error: {}", err);
    }

    #[test]
    fn test_deserialize_rejects_empty_domain() {
        let json = r#"{"sensitivity_level": 0, "data_domain": ""}"#;
        let result = serde_json::from_str::<DataClassification>(json);
        assert!(
            result.is_err(),
            "should reject empty data_domain during deserialization"
        );
    }

    #[test]
    fn test_default_trust_tier() {
        use crate::access::TrustTier;

        let cases = [
            (0, TrustTier::Public),
            (1, TrustTier::Outer),
            (2, TrustTier::Trusted),
            (3, TrustTier::Inner),
            (4, TrustTier::Owner),
        ];
        for (level, expected_tier) in cases {
            let c = DataClassification::new(level, "general").unwrap();
            assert_eq!(c.default_trust_tier(), expected_tier, "level {}", level);
        }
    }

    #[test]
    fn test_default_trust_domain() {
        assert_eq!(
            DataClassification::new(4, "medical")
                .unwrap()
                .default_trust_domain(),
            "medical"
        );
        assert_eq!(
            DataClassification::new(4, "financial")
                .unwrap()
                .default_trust_domain(),
            "financial"
        );
        assert_eq!(
            DataClassification::new(3, "identity")
                .unwrap()
                .default_trust_domain(),
            "personal"
        );
        assert_eq!(
            DataClassification::new(0, "general")
                .unwrap()
                .default_trust_domain(),
            "personal"
        );
    }

    #[test]
    fn test_can_flow_to() {
        let low = DataClassification::new(0, "general").unwrap();
        let mid = DataClassification::new(2, "general").unwrap();
        let high = DataClassification::new(4, "general").unwrap();

        // same → same: ok
        assert!(mid.can_flow_to(&mid));
        // low → high: ok
        assert!(low.can_flow_to(&high));
        // high → low: not ok
        assert!(!high.can_flow_to(&low));
        // low → mid: ok
        assert!(low.can_flow_to(&mid));
        // mid → low: not ok
        assert!(!mid.can_flow_to(&low));
    }

    #[test]
    fn test_all_sensitivity_names() {
        assert_eq!(
            DataClassification::new(0, "g").unwrap().sensitivity_name(),
            "Public"
        );
        assert_eq!(
            DataClassification::new(1, "g").unwrap().sensitivity_name(),
            "Internal"
        );
        assert_eq!(
            DataClassification::new(2, "g").unwrap().sensitivity_name(),
            "Confidential"
        );
        assert_eq!(
            DataClassification::new(3, "g").unwrap().sensitivity_name(),
            "Restricted"
        );
        assert_eq!(
            DataClassification::new(4, "g").unwrap().sensitivity_name(),
            "Highly Restricted"
        );
    }
}
