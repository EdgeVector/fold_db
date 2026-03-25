use super::core::DbOperations;
use crate::atom::{Atom, Molecule, MoleculeHash, MoleculeHashRange, MoleculeRange, MutationEvent};
use crate::schema::SchemaError;
use crate::storage::traits::TypedStore;
use serde_json::Value;
use std::collections::HashMap;

/// Enum to hold different molecule types for batch storage
#[derive(Clone)]
pub enum MoleculeData {
    Single(Molecule),
    Hash(MoleculeHash),
    Range(MoleculeRange),
    HashRange(MoleculeHashRange),
}

impl DbOperations {
    /// Creates an atom in memory without storing it.
    /// Used for batch operations where atoms are collected first then stored together.
    pub fn create_atom(
        schema_name: &str,
        pub_key: &str,
        value: Value,
        source_file_name: Option<String>,
        metadata: Option<HashMap<String, String>>,
    ) -> Atom {
        let mut atom = Atom::new(schema_name.to_string(), pub_key.to_string(), value);
        if let Some(filename) = source_file_name {
            atom = atom.with_source_file_name(filename);
        }
        if let Some(meta) = metadata {
            atom = atom.with_metadata(meta);
        }
        atom
    }

    /// Batch store multiple atoms efficiently.
    /// Uses DynamoDB BatchWriteItem to store up to 25 atoms per API call.
    /// Deduplicates by key since atoms with identical content have the same UUID.
    pub async fn batch_store_atoms(&self, atoms: Vec<Atom>) -> Result<(), SchemaError> {
        if atoms.is_empty() {
            return Ok(());
        }

        // Deduplicate by key - atoms with same content have same UUID
        // DynamoDB batch_write_item doesn't allow duplicate keys in the same batch
        let mut seen_keys = std::collections::HashSet::new();
        let items: Vec<(String, Atom)> = atoms
            .into_iter()
            .filter_map(|atom| {
                let key = format!("atom:{}", atom.uuid());
                if seen_keys.insert(key.clone()) {
                    Some((key, atom))
                } else {
                    None // Skip duplicate keys
                }
            })
            .collect();

        log::info!(
            "💾 Batch storing {} atoms to DynamoDB (after dedup)",
            items.len()
        );

        self.atoms_store()
            .batch_put_items(items)
            .await
            .map_err(|e| {
                log::error!("❌ Failed to batch store atoms: {}", e);
                SchemaError::InvalidData(format!("Failed to batch store atoms: {}", e))
            })?;

        log::info!("✅ Batch stored atoms successfully");
        Ok(())
    }

    /// Batch store multiple molecules efficiently.
    /// Accepts a vector of (molecule_uuid, molecule_data) tuples.
    /// Deduplicates by key to prevent DynamoDB batch_write_item errors.
    pub async fn batch_store_molecules(
        &self,
        molecules: Vec<(String, MoleculeData)>,
    ) -> Result<(), SchemaError> {
        if molecules.is_empty() {
            return Ok(());
        }

        // Deduplicate by key - DynamoDB batch_write_item doesn't allow duplicate keys
        let mut seen_keys = std::collections::HashSet::new();
        let items: Vec<(String, serde_json::Value)> = molecules
            .into_iter()
            .filter_map(|(uuid, mol_data)| {
                let ref_key = format!("ref:{}", uuid);
                if seen_keys.insert(ref_key.clone()) {
                    let value =
                        match mol_data {
                            MoleculeData::Single(mol) => {
                                serde_json::to_value(mol).expect("Molecule is always serializable")
                            }
                            MoleculeData::Hash(mol) => serde_json::to_value(mol)
                                .expect("MoleculeHash is always serializable"),
                            MoleculeData::Range(mol) => serde_json::to_value(mol)
                                .expect("MoleculeRange is always serializable"),
                            MoleculeData::HashRange(mol) => serde_json::to_value(mol)
                                .expect("MoleculeHashRange is always serializable"),
                        };
                    Some((ref_key, value))
                } else {
                    None // Skip duplicate keys
                }
            })
            .collect();

        log::info!(
            "💾 Batch storing {} molecules to DynamoDB (after dedup)",
            items.len()
        );

        self.molecules_store()
            .batch_put_items(items)
            .await
            .map_err(|e| {
                log::error!("❌ Failed to batch store molecules: {}", e);
                SchemaError::InvalidData(format!("Failed to batch store molecules: {}", e))
            })?;

        log::info!("✅ Batch stored molecules successfully");
        Ok(())
    }

