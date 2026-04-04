//! Conflict detection and resolution for org sync.
//!
//! When multiple org members write to the same key between sync cycles,
//! the sync engine detects the conflict and applies Last-Write-Wins (LWW)
//! using `(timestamp_ms, device_id)` as the total order.
//!
//! All conflicts are recorded in a persistent log for auditing and
//! optional manual resolution.

use serde::{Deserialize, Serialize};

/// A record of a detected conflict during org sync replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRecord {
    /// Unique ID for this conflict (UUID).
    pub id: String,
    /// The storage namespace where the conflict occurred.
    pub namespace: String,
    /// The key (base64-encoded) that was written concurrently.
    pub key: String,
    /// The winning entry (applied to storage).
    pub winner: ConflictSide,
    /// The losing entry (not applied, preserved here for audit/override).
    pub loser: ConflictSide,
    /// How the conflict was resolved.
    pub resolution: ConflictResolution,
    /// When the conflict was detected (millis since epoch).
    pub detected_at_ms: u64,
    /// Org hash, if this was an org-scoped conflict.
    pub org_hash: Option<String>,
}

/// One side of a conflict (either the winner or loser).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictSide {
    pub timestamp_ms: u64,
    pub device_id: String,
    /// The value (base64-encoded) that this side wrote. None for deletes.
    pub value: Option<String>,
    pub seq: u64,
}

/// How a conflict was resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Automatically resolved by LWW comparison.
    LastWriteWins,
    /// Manually overridden via the resolution API.
    ManualOverride { resolved_at_ms: u64 },
}

/// Per-key write metadata, stored alongside data for conflict detection.
///
/// When a remote entry is replayed, we check if metadata exists for that key.
/// If it does and the device_id differs, we compare timestamps for LWW.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteMeta {
    pub timestamp_ms: u64,
    pub device_id: String,
    pub seq: u64,
}

/// Returns true if the incoming write wins over the local write.
///
/// Tiebreaker: higher timestamp wins. If timestamps are equal,
/// the lexicographically greater device_id wins.
pub fn lww_wins(
    incoming_ts: u64,
    incoming_device: &str,
    local_ts: u64,
    local_device: &str,
) -> bool {
    (incoming_ts, incoming_device) > (local_ts, local_device)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incoming_newer_wins() {
        assert!(lww_wins(200, "dev-a", 100, "dev-b"));
    }

    #[test]
    fn incoming_older_loses() {
        assert!(!lww_wins(100, "dev-a", 200, "dev-b"));
    }

    #[test]
    fn same_timestamp_higher_device_wins() {
        assert!(lww_wins(100, "dev-b", 100, "dev-a"));
    }

    #[test]
    fn same_timestamp_lower_device_loses() {
        assert!(!lww_wins(100, "dev-a", 100, "dev-b"));
    }

    #[test]
    fn identical_entries_local_wins() {
        // Same timestamp, same device: incoming does NOT win (returns false)
        assert!(!lww_wins(100, "dev-a", 100, "dev-a"));
    }
}
