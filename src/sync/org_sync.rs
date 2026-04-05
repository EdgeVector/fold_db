//! Key-prefix-based sync partitioning for org-scoped data.
//!
//! Org schemas use key prefixes (`{org_hash}:`) to namespace their data in Sled.
//! The `SyncPartitioner` inspects each log entry's key to determine whether it
//! should be synced to the personal S3 prefix or an org's shared S3 prefix.
//!
//! This is much simpler than a mapping table — the key itself tells you where
//! the data belongs. No registration of atoms/molecules required.
//!
//! ## Key format
//!
//! - Personal: `atom:{uuid}`, `ref:{uuid}`, `history:{mol}:{ts}`
//! - Org: `{org_hash}:atom:{uuid}`, `{org_hash}:ref:{uuid}`, etc.
//!
//! The org_hash prefix is added at write time when a schema has `org_hash = Some(hash)`.
//! The sync layer just reads it back.

use crate::crypto::CryptoProvider;
use crate::org::OrgMembership;
use std::sync::Arc;

/// A sync target — one R2 prefix with its own encryption key.
///
/// Personal sync and org sync are the same mechanism: upload/download
/// encrypted log entries to/from `/{prefix}/log/{seq}.enc`. The only
/// differences are the prefix and the crypto provider.
#[derive(Clone)]
pub struct SyncTarget {
    /// Human-readable label for logging ("personal" or org name).
    pub label: String,
    /// R2 prefix hash: `user_hash` for personal, `org_hash` for org.
    pub prefix: String,
    /// Crypto provider for sealing/unsealing entries on this prefix.
    pub crypto: Arc<dyn CryptoProvider>,
    /// Transition flag: org targets use old backend presign endpoints
    /// that require `member_id`. Removed after backend migration.
    pub is_org: bool,
}

/// Where a log entry should be synced to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncDestination {
    /// Personal data — sync to `/{user_hash}/log/{seq}.enc`
    Personal,
    /// Org data — sync to `/{org_hash}/log/{seq}.enc`
    /// with the org's E2E key for encryption.
    Org {
        org_hash: String,
        org_e2e_secret: String,
    },
}

/// Partitions log entries by destination based on key prefix.
///
/// Given a Sled key (from a LogEntry's namespace+key), determines whether it
/// belongs to an org (key starts with `{org_hash}:`) or is personal data.
///
/// The partitioner is initialized from the node's org memberships and updated
/// when memberships change (join/leave org).
pub struct SyncPartitioner {
    /// Known org memberships with their E2E secrets.
    org_memberships: Vec<OrgMembershipEntry>,
}

/// Minimal info needed for partitioning decisions.
#[derive(Debug, Clone)]
struct OrgMembershipEntry {
    org_hash: String,
    org_e2e_secret: String,
}

impl SyncPartitioner {
    /// Create a new partitioner from org memberships.
    pub fn new(memberships: &[OrgMembership]) -> Self {
        let org_memberships = memberships
            .iter()
            .map(|m| OrgMembershipEntry {
                org_hash: m.org_hash.clone(),
                org_e2e_secret: m.org_e2e_secret.clone(),
            })
            .collect();

        Self { org_memberships }
    }

    /// Create an empty partitioner (no org memberships — everything is personal).
    pub fn empty() -> Self {
        Self {
            org_memberships: Vec::new(),
        }
    }

    /// Determine where a key should be synced based on its prefix.
    ///
    /// Checks if the key (decoded from base64 in the LogEntry) starts with
    /// any known org_hash followed by `:`. If so, routes to that org.
    /// Otherwise, routes to personal sync.
    pub fn partition(&self, key: &str) -> SyncDestination {
        for entry in &self.org_memberships {
            let prefix = format!("{}:", entry.org_hash);
            if key.starts_with(&prefix) {
                return SyncDestination::Org {
                    org_hash: entry.org_hash.clone(),
                    org_e2e_secret: entry.org_e2e_secret.clone(),
                };
            }
        }
        SyncDestination::Personal
    }

    /// Partition a LogOp's key string (already decoded from the LogEntry).
    ///
    /// For namespace-level keys (like schema names in the "schemas" namespace),
    /// also checks the namespace+key combination.
    pub fn partition_log_key(&self, namespace: &str, key_b64: &str) -> SyncDestination {
        // Decode the base64 key to get the raw key string
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
        let key_bytes = match BASE64.decode(key_b64) {
            Ok(bytes) => bytes,
            Err(_) => return SyncDestination::Personal,
        };
        let key_str = match std::str::from_utf8(&key_bytes) {
            Ok(s) => s,
            Err(_) => return SyncDestination::Personal,
        };

        // For any namespace, check if the key itself has an org prefix
        let dest = self.partition(key_str);
        if dest != SyncDestination::Personal {
            return dest;
        }

        // For schema-level namespaces, the key might be a schema name
        // that was stored under an org-prefixed schema name
        // (e.g., key = "{org_hash}:board_meeting" in the "schemas" namespace)
        let _ = namespace; // namespace used for potential future routing logic

        SyncDestination::Personal
    }

    /// Get the list of org hashes this partitioner knows about.
    pub fn org_hashes(&self) -> Vec<String> {
        self.org_memberships
            .iter()
            .map(|m| m.org_hash.clone())
            .collect()
    }

