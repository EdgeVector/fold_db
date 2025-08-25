use crate::atom::{Molecule, MoleculeRange, MoleculeHashRange, MoleculeBehavior};

/// Enum to hold different types of molecules
#[derive(Debug)]
pub enum MoleculeVariant {
    Single(Molecule),
    Range(MoleculeRange),
    HashRange(MoleculeHashRange),
}

impl MoleculeBehavior for MoleculeVariant {
    fn uuid(&self) -> &str {
        match self {
            MoleculeVariant::Single(m) => m.uuid(),
            MoleculeVariant::Range(m) => m.uuid(),
            MoleculeVariant::HashRange(m) => m.uuid(),
        }
    }

    fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            MoleculeVariant::Single(m) => m.updated_at(),
            MoleculeVariant::Range(m) => m.updated_at(),
            MoleculeVariant::HashRange(m) => m.updated_at(),
        }
    }

    fn status(&self) -> &crate::atom::MoleculeStatus {
        match self {
            MoleculeVariant::Single(m) => m.status(),
            MoleculeVariant::Range(m) => m.status(),
            MoleculeVariant::HashRange(m) => m.status(),
        }
    }

    fn set_status(&mut self, status: &crate::atom::MoleculeStatus, source_pub_key: String) {
        match self {
            MoleculeVariant::Single(m) => m.set_status(status, source_pub_key),
            MoleculeVariant::Range(m) => m.set_status(status, source_pub_key),
            MoleculeVariant::HashRange(m) => m.set_status(status, source_pub_key),
        }
    }

    fn update_history(&self) -> &Vec<crate::atom::MoleculeUpdate> {
        match self {
            MoleculeVariant::Single(m) => m.update_history(),
            MoleculeVariant::HashRange(m) => m.update_history(),
            MoleculeVariant::Range(m) => m.update_history(),
        }
    }
}

impl MoleculeVariant {
    /// Returns the atom UUID for this molecule variant
    /// Note: For Range and HashRange, this returns the molecule's own UUID, not a contained atom UUID
    pub fn get_atom_uuid(&self) -> &String {
        match self {
            MoleculeVariant::Single(m) => m.get_atom_uuid(),
            MoleculeVariant::Range(m) => {
                // Convert &str to &String by using the molecule's own UUID
                static RANGE_UUID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                RANGE_UUID.get_or_init(|| m.uuid().to_string())
            }
            MoleculeVariant::HashRange(m) => {
                // Convert &str to &String by using the molecule's own UUID
                static HASH_RANGE_UUID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                HASH_RANGE_UUID.get_or_init(|| m.uuid().to_string())
            }
        }
    }
}
