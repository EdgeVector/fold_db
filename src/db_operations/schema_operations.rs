use super::core::DbOperations;
use crate::schema::SchemaState;
use crate::schema::Schema;
use crate::schema::SchemaError;
use crate::schema::types::field::{FieldVariant, common::Field};
use crate::atom::{Atom, Molecule, MoleculeBehavior};
use serde_json::json;

impl DbOperations {
    /// Stores a schema state using generic tree operations
    pub fn store_schema_state(
        &self,
        schema_name: &str,
        state: SchemaState,
    ) -> Result<(), SchemaError> {
        self.store_in_tree(&self.schema_states_tree, schema_name, &state)
    }

    /// Gets a schema state using generic tree operations
    pub fn get_schema_state(&self, schema_name: &str) -> Result<Option<SchemaState>, SchemaError> {
        self.get_from_tree(&self.schema_states_tree, schema_name)
    }

    /// Lists all schemas with a specific state
    pub fn list_schemas_by_state(
        &self,
        target_state: SchemaState,
    ) -> Result<Vec<String>, SchemaError> {
        let all_states: Vec<(String, SchemaState)> =
            self.list_items_in_tree(&self.schema_states_tree)?;
        Ok(all_states
            .into_iter()
            .filter(|(_, state)| *state == target_state)
            .map(|(name, _)| name)
            .collect())
    }

    /// Stores a schema definition using generic tree operations
    ///
    /// IMPORTANT: SCHEMA STRUCTURE IS IMMUTABLE
    /// - Schema structure (field names, types, transforms) cannot be modified once stored
    /// - Field assignments (molecule_uuid values) CAN be updated as part of normal operations
    /// - This allows field mapping while preventing breaking structural changes
    ///
    /// Automatically creates placeholder Molecules/Molecules for fields that don't have them.
    /// This ensures all fields are immediately queryable after schema creation.
    pub fn store_schema(&self, schema_name: &str, schema: &Schema) -> Result<(), SchemaError> {
        // Check if schema exists for field assignment updates vs new schema creation
        if let Ok(Some(existing_schema)) = self.get_schema(schema_name) {
            // FIELD ASSIGNMENT UPDATE: Allow updates when only molecule_uuid values change
            // This is the common case for approved schemas that need field assignments
            log::info!("🔄 Updating existing schema '{}' (likely field assignments)", schema_name);
            
            // Basic sanity check: same number of fields (prevents major structural changes)
            if existing_schema.fields.len() != schema.fields.len() {
                return Err(SchemaError::InvalidData(format!(
                    "Schema '{}' field count cannot be modified (existing: {}, new: {}). \
                    Only field assignments (molecule_uuid) can be updated.",
                    schema_name, existing_schema.fields.len(), schema.fields.len()
                )));
            }
            
            // Check that field names are the same (prevents structural changes)
            let existing_field_names: std::collections::HashSet<_> = existing_schema.fields.keys().collect();
            let new_field_names: std::collections::HashSet<_> = schema.fields.keys().collect();
            
            if existing_field_names != new_field_names {
                return Err(SchemaError::InvalidData(format!(
                    "Schema '{}' field names cannot be modified. \
                    Only field assignments (molecule_uuid) can be updated.",
                    schema_name
                )));
            }
        } else {
            // This is a new schema - create with placeholder molecules
            log::info!("🆕 Creating new schema '{}' with placeholder molecules", schema_name);
        }
        
        // Clone the schema so we can modify fields to add Molecules/Molecules
        let mut schema_with_refs = schema.clone();
        
        // Process each field to ensure it has an Molecule/Molecule for immediate queryability
        for (field_name, field_variant) in &mut schema_with_refs.fields {
            match field_variant {
                FieldVariant::Single(ref mut field) => {
                    if field.molecule_uuid().is_none() {
                        // Create placeholder atom and molecule for this field
                        let placeholder_content = json!({
                            "field_name": field_name,
                            "schema_name": schema_name,
                            "initialized": false,
                            "value": null
                        });
                        
                        // Create atom with placeholder content
                        let atom = Atom::new(
                            schema_name.to_string(),
                            "system".to_string(),
                            placeholder_content,
                        );
                        let atom_uuid = atom.uuid().to_string();
                        
                        // Store the atom
                        self.store_item(&format!("atom:{}", atom_uuid), &atom)
                            .map_err(|e| SchemaError::InvalidData(format!("Failed to store placeholder atom: {}", e)))?;
                        
                        // Create molecule pointing to the atom
                        let molecule = Molecule::new(atom_uuid, "system".to_string());
                        let molecule_uuid = molecule.uuid().to_string();
                        
                        // Store the molecule
                        self.store_item(&format!("ref:{}", molecule_uuid), &molecule)
                            .map_err(|e| SchemaError::InvalidData(format!("Failed to store molecule: {}", e)))?;

                        // Link the field to the molecule
                        field.set_molecule_uuid(molecule_uuid);
                    }
                }
                FieldVariant::Range(ref mut field) => {
                    if field.molecule_uuid().is_none() {
                        // Create placeholder atom and molecule for range field
                        let placeholder_content = json!({
                            "field_name": field_name,
                            "schema_name": schema_name,
                            "initialized": false,
                            "range_data": []
                        });
                        
                        // Create atom with placeholder content
                        let atom = Atom::new(
                            schema_name.to_string(),
                            "system".to_string(),
                            placeholder_content,
                        );
                        let atom_uuid = atom.uuid().to_string();
                        
                        // Store the atom
                        self.store_item(&format!("atom:{}", atom_uuid), &atom)
                            .map_err(|e| SchemaError::InvalidData(format!("Failed to store placeholder atom: {}", e)))?;
                        
                        // Create molecule pointing to the atom
                        let molecule = Molecule::new(atom_uuid, "system".to_string());
                        let molecule_uuid = molecule.uuid().to_string();
                        
                        // Store the molecule
                        self.store_item(&format!("ref:{}", molecule_uuid), &molecule)
                            .map_err(|e| SchemaError::InvalidData(format!("Failed to store molecule: {}", e)))?;

                        // Link the field to the molecule
                        field.set_molecule_uuid(molecule_uuid);
                    }
                }
                FieldVariant::HashRange(ref mut field) => {
                    if field.molecule_uuid().is_none() {
                        // Create placeholder atom and molecule for hash-range field
                        let placeholder_content = json!({
                            "field_name": field_name,
                            "schema_name": schema_name,
                            "initialized": false,
                            "hash_range_data": {}
                        });
                        
                        // Create atom with placeholder content
                        let atom = Atom::new(
                            schema_name.to_string(),
                            "system".to_string(),
                            placeholder_content,
                        );
                        let atom_uuid = atom.uuid().to_string();
                        
                        // Store the atom
                        self.store_item(&format!("atom:{}", atom_uuid), &atom)
                            .map_err(|e| SchemaError::InvalidData(format!("Failed to store placeholder atom: {}", e)))?;
                        
                        // Create molecule pointing to the atom
                        let molecule = Molecule::new(atom_uuid, "system".to_string());
                        let molecule_uuid = molecule.uuid().to_string();
                        
                        // Store the molecule
                        self.store_item(&format!("ref:{}", molecule_uuid), &molecule)
                            .map_err(|e| SchemaError::InvalidData(format!("Failed to store molecule: {}", e)))?;

                        // Link the field to the molecule
                        field.set_molecule_uuid(molecule_uuid);
                    }
                }
            }
        }
        
        // Store the immutable schema with Molecules/Molecules
        self.store_in_tree(&self.schemas_tree, schema_name, &schema_with_refs)
    }

