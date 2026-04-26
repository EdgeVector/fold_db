use crate::atom::provenance::Provenance;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Records a single field-level change within a mutation.
/// Stored at key "history:{molecule_uuid}:{timestamp_nanos_padded}"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationEvent {
    pub molecule_uuid: String,
    pub timestamp: DateTime<Utc>,
    pub field_key: FieldKey,
    pub old_atom_uuid: Option<String>,
    pub new_atom_uuid: String,
    /// Molecule version at the time this event was recorded
    #[serde(default)]
    pub version: u64,
    /// Whether this event resulted from a merge conflict resolution.
    #[serde(default)]
    pub is_conflict: bool,
    /// The atom UUID that lost the conflict (if `is_conflict` is true).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_loser_atom: Option<String>,
    /// Base64-encoded public key of the writer at the time of the mutation.
    #[serde(default)]
    pub writer_pubkey: String,
    /// Base64-encoded Ed25519 signature from the molecule at the time of the mutation.
    #[serde(default)]
    pub signature: String,
    /// Writer identity and verifiability info. Additive during the
    /// `projects/molecule-provenance-dag` migration: propagated from the
    /// originating `Mutation.provenance` when available; `None` otherwise
    /// (including for merge-conflict-originated events). Kept alongside
    /// `writer_pubkey` / `signature` until a follow-up PR removes them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

/// Identifies which slot in the molecule was changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldKey {
    Single,
    Hash { hash: String },
    Range { range: String },
    HashRange { hash: String, range: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutation_event_serialization_roundtrip() {
        let event = MutationEvent {
            molecule_uuid: "mol-123".to_string(),
            timestamp: Utc::now(),
            field_key: FieldKey::Single,
            old_atom_uuid: None,
            new_atom_uuid: "atom-456".to_string(),
            version: 0,
            is_conflict: false,
            conflict_loser_atom: None,
            writer_pubkey: String::new(),
            signature: String::new(),
            provenance: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: MutationEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.molecule_uuid, "mol-123");
        assert_eq!(deserialized.new_atom_uuid, "atom-456");
        assert!(deserialized.old_atom_uuid.is_none());
    }

    #[test]
    fn test_field_key_variants() {
        let single = FieldKey::Single;
        let range = FieldKey::Range {
            range: "key1".to_string(),
        };
        let hash_range = FieldKey::HashRange {
            hash: "h1".to_string(),
            range: "r1".to_string(),
        };

        // Verify all variants serialize/deserialize correctly
        for key in [single, range, hash_range] {
            let json = serde_json::to_string(&key).unwrap();
            let _: FieldKey = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_mutation_event_with_old_atom() {
        let event = MutationEvent {
            molecule_uuid: "mol-abc".to_string(),
            timestamp: Utc::now(),
            field_key: FieldKey::HashRange {
                hash: "user1".to_string(),
                range: "post1".to_string(),
            },
            old_atom_uuid: Some("old-atom".to_string()),
            new_atom_uuid: "new-atom".to_string(),
            version: 0,
            is_conflict: false,
            conflict_loser_atom: None,
            writer_pubkey: String::new(),
            signature: String::new(),
            provenance: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: MutationEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.old_atom_uuid, Some("old-atom".to_string()));
        assert_eq!(deserialized.new_atom_uuid, "new-atom");
    }

    #[test]
    fn test_storage_key_format() {
        let event = MutationEvent {
            molecule_uuid: "mol-xyz".to_string(),
            timestamp: Utc::now(),
            field_key: FieldKey::Single,
            old_atom_uuid: None,
            new_atom_uuid: "atom-1".to_string(),
            version: 0,
            is_conflict: false,
            conflict_loser_atom: None,
            writer_pubkey: String::new(),
            signature: String::new(),
            provenance: None,
        };

        let ts = event.timestamp.timestamp_nanos_opt().unwrap_or(0);
        let key = format!("history:{}:{:020}", event.molecule_uuid, ts);

        // Key should start with history prefix
        assert!(key.starts_with("history:mol-xyz:"));
        // Timestamp part should be 20 digits (zero-padded)
        let parts: Vec<&str> = key.splitn(3, ':').collect();
        assert_eq!(parts[2].len(), 20);
    }

    // ------------------------------------------------------------------
    // molecule-provenance-dag PR 5 — additive `provenance: Option<Provenance>`.
    // ------------------------------------------------------------------

    /// Pre-PR-5 serialized shape — no `provenance` field. Deserializes to
    /// `None` and re-serializes byte-for-byte identical. Using a fixed
    /// epoch timestamp so the literal is stable.
    const GOLDEN_PRE_PR5_MUTATION_EVENT_JSON: &str = r#"{"molecule_uuid":"mol-1","timestamp":"2026-01-01T00:00:00Z","field_key":"Single","old_atom_uuid":null,"new_atom_uuid":"atom-1","version":0,"is_conflict":false,"writer_pubkey":"","signature":""}"#;

    #[test]
    fn pre_pr5_mutation_event_json_round_trips_unchanged() {
        let parsed: MutationEvent = serde_json::from_str(GOLDEN_PRE_PR5_MUTATION_EVENT_JSON)
            .expect("deserialize pre-PR-5 mutation event");
        assert!(parsed.provenance.is_none());
        let reserialized = serde_json::to_string(&parsed).expect("serialize");
        assert_eq!(reserialized, GOLDEN_PRE_PR5_MUTATION_EVENT_JSON);
    }

    #[test]
    fn mutation_event_round_trips_with_provenance_user() {
        let event = MutationEvent {
            molecule_uuid: "mol-1".to_string(),
            timestamp: Utc::now(),
            field_key: FieldKey::Single,
            old_atom_uuid: None,
            new_atom_uuid: "atom-1".to_string(),
            version: 0,
            is_conflict: false,
            conflict_loser_atom: None,
            writer_pubkey: "pk".to_string(),
            signature: "sig".to_string(),
            provenance: Some(Provenance::user("pk".to_string(), "sig".to_string())),
        };
        let json = serde_json::to_string(&event).expect("serialize");
        assert!(json.contains(r#""provenance":{"kind":"user""#));
        let back: MutationEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.provenance, event.provenance);
    }
}
