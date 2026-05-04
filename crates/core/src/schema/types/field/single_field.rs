use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::atom::Molecule;
use crate::db_operations::DbOperations;
use crate::schema::types::declarative_schemas::FieldMapper;
use crate::schema::types::field::base::refresh_field_from_db;
use crate::schema::types::field::FieldCommon;
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::WriteContext;
use crate::schema::types::field::{FilterApplicator, HashRangeFilter, HashRangeFilterResult};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::SchemaError;

/// Field storing a single value.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SingleField {
    #[serde(flatten)]
    pub inner: FieldCommon,
    pub molecule: Option<Molecule>,
}

impl SingleField {
    #[must_use]
    pub fn new(field_mappers: HashMap<String, FieldMapper>, molecule: Option<Molecule>) -> Self {
        Self {
            inner: FieldCommon::new(field_mappers),
            molecule,
        }
    }
}

#[async_trait::async_trait]
impl crate::schema::types::field::Field for SingleField {
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
        _key_value: &crate::schema::types::key_value::KeyValue,
        ctx: WriteContext,
    ) {
        // Initialize molecule if needed and set molecule_uuid in FieldCommon
        if self.molecule.is_none() {
            let new_molecule = crate::atom::Molecule::new(
                ctx.atom.uuid().to_string(),
                &ctx.schema_name,
                &ctx.field_name,
            );
            // Get the molecule's UUID and set it in FieldCommon for persistence lookup
            self.inner
                .set_molecule_uuid(new_molecule.uuid().to_string());
            self.molecule = Some(new_molecule);
        }

        // For SingleField, we store the atom and sign — unless the caller
        // provided a `writer_override` (replay/import path), in which case
        // we stamp the supplied identity onto the molecule directly.
        if let Some(molecule) = &mut self.molecule {
            match ctx.writer_override {
                Some(crate::atom::provenance::Provenance::User {
                    pubkey,
                    signature,
                    signature_version,
                }) => {
                    molecule.set_atom_uuid_imported(
                        ctx.atom.uuid().to_string(),
                        pubkey,
                        signature,
                        signature_version,
                    );
                }
                _ => {
                    molecule.set_atom_uuid(ctx.atom.uuid().to_string(), &ctx.signer);
                }
            }
            // Store per-key metadata on the molecule
            molecule.set_key_metadata(crate::atom::KeyMetadata {
                source_file_name: ctx.source_file_name,
                metadata: ctx.metadata,
            });
        }
    }

    async fn resolve_value(
        &mut self,
        db_ops: &Arc<DbOperations>,
        _filter: Option<HashRangeFilter>,
        _as_of: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
        self.refresh_from_db(db_ops).await;
        if let Some(molecule) = &self.molecule {
            let uuid = molecule.get_atom_uuid().clone();
            let key_meta = molecule.get_key_metadata().cloned();
            let writer_pubkey = {
                let pk = molecule.writer_pubkey();
                if pk.is_empty() {
                    None
                } else {
                    Some(pk.to_string())
                }
            };
            let result = super::fetch_atoms_with_key_metadata_async_with_org(
                db_ops,
                vec![(KeyValue::new(None, None), uuid, key_meta, writer_pubkey)],
                self.inner.org_hash(),
            )
            .await?;
            Ok(result)
        } else {
            Ok(HashMap::new())
        }
    }
}

impl FilterApplicator for SingleField {
    fn apply_filter(&self, filter: Option<HashRangeFilter>) -> HashRangeFilterResult {
        let Some(molecule) = &self.molecule else {
            return HashRangeFilterResult::empty();
        };

        let uuid = molecule.get_atom_uuid().clone();
        let mut matches = std::collections::HashMap::new();
        if let HashRangeFilter::SampleN(n) = filter.unwrap_or(HashRangeFilter::SampleN(1)) {
            if n > 0 {
                matches.insert(
                    crate::schema::types::key_value::KeyValue::new(None, None),
                    uuid,
                );
            }
        }

        HashRangeFilterResult::new(matches)
    }
}
