use crate::atom::{Molecule, MoleculeHashRange, MoleculeRange};
use serde::{Deserialize, Serialize};

/// Enum to hold different types of molecules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MoleculeVariant {
    Single(Molecule),
    Range(MoleculeRange),
    HashRange(MoleculeHashRange),
}

impl MoleculeVariant {
    pub fn uuid(&self) -> &str {
        match self {
            MoleculeVariant::Single(m) => m.uuid(),
            MoleculeVariant::Range(m) => m.uuid(),
            MoleculeVariant::HashRange(m) => m.uuid(),
        }
    }

    pub fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            MoleculeVariant::Single(m) => m.updated_at(),
            MoleculeVariant::Range(m) => m.updated_at(),
            MoleculeVariant::HashRange(m) => m.updated_at(),
        }
    }

    pub fn version(&self) -> u64 {
        match self {
            MoleculeVariant::Single(m) => m.version(),
            MoleculeVariant::Range(m) => m.version(),
            MoleculeVariant::HashRange(m) => m.version(),
        }
    }

    /// Returns the atom UUID for this molecule variant
    /// Note: For Range and HashRange, this returns the molecule's own UUID, not a contained atom UUID
    pub fn get_atom_uuid(&self) -> &String {
        match self {
            MoleculeVariant::Single(m) => m.get_atom_uuid(),
            MoleculeVariant::Range(m) => {
                static RANGE_UUID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                RANGE_UUID.get_or_init(|| m.uuid().to_string())
            }
            MoleculeVariant::HashRange(m) => {
                static HASH_RANGE_UUID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                HASH_RANGE_UUID.get_or_init(|| m.uuid().to_string())
            }
        }
    }
}
