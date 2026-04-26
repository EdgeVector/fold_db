//! Override molecule for transform fields.
//!
//! When a user directly writes a value to an irreversible transform field
//! (a transform whose WASM has no inverse), the write cannot be reduced to
//! a source mutation. Instead it is recorded as a `TransformFieldOverride`
//! molecule. The override carries the supplied value, marks the source link
//! stale, and records writer provenance for last-writer-wins reconciliation
//! across replicas.
//!
//! The on-disk shape is a normal serde-serializable struct so the override
//! syncs through the same encrypted log path as any other molecule.
//!
//! # State machine
//!
//! Per `transform_views_design.md` a transform field has three states:
//! `Empty`, `Cached`, `Overridden`. This module owns the data shape for
//! the `Overridden` state. The `Empty`/`Cached` lifecycle was driven by
//! the per-view cache (`ViewCacheState`), which the cache-deletion
//! follow-up of `projects/view-compute-as-mutations` retired in favor of
//! atom-store reads — so this module is intentionally untouched but the
//! companion cache no longer exists.
//!
//! # LWW
//!
//! Two replicas that override the same `(view, field, key)` tuple settle by
//! `written_at`. The newer wins. Source mutations against the *source* field
//! never beat an override — the override is sticky by virtue of being checked
//! first on the read path.

use serde::{Deserialize, Serialize};

/// Direct write to an irreversible transform field.
///
/// Stored per-(view, field, key). Participates in LWW like any other
/// molecule. `source_link_stale` is `true` for the lifetime of the override:
/// once a user has pinned a value here, future source mutations no longer
/// invalidate the field (per the 3-state machine in
/// `transform_views_design.md`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformFieldOverride {
    /// User-supplied override value.
    pub value: serde_json::Value,
    /// Once `true`, source mutations stop driving this field. Always `true`
    /// for newly-created overrides; the field exists so callers can reason
    /// about the flag explicitly rather than implying it from presence.
    pub source_link_stale: bool,
    /// Nanos since Unix epoch. Sole input to LWW comparison.
    pub written_at: u64,
    /// Base64-encoded public key of the writer.
    pub writer_pubkey: String,
    /// Base64-encoded signature over canonical bytes. Optional today —
    /// signing-key plumbing arrives with the broader provenance work; the
    /// shape is reserved so callers downstream can verify once it's wired.
    #[serde(default)]
    pub signature: String,
    /// Signature scheme version (`0` = unsigned, `1+` = future schemes).
    #[serde(default)]
    pub signature_version: u8,
}

impl TransformFieldOverride {
    /// Build a fresh override stamped with the current wall clock.
    pub fn new(value: serde_json::Value, writer_pubkey: impl Into<String>) -> Self {
        Self {
            value,
            source_link_stale: true,
            written_at: now_nanos(),
            writer_pubkey: writer_pubkey.into(),
            signature: String::new(),
            signature_version: 0,
        }
    }

    /// Build an override with an explicit timestamp. Tests use this to drive
    /// LWW deterministically; callers in production paths should prefer
    /// `new()` so the write reflects real wall-clock order.
    pub fn with_timestamp(
        value: serde_json::Value,
        writer_pubkey: impl Into<String>,
        written_at: u64,
    ) -> Self {
        Self {
            value,
            source_link_stale: true,
            written_at,
            writer_pubkey: writer_pubkey.into(),
            signature: String::new(),
            signature_version: 0,
        }
    }

    /// LWW: should `incoming` overwrite `existing`?
    ///
    /// Strictly newer `written_at` wins. Ties break on `writer_pubkey` to
    /// keep the result deterministic across replicas — without that, two
    /// nodes seeing the same pair in different orders could disagree on
    /// which they kept locally.
    pub fn should_replace(existing: &Self, incoming: &Self) -> bool {
        match incoming.written_at.cmp(&existing.written_at) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => incoming.writer_pubkey > existing.writer_pubkey,
        }
    }
}

fn now_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn new_sets_stale_and_pubkey() {
        let o = TransformFieldOverride::new(json!("v"), "pkA");
        assert!(o.source_link_stale);
        assert_eq!(o.value, json!("v"));
        assert_eq!(o.writer_pubkey, "pkA");
        assert_eq!(o.signature_version, 0);
        assert!(o.written_at > 0);
    }

    #[test]
    fn serde_round_trip_preserves_all_fields() {
        let original = TransformFieldOverride {
            value: json!({"nested": [1, 2, 3]}),
            source_link_stale: true,
            written_at: 1_700_000_000_000_000_000,
            writer_pubkey: "base64key==".to_string(),
            signature: "sig".to_string(),
            signature_version: 1,
        };
        let bytes = serde_json::to_vec(&original).unwrap();
        let decoded: TransformFieldOverride = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn serde_back_compat_defaults_signature_fields() {
        // An override persisted before signature plumbing existed should
        // still deserialize. signature/signature_version default.
        let json_str = r#"{
            "value": "hello",
            "source_link_stale": true,
            "written_at": 42,
            "writer_pubkey": "pk"
        }"#;
        let decoded: TransformFieldOverride = serde_json::from_str(json_str).unwrap();
        assert_eq!(decoded.signature, "");
        assert_eq!(decoded.signature_version, 0);
        assert_eq!(decoded.written_at, 42);
    }

    #[test]
    fn lww_newer_replaces_older() {
        let older = TransformFieldOverride::with_timestamp(json!("a"), "pkA", 100);
        let newer = TransformFieldOverride::with_timestamp(json!("b"), "pkB", 101);
        assert!(TransformFieldOverride::should_replace(&older, &newer));
        assert!(!TransformFieldOverride::should_replace(&newer, &older));
    }

    #[test]
    fn lww_equal_timestamp_breaks_on_pubkey() {
        // Same wall clock on two replicas — pick the lexicographically
        // larger pubkey so both replicas converge.
        let a = TransformFieldOverride::with_timestamp(json!("a"), "pkA", 100);
        let b = TransformFieldOverride::with_timestamp(json!("b"), "pkB", 100);
        assert!(TransformFieldOverride::should_replace(&a, &b));
        assert!(!TransformFieldOverride::should_replace(&b, &a));
    }

    #[test]
    fn lww_identical_writer_does_not_replace() {
        // Same writer, same timestamp → no churn. Idempotent re-delivery
        // of the same override should not bump anything.
        let a = TransformFieldOverride::with_timestamp(json!("a"), "pk", 100);
        let b = TransformFieldOverride::with_timestamp(json!("a"), "pk", 100);
        assert!(!TransformFieldOverride::should_replace(&a, &b));
    }
}
