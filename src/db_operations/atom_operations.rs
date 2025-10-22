use super::core::DbOperations;
use crate::atom::{Atom, AtomStatus, Molecule, MoleculeRange};
use crate::schema::{
    types::{
        field::{Field, FieldVariant},
        key_value::KeyValue,
    },
    SchemaError,
};
use serde_json::Value;

impl DbOperations {
    /// Creates a new atom and stores it in the database.
    /// If an atom with the same content already exists (content-based deduplication),
    /// returns the existing atom instead of creating a duplicate.
    pub fn create_atom(
        &self,
        schema_name: &str,
        source_pub_key: String,
        prev_atom_uuid: Option<String>,
        content: Value,
        status: Option<AtomStatus>,
    ) -> Result<Atom, SchemaError> {
        let mut atom = Atom::new(schema_name.to_string(), source_pub_key, content);

        // Check if atom with this content-based UUID already exists
        let atom_key = format!("atom:{}", atom.uuid());
        if let Some(existing_atom) = self.get_item::<Atom>(&atom_key)? {
            return Ok(existing_atom);
        }

        // Only set prev_atom_uuid if it's Some
        if let Some(prev_uuid) = prev_atom_uuid {
            if !prev_uuid.is_empty() {
                atom = atom.with_prev_version(prev_uuid);
            }
        }

        atom = atom.with_status(status.unwrap_or(AtomStatus::Active));
        // Persist with an "atom:" prefix so we can easily distinguish entries
        // when reloading from disk
        self.store_item(&atom_key, &atom)?;
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

    /// Creates and stores an atom for a mutation field.
    /// If an atom with the same content already exists (content-based deduplication),
    /// returns the existing atom instead of creating a duplicate.
    pub fn create_and_store_atom_for_mutation(
        &self,
        schema_name: &str,
        pub_key: &str,
        value: Value,
    ) -> Result<Atom, SchemaError> {
        let new_atom = Atom::new(schema_name.to_string(), pub_key.to_string(), value);

        // Check if atom with this content-based UUID already exists
        let atom_key = format!("atom:{}", new_atom.uuid());
        if let Some(existing_atom) = self.get_item::<Atom>(&atom_key)? {
            return Ok(existing_atom);
        }

        // Persist the atom to the database
        self.store_item(&atom_key, &new_atom)?;

        Ok(new_atom)
    }

    /// Persists a molecule from a field to the database
    pub fn persist_field_molecule(
        &self,
        field: &FieldVariant,
        molecule_uuid: &str,
    ) -> Result<(), SchemaError> {
        use crate::schema::types::field::FieldVariant;
        match field {
            FieldVariant::Single(f) => {
                if let Some(mol) = &f.molecule {
                    self.store_item(&format!("ref:{}", molecule_uuid), mol)?;
                }
            }
            FieldVariant::Range(f) => {
                if let Some(mol) = &f.molecule {
                    self.store_item(&format!("ref:{}", molecule_uuid), mol)?;
                }
            }
            FieldVariant::HashRange(f) => {
                if let Some(mol) = &f.molecule {
                    self.store_item(&format!("ref:{}", molecule_uuid), mol)?;
                }
            }
        }
        Ok(())
    }

    /// Processes a mutation field - creates atom, applies mutation, and persists molecule
    pub fn process_mutation_field(
        &self,
        schema_name: &str,
        field_name: &str,
        pub_key: &str,
        value: Value,
        key_value: &KeyValue,
        schema_field: &mut FieldVariant,
    ) -> Result<(), SchemaError> {
        // Refresh field from database
        schema_field.refresh_from_db(self);

        let index_value = value.clone();
        // Create and store the atom
        let new_atom = self.create_and_store_atom_for_mutation(schema_name, pub_key, value)?;

        // Write mutation to field (updates in-memory molecule)
        schema_field.write_mutation(key_value, new_atom, pub_key.to_string());

        // Persist the updated molecule to the database
        if let Some(molecule_uuid) = schema_field.common().molecule_uuid() {
            self.persist_field_molecule(schema_field, molecule_uuid)?;
        }

        self.native_index_manager().index_field_value(
            schema_name,
            field_name,
            key_value,
            &index_value,
        )?;

        Ok(())
    }
}
