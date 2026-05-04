//! Hash field type for schema indexing
//!
//! Provides a field type keyed by a single hash key (unordered collection).

use crate::db_operations::DbOperations;
use crate::schema::types::declarative_schemas::FieldMapper;
use crate::schema::types::field::base::refresh_field_from_db;
use crate::schema::types::field::hash_range_filter::{HashRangeFilter, HashRangeFilterResult};
use crate::schema::types::field::FieldCommon;
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::WriteContext;
use crate::schema::types::field::{apply_hash_filter, FilterApplicator};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::SchemaError;
use serde::{Deserialize, Serialize};

use crate::atom::MoleculeHash;
use std::collections::HashMap;
use std::sync::Arc;

/// Field keyed by a single hash key (unordered collection).
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HashField {
    #[serde(flatten)]
    pub inner: FieldCommon,
    pub molecule: Option<MoleculeHash>,
}

impl HashField {
    /// Creates a new Hash field
    #[must_use]
    pub fn new(
        field_mappers: HashMap<String, FieldMapper>,
        molecule: Option<MoleculeHash>,
    ) -> Self {
        Self {
            inner: FieldCommon::new(field_mappers),
            molecule,
        }
    }
}

#[async_trait::async_trait]
impl crate::schema::types::field::Field for HashField {
    fn common(&self) -> &crate::schema::types::field::FieldCommon {
        &self.inner
    }

    fn common_mut(&mut self) -> &mut crate::schema::types::field::FieldCommon {
        &mut self.inner
    }

    async fn refresh_from_db(&mut self, db_ops: &crate::db_operations::DbOperations) {
        refresh_field_from_db(&mut self.inner, &mut self.molecule, db_ops).await;
    }

    fn write_mutation(
        &mut self,
        key_value: &crate::schema::types::key_value::KeyValue,
        ctx: WriteContext,
    ) {
        // Initialize molecule if needed and set molecule_uuid in FieldCommon
        if self.molecule.is_none() {
            let new_molecule = crate::atom::MoleculeHash::new(&ctx.schema_name, &ctx.field_name);
            self.inner
                .set_molecule_uuid(new_molecule.uuid().to_string());
            self.molecule = Some(new_molecule);
        }

        // For HashField, we use the hash key to store the atom. If the
        // caller supplied a `writer_override` (replay/import path), stamp
        // the supplied identity onto the AtomEntry directly instead of
        // signing with the local keypair.
        if let Some(hash_key) = &key_value.hash {
            if let Some(molecule) = &mut self.molecule {
                match ctx.writer_override {
                    Some(crate::atom::provenance::Provenance::User {
                        pubkey,
                        signature,
                        signature_version,
                    }) => {
                        molecule.set_atom_uuid_imported(
                            hash_key.clone(),
                            ctx.atom.uuid().to_string(),
                            pubkey,
                            signature,
                            signature_version,
                        );
                    }
                    _ => {
                        molecule.set_atom_uuid(
                            hash_key.clone(),
                            ctx.atom.uuid().to_string(),
                            &ctx.signer,
                        );
                    }
                }
                // Store per-key metadata on the molecule
                molecule.set_key_metadata(
                    hash_key.clone(),
                    crate::atom::KeyMetadata {
                        source_file_name: ctx.source_file_name,
                        metadata: ctx.metadata,
                    },
                );
            }
        }
    }

    async fn resolve_value(
        &mut self,
        db_ops: &Arc<DbOperations>,
        filter: Option<HashRangeFilter>,
        _as_of: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
        self.refresh_from_db(db_ops).await;
        let result = self.apply_filter(filter);
        // Attach per-key metadata + per-AtomEntry writer_pubkey from the
        // molecule to each match. The writer_pubkey is the only way Hash
        // variants surface authorship into the query response — there is
        // no molecule-level signing key for Hash molecules.
        let matches_with_meta: Vec<(
            KeyValue,
            String,
            Option<crate::atom::KeyMetadata>,
            Option<String>,
        )> = result
            .matches
            .into_iter()
            .map(|(kv, atom_uuid)| {
                let (key_meta, writer_pubkey) = match kv.hash.as_ref() {
                    Some(h) => match self.molecule.as_ref() {
                        Some(m) => (
                            m.get_key_metadata(h).cloned(),
                            m.get_atom_entry(h).map(|e| e.writer_pubkey.clone()),
                        ),
                        None => (None, None),
                    },
                    None => (None, None),
                };
                (kv, atom_uuid, key_meta, writer_pubkey)
            })
            .collect();
        super::fetch_atoms_with_key_metadata_async_with_org(
            db_ops,
            matches_with_meta,
            self.inner.org_hash(),
        )
        .await
    }
}

impl HashField {
    /// Gets all keys in the hash (useful for pagination or listing)
    pub fn get_all_keys(&self) -> Vec<String> {
        self.molecule
            .as_ref()
            .map(|molecule| molecule.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl FilterApplicator for HashField {
    fn apply_filter(&self, filter: Option<HashRangeFilter>) -> HashRangeFilterResult {
        let Some(molecule) = &self.molecule else {
            return HashRangeFilterResult::empty();
        };

        apply_hash_filter(molecule, filter)
    }
}