    /// Creates and stores an atom for a mutation field with deferred flush.
    /// If an atom with the same content already exists (content-based deduplication),
    /// returns the existing atom instead of creating a duplicate.
    ///
    /// This is the async V2 version for use with DbOperations.
    pub async fn create_and_store_atom_for_mutation_deferred(
        &self,
        schema_name: &str,
        pub_key: &str,
        value: Value,
        source_file_name: Option<String>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<Atom, SchemaError> {
        let mut new_atom = Atom::new(schema_name.to_string(), pub_key.to_string(), value);

        // Set source filename if provided
        if let Some(filename) = source_file_name {
            new_atom = new_atom.with_source_file_name(filename);
        }

        // Set metadata if provided
        if let Some(meta) = metadata {
            new_atom = new_atom.with_metadata(meta);
        }

        // Check if atom with this content-based UUID already exists
        let atom_key = format!("atom:{}", new_atom.uuid());

        log::debug!("🔍 Checking for existing atom: {}", atom_key);
        if let Some(existing_atom) = self
            .atoms_store()
            .get_item::<Atom>(&atom_key)
            .await
            .map_err(|e| {
                log::error!("❌ Failed to check existing atom '{}': {}", atom_key, e);
                SchemaError::InvalidData(format!("Failed to check existing atom: {}", e))
            })?
        {
            log::debug!("✅ Atom already exists, returning existing: {}", atom_key);
            return Ok(existing_atom);
        }

        // Store the new atom (deferred - no immediate flush)
        log::info!(
            "💾 Writing atom to DynamoDB: key={}, uuid={}",
            atom_key,
            new_atom.uuid()
        );
        self.atoms_store()
            .put_item(&atom_key, &new_atom)
            .await
            .map_err(|e| {
                log::error!("❌ Failed to store atom '{}': {}", atom_key, e);
                SchemaError::InvalidData(format!("Failed to store atom: {}", e))
            })?;
        log::info!("✅ Atom written to DynamoDB: {}", atom_key);

        Ok(new_atom)
    }

    /// Batch store mutation events for point-in-time query support.
    /// Events are stored with zero-padded nanosecond timestamps for lexicographic ordering.
    pub async fn batch_store_mutation_events(
        &self,
        events: Vec<MutationEvent>,
    ) -> Result<(), SchemaError> {
        if events.is_empty() {
            return Ok(());
        }

        let items: Vec<(String, MutationEvent)> = events
            .into_iter()
            .map(|e| {
                let ts = e.timestamp.timestamp_nanos_opt().unwrap_or(0);
                let key = format!("history:{}:{:020}", e.molecule_uuid, ts);
                (key, e)
            })
            .collect();

        self.atoms_store()
            .batch_put_items(items)
            .await
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to store mutation events: {}", e))
            })
    }

    /// Load all mutation events for a molecule, sorted chronologically.
    pub async fn get_mutation_events(
        &self,
        molecule_uuid: &str,
    ) -> Result<Vec<MutationEvent>, SchemaError> {
        let prefix = format!("history:{}:", molecule_uuid);
        let items: Vec<(String, MutationEvent)> = self
            .atoms_store()
            .scan_items_with_prefix(&prefix)
            .await
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to load mutation events: {}", e))
            })?;

        // Items from scan_prefix are already in lexicographic order (= chronological due to zero-padding)
        let events: Vec<MutationEvent> = items.into_iter().map(|(_, e)| e).collect();
        Ok(events)
    }
}