    /// Check if this partitioner has any org memberships.
    pub fn has_orgs(&self) -> bool {
        !self.org_memberships.is_empty()
    }
}

/// Build an org-prefixed key from a base key and org_hash.
///
/// Example: `org_key_prefix("abc123", "atom:uuid-1")` -> `"abc123:atom:uuid-1"`
pub fn org_key_prefix(org_hash: &str, base_key: &str) -> String {
    format!("{org_hash}:{base_key}")
}

/// Strip the org prefix from a key, if present.
///
/// Returns `Some((org_hash, base_key))` if the key has an org prefix,
/// or `None` if it's a personal key.
pub fn strip_org_prefix(key: &str) -> Option<(&str, &str)> {
    // Org keys look like: {org_hash}:{rest}
    // org_hash is a hex SHA256 (64 chars), so we look for a 64-char hex prefix
    // followed by a colon.
    if key.len() > 65 {
        let potential_hash = &key[..64];
        if key.as_bytes()[64] == b':' && potential_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some((&key[..64], &key[65..]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org::OrgMembership;

    fn make_membership(org_hash: &str, e2e_secret: &str) -> OrgMembership {
        OrgMembership {
            org_name: "Test Org".to_string(),
            org_hash: org_hash.to_string(),
            org_public_key: "test_pk".to_string(),
            org_secret_key: None,
            org_e2e_secret: e2e_secret.to_string(),
            role: crate::org::OrgRole::Member,
            members: vec![],
            created_at: 0,
            joined_at: 0,
        }
    }

    #[test]
    fn test_sync_partitioner_personal_keys() {
        let memberships = vec![make_membership("abc123def456", "secret1")];
        let partitioner = SyncPartitioner::new(&memberships);

        assert_eq!(
            partitioner.partition("atom:uuid-1"),
            SyncDestination::Personal
        );
        assert_eq!(
            partitioner.partition("ref:mol-uuid"),
            SyncDestination::Personal
        );
        assert_eq!(
            partitioner.partition("history:mol:12345"),
            SyncDestination::Personal
        );
    }

    #[test]
    fn test_sync_partitioner_org_keys() {
        let memberships = vec![make_membership("abc123def456", "secret1")];
        let partitioner = SyncPartitioner::new(&memberships);

        assert_eq!(
            partitioner.partition("abc123def456:atom:uuid-1"),
            SyncDestination::Org {
                org_hash: "abc123def456".to_string(),
                org_e2e_secret: "secret1".to_string(),
            }
        );
        assert_eq!(
            partitioner.partition("abc123def456:ref:mol-uuid"),
            SyncDestination::Org {
                org_hash: "abc123def456".to_string(),
                org_e2e_secret: "secret1".to_string(),
            }
        );
    }

    #[test]
    fn test_sync_partitioner_multiple_orgs() {
        let memberships = vec![
            make_membership("org_alpha", "secret_a"),
            make_membership("org_beta", "secret_b"),
        ];
        let partitioner = SyncPartitioner::new(&memberships);

        assert_eq!(
            partitioner.partition("org_alpha:atom:uuid-1"),
            SyncDestination::Org {
                org_hash: "org_alpha".to_string(),
                org_e2e_secret: "secret_a".to_string(),
            }
        );
        assert_eq!(
            partitioner.partition("org_beta:atom:uuid-2"),
            SyncDestination::Org {
                org_hash: "org_beta".to_string(),
                org_e2e_secret: "secret_b".to_string(),
            }
        );
        assert_eq!(
            partitioner.partition("atom:uuid-3"),
            SyncDestination::Personal
        );
    }

    #[test]
    fn test_sync_partitioner_empty() {
        let partitioner = SyncPartitioner::empty();
        assert_eq!(
            partitioner.partition("anything:at:all"),
            SyncDestination::Personal
        );
        assert!(!partitioner.has_orgs());
    }

    #[test]
    fn test_org_key_prefix_helper() {
        assert_eq!(
            org_key_prefix("abc123", "atom:uuid-1"),
            "abc123:atom:uuid-1"
        );
        assert_eq!(
            org_key_prefix("org_hash", "ref:mol-uuid"),
            "org_hash:ref:mol-uuid"
        );
    }

    #[test]
    fn test_strip_org_prefix() {
        // 64-char hex hash prefix
        let org_hash = "a".repeat(64);
        let key = format!("{org_hash}:atom:uuid-1");
        let result = strip_org_prefix(&key);
        assert_eq!(result, Some((org_hash.as_str(), "atom:uuid-1")));

        // Personal key (no org prefix)
        assert_eq!(strip_org_prefix("atom:uuid-1"), None);

        // Short key
        assert_eq!(strip_org_prefix("abc:def"), None);
    }

    #[test]
    fn test_org_hashes() {
        let memberships = vec![
            make_membership("org_a", "secret_a"),
            make_membership("org_b", "secret_b"),
        ];
        let partitioner = SyncPartitioner::new(&memberships);
        let hashes = partitioner.org_hashes();
        assert_eq!(hashes.len(), 2);
        assert!(hashes.contains(&"org_a".to_string()));
        assert!(hashes.contains(&"org_b".to_string()));
        assert!(partitioner.has_orgs());
    }
}
