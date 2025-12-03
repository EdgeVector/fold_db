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
        
        log::debug!("🔍 Checking for existing atom: {}", atom_key);
        if let Some(existing_atom) = self.atoms_store().get_item::<Atom>(&atom_key).await
            .map_err(|e| {
                log::error!("❌ Failed to check existing atom '{}': {}", atom_key, e);
                SchemaError::InvalidData(format!("Failed to check existing atom: {}", e))
            })? {
            log::debug!("✅ Atom already exists, returning existing: {}", atom_key);
            return Ok(existing_atom);
        }

        // Store the new atom (deferred - no immediate flush)
        log::info!("💾 Writing atom to DynamoDB: key={}, uuid={}", atom_key, new_atom.uuid());
        self.atoms_store().put_item(&atom_key, &new_atom).await
            .map_err(|e| {
                log::error!("❌ Failed to store atom '{}': {}", atom_key, e);
                SchemaError::InvalidData(format!("Failed to store atom: {}", e))
            })?;
        log::info!("✅ Atom written to DynamoDB: {}", atom_key);

        Ok(new_atom)
    }

    /// Persists a field's molecule to storage with deferred flush.
    /// This is the async V2 version for use with DbOperationsV2.
    pub async fn persist_field_molecule_deferred(
        &self,
        field: &FieldVariant,
        molecule_uuid: &str,
    ) -> Result<(), SchemaError> {
        let ref_key = format!("ref:{}", molecule_uuid);
        log::info!("🔗 persist_field_molecule_deferred: molecule_uuid={}, ref_key={}", molecule_uuid, ref_key);
        
        match field {
            FieldVariant::Single(f) => {
                if let Some(mol) = &f.molecule {
                    log::info!("💾 Writing Single molecule to DynamoDB: ref_key={}, has_atom={}", 
                        ref_key, !mol.get_atom_uuid().is_empty());
                    self.molecules_store().put_item(&ref_key, mol).await
                        .map_err(|e| {
                            log::error!("❌ Failed to store molecule '{}': {}", ref_key, e);
                            SchemaError::InvalidData(format!("Failed to store molecule: {}", e))
                        })?;
                    log::info!("✅ Single molecule written to DynamoDB: {}", ref_key);
                } else {
                    log::warn!("⚠️ No molecule to persist for Single field (molecule is None)");
                }
            }
            FieldVariant::Range(f) => {
                if let Some(mol) = &f.molecule {
                    log::info!("💾 Writing Range molecule to DynamoDB: ref_key={}, atom_count={}", 
                        ref_key, mol.atom_uuids.len());
                    self.molecules_store().put_item(&ref_key, mol).await
                        .map_err(|e| {
                            log::error!("❌ Failed to store molecule '{}': {}", ref_key, e);
                            SchemaError::InvalidData(format!("Failed to store molecule: {}", e))
                        })?;
                    log::info!("✅ Range molecule written to DynamoDB: {}", ref_key);
                } else {
                    log::warn!("⚠️ No molecule to persist for Range field (molecule is None)");
                }
            }
            FieldVariant::HashRange(f) => {
                if let Some(mol) = &f.molecule {
                    log::info!("💾 Writing HashRange molecule to DynamoDB: ref_key={}, hash_count={}, total_atoms={}", 
                        ref_key, mol.hash_values().count(), mol.atom_count());
                    self.molecules_store().put_item(&ref_key, mol).await
                        .map_err(|e| {
                            log::error!("❌ Failed to store molecule '{}': {}", ref_key, e);
                            SchemaError::InvalidData(format!("Failed to store molecule: {}", e))
                        })?;
                    log::info!("✅ HashRange molecule written to DynamoDB: {}", ref_key);
                } else {
                    log::warn!("⚠️ No molecule to persist for HashRange field (molecule is None)");
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

