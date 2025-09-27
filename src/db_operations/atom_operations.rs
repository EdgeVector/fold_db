use super::core::DbOperations;
use crate::atom::{Atom, AtomStatus, Molecule, MoleculeRange};
use crate::schema::SchemaError;
use serde_json::Value;

impl DbOperations {
    /// Creates a new atom and stores it in the database
    pub fn create_atom(
        &self,
        schema_name: &str,
        source_pub_key: String,
        prev_atom_uuid: Option<String>,
        content: Value,
        status: Option<AtomStatus>,
    ) -> Result<Atom, SchemaError> {
        let mut atom = Atom::new(schema_name.to_string(), source_pub_key, content);

        // Only set prev_atom_uuid if it's Some
        if let Some(prev_uuid) = prev_atom_uuid {
            if !prev_uuid.is_empty() {
                atom = atom.with_prev_version(prev_uuid);
            }
        }

        atom = atom.with_status(status.unwrap_or(AtomStatus::Active));
        // Persist with an "atom:" prefix so we can easily distinguish entries
        // when reloading from disk
        self.store_item(&format!("atom:{}", atom.uuid()), &atom)?;
        Ok(atom)
    }

    /// Creates or updates a molecule reference
    pub fn update_molecule(
        &self,
        molecule_uuid: &str,
        atom_uuid: String,
        source_pub_key: String,
    ) -> Result<Molecule, SchemaError> {
        let mut molecule = match self.get_item::<Molecule>(&format!("ref:{}", molecule_uuid))? {
            Some(existing_molecule) => existing_molecule,
            None => Molecule::new(atom_uuid.clone(), source_pub_key),
        };

        molecule.set_atom_uuid(atom_uuid.clone());

        self.store_item(&format!("ref:{}", molecule_uuid), &molecule)?;

        Ok(molecule)
    }

    /// Creates or updates a range of molecule references
    pub fn update_molecule_range(
        &self,
        molecule_uuid: &str,
        atom_uuid: String,
        key: String,
        source_pub_key: String,
    ) -> Result<MoleculeRange, SchemaError> {
        let mut molecule_range =
            match self.get_item::<MoleculeRange>(&format!("ref:{}", molecule_uuid))? {
                Some(existing_range) => existing_range,
                None => MoleculeRange::new(source_pub_key),
            };

        molecule_range.set_atom_uuid(key, atom_uuid);
        self.store_item(&format!("ref:{}", molecule_uuid), &molecule_range)?;
        Ok(molecule_range)
    }
}
