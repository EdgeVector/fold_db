mod atom_def;
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct KeyMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}
