mod atom_def;
pub mod input_snapshot;
pub mod merkle;
mod molecule;
mod molecule_hash;
mod molecule_hash_range;
mod molecule_range;
mod molecule_tests;
pub mod mutation_event;
pub mod provenance;

pub use atom_def::Atom;
pub use molecule::Molecule;
pub use molecule_hash::MoleculeHash;
pub use molecule_hash_range::MoleculeHashRange;
pub use molecule_range::MoleculeRange;
pub use mutation_event::{FieldKey, MutationEvent};
pub use provenance::{MoleculeRef, Provenance};

/// An atom reference with per-key write timestamp for merge resolution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, utoipa::ToSchema)]
pub struct AtomEntry {
    pub atom_uuid: String,
    #[serde(default)]
    pub written_at: u64, // nanos since epoch
    /// Base64-encoded public key of the writer who signed this entry.
    #[serde(default)]
    pub writer_pubkey: String,
    /// Base64-encoded Ed25519 signature over canonical bytes.
    #[serde(default)]
    pub signature: String,
    /// Signature scheme version (1 = hand-rolled canonical concat).
    #[serde(default)]
    pub signature_version: u8,
    /// Writer identity and verifiability info. Additive during the
    /// `projects/molecule-provenance-dag` migration: `None` on pre-PR-5
    /// entries; `Some(Provenance::User{..})` on signed entries. Kept
    /// alongside `writer_pubkey` / `signature` (not in place of) until a
    /// follow-up PR removes them after full wire-through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

/// Returns the current time in nanoseconds since the Unix epoch.
fn now_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_nanos() as u64
}

/// Generates a deterministic molecule UUID from schema name and field name.
/// Uses SHA-256 to produce a stable, collision-resistant identifier.
pub fn deterministic_molecule_uuid(schema_name: &str, field_name: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}", schema_name, field_name).as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Records a same-key conflict detected during molecule merge.
#[derive(Debug, Clone)]
pub struct MergeConflict {
    pub key: String,
    pub winner_atom: String,
    pub loser_atom: String,
    pub winner_written_at: u64,
    pub loser_written_at: u64,
}

/// Write-time metadata stored per-key on the molecule.
/// Survives atom deduplication because it lives on the key-to-atom
/// association, not on the content-addressed atom itself.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq, utoipa::ToSchema)]
pub struct KeyMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
mod atom_entry_provenance_tests {
    //! molecule-provenance-dag PR 5 — additive `AtomEntry.provenance`.

    use super::*;

    /// Pre-PR-5 on-disk shape — no `provenance` field. Deserializes to
    /// `None` and re-serializes byte-for-byte identical.
    const GOLDEN_PRE_PR5_ATOM_ENTRY_JSON: &str = r#"{"atom_uuid":"atom-1","written_at":42,"writer_pubkey":"","signature":"","signature_version":0}"#;

    #[test]
    fn pre_pr5_atom_entry_json_round_trips_unchanged() {
        let parsed: AtomEntry = serde_json::from_str(GOLDEN_PRE_PR5_ATOM_ENTRY_JSON)
            .expect("deserialize pre-PR-5 atom entry");
        assert!(parsed.provenance.is_none());
        let reserialized = serde_json::to_string(&parsed).expect("serialize");
        assert_eq!(reserialized, GOLDEN_PRE_PR5_ATOM_ENTRY_JSON);
    }

    #[test]
    fn atom_entry_round_trips_with_provenance_user() {
        let entry = AtomEntry {
            atom_uuid: "atom-1".to_string(),
            written_at: 7,
            writer_pubkey: "pk".to_string(),
            signature: "sig".to_string(),
            signature_version: 1,
            provenance: Some(Provenance::user("pk".to_string(), "sig".to_string())),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(json.contains(r#""provenance":{"kind":"user""#));
        let back: AtomEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, entry);
    }

    /// Signed range insert must populate `Provenance::User` with the same
    /// pubkey and signature that land in the legacy fields. The
    /// consistency property — `writer_pubkey == Provenance::User.pubkey`
    /// when both are populated — is what the drift debug-assert on
    /// Molecule protects; entries don't have a helper of their own yet,
    /// but the invariant is the same and is checked here.
    #[test]
    fn molecule_range_signed_entry_provenance_matches_legacy_fields() {
        let kp = crate::security::Ed25519KeyPair::generate().unwrap();
        let mut mol = crate::atom::MoleculeRange::new("s", "f");
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
        let entry = mol.get_atom_entry("k1").expect("entry present");
        match entry
            .provenance
            .as_ref()
            .expect("signed entry has provenance")
        {
            Provenance::User {
                pubkey,
                signature,
                signature_version,
            } => {
                assert_eq!(pubkey, &entry.writer_pubkey);
                assert_eq!(signature, &entry.signature);
                assert_eq!(*signature_version, 1);
            }
            _ => panic!("expected User variant"),
        }
    }

    #[test]
    fn molecule_hash_signed_entry_provenance_matches_legacy_fields() {
        let kp = crate::security::Ed25519KeyPair::generate().unwrap();
        let mut mol = crate::atom::MoleculeHash::new("s", "f");
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
        let entry = mol.get_atom_entry("k1").expect("entry present");
        match entry
            .provenance
            .as_ref()
            .expect("signed entry has provenance")
        {
            Provenance::User {
                pubkey, signature, ..
            } => {
                assert_eq!(pubkey, &entry.writer_pubkey);
                assert_eq!(signature, &entry.signature);
            }
            _ => panic!("expected User variant"),
        }
    }

    #[test]
    fn molecule_hash_range_signed_entry_provenance_matches_legacy_fields() {
        let kp = crate::security::Ed25519KeyPair::generate().unwrap();
        let mut mol = crate::atom::MoleculeHashRange::new("s", "f");
        mol.set_atom_uuid_from_values(
            "h1".to_string(),
            "r1".to_string(),
            "atom-1".to_string(),
            &kp,
        );
        let entry = mol.get_atom_entry("h1", "r1").expect("entry present");
        match entry
            .provenance
            .as_ref()
            .expect("signed entry has provenance")
        {
            Provenance::User {
                pubkey, signature, ..
            } => {
                assert_eq!(pubkey, &entry.writer_pubkey);
                assert_eq!(signature, &entry.signature);
            }
            _ => panic!("expected User variant"),
        }
    }
}
