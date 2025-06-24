use super::core::DbOperations;
use crate::schema::core::SchemaState;
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
    /// IMPORTANT: SCHEMAS ARE IMMUTABLE
    /// - Once a schema is stored, it CANNOT be modified or updated
    /// - Attempting to store a schema with the same name will be rejected
    /// - This enforces data consistency and prevents breaking existing Molecule/Molecule chains
    /// - All fields in a schema must be defined at creation time
    ///
    /// Automatically creates placeholder Molecules/Molecules for fields that don't have them.
    /// This ensures all fields are immediately queryable after schema creation.
    pub fn store_schema(&self, schema_name: &str, schema: &Schema) -> Result<(), SchemaError> {
        // IMMUTABILITY CHECK: Reject if schema already exists
        if self.schema_exists(schema_name)? {
            return Err(SchemaError::InvalidData(format!(
                "Schema '{}' already exists. Schemas are immutable and cannot be updated. \
                Create a new schema with a different name instead.",
                schema_name
            )));
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
                        let ref_uuid = molecule.uuid().to_string();
                        
                        // Store the molecule
                        self.store_item(&format!("ref:{}", ref_uuid), &molecule)
                            .map_err(|e| SchemaError::InvalidData(format!("Failed to store molecule: {}", e)))?;

                        // Link the field to the molecule
                        field.set_molecule_uuid(ref_uuid);
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
                        let ref_uuid = molecule.uuid().to_string();
                        
                        // Store the molecule
                        self.store_item(&format!("ref:{}", ref_uuid), &molecule)
                            .map_err(|e| SchemaError::InvalidData(format!("Failed to store molecule: {}", e)))?;

                        // Link the field to the molecule
                        field.set_molecule_uuid(ref_uuid);
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
