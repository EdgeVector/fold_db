//! Thin delegators forwarding atom/molecule methods on `DbOperations`
//! to the underlying `AtomStore`. New code should prefer
//! `db_ops.atoms()` directly.
//!
//! Re-exports `MoleculeData` from `atom_store` for backward compatibility.

use super::atom_store::AtomStore;
use super::core::DbOperations;
use crate::atom::{Atom, MutationEvent};
use crate::schema::SchemaError;
use serde_json::Value;
use std::collections::HashMap;

pub use super::atom_store::MoleculeData;

impl DbOperations {
    pub async fn get_atom_by_uuid(
        &self,
        atom_uuid: &str,
        org_hash: Option<&str>,
    ) -> Result<Option<Atom>, SchemaError> {
        self.atoms().get_atom_by_uuid(atom_uuid, org_hash).await
    }

    pub fn create_atom(
        schema_name: &str,
        value: Value,
        source_file_name: Option<String>,
        metadata: Option<HashMap<String, String>>,
    ) -> Atom {
        AtomStore::create_atom(schema_name, value, source_file_name, metadata)
    }

    pub async fn batch_store_atoms(
        &self,
        atoms: Vec<Atom>,
        org_hash: Option<&str>,
    ) -> Result<(), SchemaError> {
        self.atoms().batch_store_atoms(atoms, org_hash).await
    }

    pub async fn batch_store_molecules(
        &self,
        molecules: Vec<(String, MoleculeData)>,
        org_hash: Option<&str>,
    ) -> Result<(), SchemaError> {
        self.atoms()
            .batch_store_molecules(molecules, org_hash)
            .await
    }

    pub async fn create_and_store_atom_for_mutation_deferred(
        &self,
        schema_name: &str,
        value: Value,
        source_file_name: Option<String>,
        metadata: Option<HashMap<String, String>>,
        org_hash: Option<&str>,
    ) -> Result<Atom, SchemaError> {
        self.atoms()
            .create_and_store_atom_for_mutation_deferred(
                schema_name,
                value,
                source_file_name,
                metadata,
                org_hash,
            )
            .await
    }

    pub async fn batch_store_mutation_events(
        &self,
        events: Vec<MutationEvent>,
        org_hash: Option<&str>,
    ) -> Result<(), SchemaError> {
        self.atoms()
            .batch_store_mutation_events(events, org_hash)
            .await
    }

    pub async fn get_mutation_events(
        &self,
        molecule_uuid: &str,
        org_hash: Option<&str>,
    ) -> Result<Vec<MutationEvent>, SchemaError> {
        self.atoms()
            .get_mutation_events(molecule_uuid, org_hash)
            .await
    }
}
