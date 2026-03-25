mod atom_def;
mod molecule;
mod molecule_hash;
mod molecule_hash_range;
mod molecule_range;
mod molecule_tests;
pub mod mutation_event;

pub use atom_def::Atom;
pub use molecule::Molecule;
pub use molecule_hash::MoleculeHash;
pub use molecule_hash_range::MoleculeHashRange;
pub use molecule_range::MoleculeRange;
pub use mutation_event::{FieldKey, MutationEvent};

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