    /// Gets a schema definition using generic tree operations
    pub fn get_schema(&self, schema_name: &str) -> Result<Option<Schema>, SchemaError> {
        self.get_from_tree(&self.schemas_tree, schema_name)
    }

    /// Lists all stored schemas using generic tree operations
    pub fn list_all_schemas(&self) -> Result<Vec<String>, SchemaError> {
        self.list_keys_in_tree(&self.schemas_tree)
    }

    /// Deletes a schema definition
    pub fn delete_schema(&self, schema_name: &str) -> Result<bool, SchemaError> {
        self.delete_from_tree(&self.schemas_tree, schema_name)
    }

    /// Deletes a schema state
    pub fn delete_schema_state(&self, schema_name: &str) -> Result<bool, SchemaError> {
        self.delete_from_tree(&self.schema_states_tree, schema_name)
    }

    // NOTE: add_schema_to_available_directory has been removed to eliminate duplication.
    // Use SchemaCore::add_schema_to_available_directory instead, which provides:
    // - Comprehensive validation
    // - Hash-based de-duplication
    // - Conflict resolution
    // - Proper integration with the schema system

    /// Checks if a schema exists
    pub fn schema_exists(&self, schema_name: &str) -> Result<bool, SchemaError> {
        self.exists_in_tree(&self.schemas_tree, schema_name)
    }

    /// Checks if a schema state exists
    pub fn schema_state_exists(&self, schema_name: &str) -> Result<bool, SchemaError> {
        self.exists_in_tree(&self.schema_states_tree, schema_name)
    }

    /// Gets all schema states as a HashMap
    pub fn get_all_schema_states(
        &self,
    ) -> Result<std::collections::HashMap<String, SchemaState>, SchemaError> {
        let items: Vec<(String, SchemaState)> =
            self.list_items_in_tree(&self.schema_states_tree)?;
        Ok(items.into_iter().collect())
    }

}
