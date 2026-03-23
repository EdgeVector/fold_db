use serde::{Deserialize, Serialize};

/// Information-flow security label using lattice ordering.
///
/// Higher level = more classified. Data can only flow from lower to higher
/// (or equal) levels, never downward. The `flows_to` method implements this check.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SecurityLabel {
    /// Classification level (0 = public, higher = more classified)
    pub level: u32,
    /// Human-readable category (e.g., "health", "financial", "public")
    pub category: String,
}

impl SecurityLabel {
    pub fn new(level: u32, category: impl Into<String>) -> Self {
        Self {
            level,
            category: category.into(),
        }
    }

    /// Public (level 0, no category restriction)
    pub fn public() -> Self {
        Self::new(0, "public")
    }

    /// Lattice ordering: self can flow to other iff self.level <= other.level.
    /// This prevents data from being downclassified through views or transforms.
    pub fn flows_to(&self, other: &SecurityLabel) -> bool {
        self.level <= other.level
    }

    /// Check if a caller with the given clearance level can read this label
    pub fn allows_read(&self, clearance_level: u32) -> bool {
        self.level <= clearance_level
    }
}

impl PartialOrd for SecurityLabel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SecurityLabel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.level.cmp(&other.level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_label() {
        let label = SecurityLabel::public();
        assert_eq!(label.level, 0);
        assert_eq!(label.category, "public");
    }

    #[test]
    fn test_flows_to_same_level() {
        let a = SecurityLabel::new(2, "health");
        let b = SecurityLabel::new(2, "financial");
        assert!(a.flows_to(&b));
        assert!(b.flows_to(&a));
    }

    #[test]
    fn test_flows_to_higher() {
        let low = SecurityLabel::new(1, "public");
        let high = SecurityLabel::new(3, "classified");
        assert!(low.flows_to(&high));
        assert!(!high.flows_to(&low));
    }

    #[test]
    fn test_allows_read() {
        let label = SecurityLabel::new(3, "secret");
        assert!(label.allows_read(3));
        assert!(label.allows_read(5));
        assert!(!label.allows_read(2));
    }

    #[test]
    fn test_ordering() {
        let a = SecurityLabel::new(1, "low");
        let b = SecurityLabel::new(3, "high");
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn test_serialization() {
        let label = SecurityLabel::new(2, "health");
        let json = serde_json::to_string(&label).unwrap();
        let deserialized: SecurityLabel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.level, 2);
        assert_eq!(deserialized.category, "health");
    }
}
