use super::core_refactored::DbOperationsV2;
use crate::atom::Atom;
use crate::schema::types::field::FieldVariant;
use crate::schema::SchemaError;
use crate::storage::traits::TypedStore;
use serde_json::Value;

impl DbOperationsV2 {
    /// Creates and stores an atom for a mutation field with deferred flush.
    /// If an atom with the same content already exists (content-based deduplication),
    /// returns the existing atom instead of creating a duplicate.
    /// 
    /// This is the async V2 version for use with DbOperationsV2.
    pub async fn create_and_store_atom_for_mutation_deferred(
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
        
        if let Some(existing_atom) = self.atoms_store().get_item::<Atom>(&atom_key).await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to check existing atom: {}", e)))? {
            return Ok(existing_atom);
        }

        // Store the new atom (deferred - no immediate flush)
        self.atoms_store().put_item(&atom_key, &new_atom).await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to store atom: {}", e)))?;

        Ok(new_atom)
    }

    /// Persists a field's molecule to storage with deferred flush.
    /// This is the async V2 version for use with DbOperationsV2.
    pub async fn persist_field_molecule_deferred(
        &self,
        field: &FieldVariant,
        molecule_uuid: &str,
    ) -> Result<(), SchemaError> {
        match field {
            FieldVariant::Single(f) => {
                if let Some(mol) = &f.molecule {
                    self.molecules_store().put_item(&format!("ref:{}", molecule_uuid), mol).await
                        .map_err(|e| SchemaError::InvalidData(format!("Failed to store molecule: {}", e)))?;
                }
            }
            FieldVariant::Range(f) => {
                if let Some(mol) = &f.molecule {
                    self.molecules_store().put_item(&format!("ref:{}", molecule_uuid), mol).await
                        .map_err(|e| SchemaError::InvalidData(format!("Failed to store molecule: {}", e)))?;
                }
            }
            FieldVariant::HashRange(f) => {
                if let Some(mol) = &f.molecule {
                    self.molecules_store().put_item(&format!("ref:{}", molecule_uuid), mol).await
                        .map_err(|e| SchemaError::InvalidData(format!("Failed to store molecule: {}", e)))?;
                }
            }
        }
        Ok(())
    }

    /// Flush atoms store to ensure persistence.
    /// This is a convenience method for explicit flushing when needed.
    pub async fn flush_atoms(&self) -> Result<(), SchemaError> {
        self.atoms_store().inner().flush().await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to flush atoms: {}", e)))
    }

    /// Flush molecules store to ensure persistence.
    /// This is a convenience method for explicit flushing when needed.
    pub async fn flush_molecules(&self) -> Result<(), SchemaError> {
        self.molecules_store().inner().flush().await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to flush molecules: {}", e)))
    }
}

