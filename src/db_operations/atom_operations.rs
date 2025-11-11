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

/// Type alias for mutation field tuple: (schema_name, field_name, pub_key, value, key_value, schema_field, field_classifications)
type MutationFieldBatch<'a> = Vec<(String, String, String, Value, KeyValue, &'a mut FieldVariant, Option<Vec<String>>)>;

/// Type alias for index operation tuple: (schema_name, field_name, key_value, value, field_classifications)
type IndexOperationBatch = Vec<(String, String, KeyValue, Value, Option<Vec<String>>)>;

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
        source_file_name: Option<String>,
    ) -> Result<Atom, SchemaError> {
        let mut new_atom = Atom::new(schema_name.to_string(), pub_key.to_string(), value);

        // Set source filename if provided
        if let Some(filename) = source_file_name {
            new_atom = new_atom.with_source_file_name(filename);
        }

        // Check if atom with this content-based UUID already exists
        let atom_key = format!("atom:{}", new_atom.uuid());
        if let Some(existing_atom) = self.get_item::<Atom>(&atom_key)? {
            return Ok(existing_atom);
        }

        // Persist the atom to the database
        self.store_item(&atom_key, &new_atom)?;

        Ok(new_atom)
    }

    /// Creates and stores an atom for a mutation field using deferred storage.
    /// If an atom with the same content already exists (content-based deduplication),
    /// returns the existing atom instead of creating a duplicate.
    pub fn create_and_store_atom_for_mutation_deferred(
        &self,
        schema_name: &str,
        pub_key: &str,
        value: Value,
        source_file_name: Option<String>,
    ) -> Result<Atom, SchemaError> {
        let mut new_atom = Atom::new(schema_name.to_string(), pub_key.to_string(), value);

        // Set source filename if provided
        if let Some(filename) = source_file_name {
            new_atom = new_atom.with_source_file_name(filename);
        }

        // Check if atom with this content-based UUID already exists
        let atom_key = format!("atom:{}", new_atom.uuid());
        if let Some(existing_atom) = self.get_item::<Atom>(&atom_key)? {
            return Ok(existing_atom);
        }

        // Persist the atom to the database using deferred storage
        self.store_item_deferred(&atom_key, &new_atom)?;

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

    /// Persists a molecule from a field to the database using deferred storage
    pub fn persist_field_molecule_deferred(
        &self,
        field: &FieldVariant,
        molecule_uuid: &str,
    ) -> Result<(), SchemaError> {
        use crate::schema::types::field::FieldVariant;
        match field {
            FieldVariant::Single(f) => {
                if let Some(mol) = &f.molecule {
                    self.store_item_deferred(&format!("ref:{}", molecule_uuid), mol)?;
                }
            }
            FieldVariant::Range(f) => {
                if let Some(mol) = &f.molecule {
                    self.store_item_deferred(&format!("ref:{}", molecule_uuid), mol)?;
                }
            }
            FieldVariant::HashRange(f) => {
                if let Some(mol) = &f.molecule {
                    self.store_item_deferred(&format!("ref:{}", molecule_uuid), mol)?;
                }
            }
        }
        Ok(())
    }

    /// Processes a mutation field - creates atom, applies mutation, and persists molecule
    /// Now accepts optional field classifications from the schema topology
    /// 
    /// # Deprecated
    /// Use `process_mutation_fields_batch()` instead for better performance.
    /// Single field processing causes flush-per-field, while batching allows a single flush.
    #[deprecated(since = "0.1.0", note = "Use process_mutation_fields_batch() instead for better performance")]
    #[allow(deprecated)]
    #[allow(clippy::too_many_arguments)]
    pub fn process_mutation_field_with_schema(
        &self,
        schema_name: &str,
        field_name: &str,
        pub_key: &str,
        value: Value,
        key_value: &KeyValue,
        schema_field: &mut FieldVariant,
        field_classifications: Option<Vec<String>>,
    ) -> Result<(), SchemaError> {
        // Refresh field from database
        schema_field.refresh_from_db(self);

        let index_value = value.clone();
        // Create and store the atom (no source_file_name in deprecated single-field path)
        let new_atom = self.create_and_store_atom_for_mutation(schema_name, pub_key, value, None)?;

        // Write mutation to field (updates in-memory molecule)
        schema_field.write_mutation(key_value, new_atom, pub_key.to_string());

        // Persist the updated molecule to the database
        if let Some(molecule_uuid) = schema_field.common().molecule_uuid() {
            self.persist_field_molecule(schema_field, molecule_uuid)?;
        }

        // Use batch indexing (with single element) for consistency
        let index_operations = vec![(
            schema_name.to_string(),
            field_name.to_string(),
            key_value.clone(),
            index_value,
            field_classifications,
        )];
        self.native_index_manager().batch_index_field_values_with_classifications(&index_operations)?;

        Ok(())
    }

    /// Legacy method for backward compatibility
    /// 
    /// # Deprecated
    /// Use `process_mutation_fields_batch()` instead for better performance.
    #[deprecated(since = "0.1.0", note = "Use process_mutation_fields_batch() instead for better performance")]
    #[allow(deprecated)]
    pub fn process_mutation_field(
        &self,
        schema_name: &str,
        field_name: &str,
        pub_key: &str,
        value: Value,
        key_value: &KeyValue,
        schema_field: &mut FieldVariant,
    ) -> Result<(), SchemaError> {
        self.process_mutation_field_with_schema(
            schema_name,
            field_name,
            pub_key,
            value,
            key_value,
            schema_field,
            None, // No classifications = word-only
        )
    }


    // ========== BATCH OPERATIONS ==========

    /// Batch refresh multiple fields from database
    pub fn batch_refresh_fields_from_db(
        &self,
        fields: &mut Vec<(&mut FieldVariant, String)>, // (field, molecule_uuid)
    ) -> Result<(), SchemaError> {
        // Collect all molecule UUIDs
        let molecule_uuids: Vec<String> = fields.iter()
            .map(|(_, uuid)| uuid.clone())
            .collect();

        // Batch fetch all molecules
        let molecule_keys: Vec<String> = molecule_uuids.iter()
            .map(|uuid| format!("ref:{}", uuid))
            .collect();

        let molecules = self.batch_get_items::<serde_json::Value>(&molecule_keys)?;

        // Update fields with fetched molecules
        for (i, (field, _molecule_uuid)) in fields.iter_mut().enumerate() {
            if let Some(molecule_data) = &molecules[i] {
                // Deserialize based on field type
                match field {
                    FieldVariant::Single(f) => {
                        if let Ok(molecule) = serde_json::from_value::<crate::atom::Molecule>(molecule_data.clone()) {
                            f.molecule = Some(molecule);
                        }
                    }
                    FieldVariant::Range(f) => {
                        if let Ok(molecule) = serde_json::from_value::<crate::atom::MoleculeRange>(molecule_data.clone()) {
                            f.molecule = Some(molecule);
                        }
                    }
                    FieldVariant::HashRange(f) => {
                        if let Ok(molecule) = serde_json::from_value::<crate::atom::MoleculeHashRange>(molecule_data.clone()) {
                            f.molecule = Some(molecule);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Batch create and store atoms for mutations
    pub fn batch_create_and_store_atoms(
        &self,
        atoms_to_create: &[(String, String, Value)], // (schema_name, pub_key, value)
    ) -> Result<Vec<Atom>, SchemaError> {
        let mut atoms = Vec::new();
        let mut atoms_to_store = Vec::new();

        for (schema_name, pub_key, value) in atoms_to_create {
            let new_atom = Atom::new(schema_name.clone(), pub_key.clone(), value.clone());
            let atom_key = format!("atom:{}", new_atom.uuid());

            // Check if atom already exists
            if let Some(existing_atom) = self.get_item::<Atom>(&atom_key)? {
                atoms.push(existing_atom);
            } else {
                atoms_to_store.push((atom_key, new_atom.clone()));
                atoms.push(new_atom);
            }
        }

        // Batch store all new atoms
        if !atoms_to_store.is_empty() {
            self.batch_store_items(&atoms_to_store)?;
        }

        Ok(atoms)
    }

    /// Batch persist molecules from fields
    pub fn batch_persist_field_molecules(
        &self,
        molecules_to_store: &[(FieldVariant, String)], // (field, molecule_uuid)
    ) -> Result<(), SchemaError> {
        for (field, molecule_uuid) in molecules_to_store {
            let ref_key = format!("ref:{}", molecule_uuid);
            
            match field {
                FieldVariant::Single(f) => {
                    if let Some(mol) = &f.molecule {
                        self.store_item_deferred(&ref_key, mol)?;
                    }
                }
                FieldVariant::Range(f) => {
                    if let Some(mol) = &f.molecule {
                        self.store_item_deferred(&ref_key, mol)?;
                    }
                }
                FieldVariant::HashRange(f) => {
                    if let Some(mol) = &f.molecule {
                        self.store_item_deferred(&ref_key, mol)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Processes multiple mutation fields in a batch for optimal performance
    /// This method collects all operations and executes them in batches
    pub fn process_mutation_fields_batch(
        &self,
        mutation_fields: &mut MutationFieldBatch,
    ) -> Result<(), SchemaError> {
        if mutation_fields.is_empty() {
            return Ok(());
        }

        // Step 1: Batch refresh all fields from database
        // Collect all molecule UUIDs first
        let molecule_uuids: Vec<String> = mutation_fields.iter()
            .filter_map(|(_, _, _, _, _, schema_field, _)| schema_field.common().molecule_uuid().cloned())
            .collect();

        if !molecule_uuids.is_empty() {
            // Batch fetch all molecules
            let molecule_keys: Vec<String> = molecule_uuids.iter()
                .map(|uuid| format!("ref:{}", uuid))
                .collect();

            let molecules = self.batch_get_items::<serde_json::Value>(&molecule_keys)?;

            // Update fields with fetched molecules
            let mut molecule_index = 0;
            for (_, _, _, _, _, schema_field, _) in mutation_fields.iter_mut() {
                if schema_field.common().molecule_uuid().is_some() {
                    if let Some(molecule_data) = &molecules[molecule_index] {
                        // Deserialize based on field type
                        match schema_field {
                            FieldVariant::Single(f) => {
                                if let Ok(molecule) = serde_json::from_value::<crate::atom::Molecule>(molecule_data.clone()) {
                                    f.molecule = Some(molecule);
                                }
                            }
                            FieldVariant::Range(f) => {
                                if let Ok(molecule) = serde_json::from_value::<crate::atom::MoleculeRange>(molecule_data.clone()) {
                                    f.molecule = Some(molecule);
                                }
                            }
                            FieldVariant::HashRange(f) => {
                                if let Ok(molecule) = serde_json::from_value::<crate::atom::MoleculeHashRange>(molecule_data.clone()) {
                                    f.molecule = Some(molecule);
                                }
                            }
                        }
                    }
                    molecule_index += 1;
                }
            }
        }

        // Step 2: Batch create and store atoms
        let atoms_to_create: Vec<(String, String, Value)> = mutation_fields.iter()
            .map(|(schema_name, _, pub_key, value, _, _, _)| (schema_name.clone(), pub_key.clone(), value.clone()))
            .collect();

        let atoms = self.batch_create_and_store_atoms(&atoms_to_create)?;

        // Step 3: Apply mutations to fields (in-memory operations)
        for (i, (_, _, pub_key, _, key_value, schema_field, _)) in mutation_fields.iter_mut().enumerate() {
            let atom = &atoms[i];
            
            // Write mutation to field (updates in-memory molecule)
            schema_field.write_mutation(key_value, atom.clone(), pub_key.clone());

            // Persist molecule using deferred storage
            if let Some(molecule_uuid) = schema_field.common().molecule_uuid() {
                self.persist_field_molecule_deferred(schema_field, molecule_uuid)?;
            }
        }

        // Step 5: Batch index operations
        let index_operations: IndexOperationBatch = mutation_fields.iter()
            .map(|(schema_name, field_name, _, value, key_value, _, classifications)| {
                (schema_name.clone(), field_name.clone(), key_value.clone(), value.clone(), classifications.clone())
            })
            .collect();

        self.native_index_manager().batch_index_field_values_with_classifications(&index_operations)?;

        Ok(())
    }
}
