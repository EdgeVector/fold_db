use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::atom::{FieldKey, MutationEvent};
use crate::db_operations::DbOperations;
use crate::schema::types::field::{
    Field, FieldCommon, FilterApplicator, HashField, HashRangeField, HashRangeFilter, RangeField,
    SingleField, WriteContext,
};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::SchemaError;
use serde_json::Value as JsonValue;

/// Enumeration over all field variants.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum FieldVariant {
    /// Single value field
    Single(SingleField),
    /// Hash-keyed collection (unordered)
    Hash(HashField),
    /// Range of values
    Range(RangeField),
    /// Hash-range field for complex indexing
    HashRange(HashRangeField),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValue {
    pub value: JsonValue,
    pub atom_uuid: String,
    pub source_file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub molecule_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub molecule_version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_pub_key: Option<String>,
}

// Macro to reduce boilerplate for Field trait implementation
macro_rules! delegate_field_method {
    ($self:ident, $method:ident) => {
        match $self {
            Self::Single(f) => f.$method(),
            Self::Hash(f) => f.$method(),
            Self::Range(f) => f.$method(),
            Self::HashRange(f) => f.$method(),
        }
    };
    ($self:ident, $method:ident, $($args:expr),+) => {
        match $self {
            Self::Single(f) => f.$method($($args),+),
            Self::Hash(f) => f.$method($($args),+),
            Self::Range(f) => f.$method($($args),+),
            Self::HashRange(f) => f.$method($($args),+),
        }
    };
}

#[async_trait]
impl Field for FieldVariant {
    fn common(&self) -> &FieldCommon {
        delegate_field_method!(self, common)
    }

    fn common_mut(&mut self) -> &mut FieldCommon {
        delegate_field_method!(self, common_mut)
    }

    async fn refresh_from_db(&mut self, db_ops: &DbOperations) {
        match self {
            Self::Single(f) => f.refresh_from_db(db_ops).await,
            Self::Hash(f) => f.refresh_from_db(db_ops).await,
            Self::Range(f) => f.refresh_from_db(db_ops).await,
            Self::HashRange(f) => f.refresh_from_db(db_ops).await,
        }
    }

    fn write_mutation(
        &mut self,
        key_value: &crate::schema::types::key_value::KeyValue,
        ctx: WriteContext,
    ) {
        delegate_field_method!(self, write_mutation, key_value, ctx)
    }

    async fn resolve_value(
        &mut self,
        db_ops: &Arc<DbOperations>,
        filter: Option<HashRangeFilter>,
        as_of: Option<DateTime<Utc>>,
    ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
        // Refresh field data from database first
        self.refresh_from_db(db_ops).await;

        // If as_of is requested, rewind molecule to that point in time
        if let Some(as_of) = as_of {
            self.rewind_to(db_ops, as_of).await?;
        }

        // Apply filter then attach per-key molecule metadata to each match.
        // This is where KeyMetadata (stored on the molecule) gets plumbed into
        // the read path so it takes precedence over atom-level metadata.
        use crate::schema::types::field::fetch_atoms_with_key_metadata_async_with_org;
        let org_hash_owned: Option<String> = self.common().org_hash.clone();
        let results = match self {
            FieldVariant::Single(f) => f.apply_filter(filter),
            FieldVariant::Hash(f) => f.apply_filter(filter),
            FieldVariant::Range(f) => f.apply_filter(filter),
            FieldVariant::HashRange(f) => f.apply_filter(filter),
        };
        let matches_with_meta: Vec<(KeyValue, String, Option<crate::atom::KeyMetadata>)> = results
            .matches
            .into_iter()
            .map(|(kv, atom_uuid)| {
                let key_meta = match self {
                    FieldVariant::Single(f) => f
                        .base
                        .molecule
                        .as_ref()
                        .and_then(|m| m.get_key_metadata().cloned()),
                    FieldVariant::Hash(f) => kv.hash.as_ref().and_then(|h| {
                        f.base
                            .molecule
                            .as_ref()
                            .and_then(|m| m.get_key_metadata(h).cloned())
                    }),
                    FieldVariant::Range(f) => kv.range.as_ref().and_then(|r| {
                        f.base
                            .molecule
                            .as_ref()
                            .and_then(|m| m.get_key_metadata(r).cloned())
                    }),
                    FieldVariant::HashRange(f) => {
                        kv.hash.as_ref().zip(kv.range.as_ref()).and_then(|(h, r)| {
                            f.base
                                .molecule
                                .as_ref()
                                .and_then(|m| m.get_key_metadata(h, r).cloned())
                        })
                    }
                };
                (kv, atom_uuid, key_meta)
            })
            .collect();
        let mut resolved = fetch_atoms_with_key_metadata_async_with_org(
            db_ops,
            matches_with_meta,
            org_hash_owned.as_deref(),
        )
        .await?;

        // Stamp molecule info on each resolved FieldValue
        let mol_uuid = self.common().molecule_uuid().cloned();
        let mol_version = self.molecule_version();
        for fv in resolved.values_mut() {
            fv.molecule_uuid = mol_uuid.clone();
            fv.molecule_version = mol_version;
        }

        Ok(resolved)
    }
}

impl FieldVariant {
    /// Returns whether a molecule is present in this field.
    #[must_use]
    pub fn has_molecule(&self) -> bool {
        match self {
            Self::Single(f) => f.base.molecule.is_some(),
            Self::Hash(f) => f.base.molecule.is_some(),
            Self::Range(f) => f.base.molecule.is_some(),
            Self::HashRange(f) => f.base.molecule.is_some(),
        }
    }

    /// Clone the molecule data for persistence, if present.
    #[must_use]
    pub fn clone_molecule_data(&self) -> Option<crate::db_operations::MoleculeData> {
        use crate::db_operations::MoleculeData;
        match self {
            Self::Single(f) => f.base.molecule.clone().map(MoleculeData::Single),
            Self::Hash(f) => f.base.molecule.clone().map(MoleculeData::Hash),
            Self::Range(f) => f.base.molecule.clone().map(MoleculeData::Range),
            Self::HashRange(f) => f.base.molecule.clone().map(MoleculeData::HashRange),
        }
    }

    /// Returns all keys present in this field's molecule.
    pub fn get_all_keys(&self) -> Vec<KeyValue> {
        match self {
            Self::Single(_) => vec![KeyValue::new(None, None)],
            Self::Hash(f) => f
                .get_all_keys()
                .into_iter()
                .map(|hash| KeyValue::new(Some(hash), None))
                .collect(),
            Self::Range(f) => f
                .get_all_keys()
                .into_iter()
                .map(|range| KeyValue::new(None, Some(range)))
                .collect(),
            Self::HashRange(f) => f.get_all_keys(),
        }
    }

    /// Returns the current molecule version, if a molecule is present.
    #[must_use]
    pub fn molecule_version(&self) -> Option<u64> {
        match self {
            Self::Single(f) => f.base.molecule.as_ref().map(|m| m.version()),
            Self::Hash(f) => f.base.molecule.as_ref().map(|m| m.version()),
            Self::Range(f) => f.base.molecule.as_ref().map(|m| m.version()),
            Self::HashRange(f) => f.base.molecule.as_ref().map(|m| m.version()),
        }
    }

    /// Rewinds the in-memory molecule to its state at the given point in time
    /// by undoing all mutation events that occurred after `as_of`.
    async fn rewind_to(
        &mut self,
        db_ops: &Arc<DbOperations>,
        as_of: DateTime<Utc>,
    ) -> Result<(), SchemaError> {
        let mol_uuid = match self.common().molecule_uuid() {
            Some(uuid) => uuid.to_string(),
            None => return Ok(()),
        };

        let events = db_ops
            .get_mutation_events(&mol_uuid, self.common().org_hash())
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to load history: {}", e)))?;

        // Get events AFTER as_of, in reverse chronological order
        let mut events_to_undo: Vec<&MutationEvent> =
            events.iter().filter(|e| e.timestamp > as_of).collect();
        events_to_undo.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Undo each event (restore old state)
        for event in events_to_undo {
            match (&event.field_key, &mut *self) {
                (FieldKey::Single, FieldVariant::Single(f)) => {
                    match &event.old_atom_uuid {
                        Some(old) => {
                            if let Some(mol) = &mut f.base.molecule {
                                mol.set_atom_uuid(old.clone());
                            }
                        }
                        None => {
                            // Field didn't exist before this mutation — clear molecule
                            f.base.molecule = None;
                        }
                    }
                }
                (FieldKey::Hash { hash }, FieldVariant::Hash(f)) => {
                    if let Some(mol) = &mut f.base.molecule {
                        match &event.old_atom_uuid {
                            Some(old) => {
                                mol.set_atom_uuid(hash.clone(), old.clone());
                            }
                            None => {
                                mol.remove_atom_uuid(hash);
                            }
                        }
                    }
                }
                (FieldKey::Range { range }, FieldVariant::Range(f)) => {
                    if let Some(mol) = &mut f.base.molecule {
                        match &event.old_atom_uuid {
                            Some(old) => {
                                mol.set_atom_uuid(range.clone(), old.clone());
                            }
                            None => {
                                mol.remove_atom_uuid(range);
                            }
                        }
                    }
                }
                (FieldKey::HashRange { hash, range }, FieldVariant::HashRange(f)) => {
                    if let Some(mol) = &mut f.base.molecule {
                        match &event.old_atom_uuid {
                            Some(old) => {
                                mol.set_atom_uuid_from_values(
                                    hash.clone(),
                                    range.clone(),
                                    old.clone(),
                                );
                            }
                            None => {
                                mol.remove_atom_uuid(hash, range);
                            }
                        }
                    }
                }
                _ => {} // Type mismatch — skip
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::{FieldKey, Molecule, MoleculeHashRange, MoleculeRange, MutationEvent};
    use crate::schema::types::declarative_schemas::FieldMapper;
    use chrono::{Duration, Utc};
    use std::collections::HashMap;

    // Helper to create a SingleField with a molecule pointing to a given atom_uuid
    fn make_single_field(atom_uuid: &str, mol_uuid: &str) -> FieldVariant {
        let mol = Molecule::new(atom_uuid.to_string(), "test_schema", "test_key");
        // Override the molecule UUID to a known value for event matching
        // We can't set it directly, so we use FieldCommon to track it
        let mut field = SingleField::new(HashMap::<String, FieldMapper>::new(), Some(mol));
        field.base.inner.set_molecule_uuid(mol_uuid.to_string());
        FieldVariant::Single(field)
    }

    // Helper to create a RangeField with entries
    fn make_range_field(entries: &[(&str, &str)], mol_uuid: &str) -> FieldVariant {
        let mut mol = MoleculeRange::new("test_schema", "test_key");
        for (range_key, atom_uuid) in entries {
            mol.set_atom_uuid(range_key.to_string(), atom_uuid.to_string());
        }
        let mut field = RangeField::new(HashMap::<String, FieldMapper>::new(), Some(mol));
        field.base.inner.set_molecule_uuid(mol_uuid.to_string());
        FieldVariant::Range(field)
    }

    // Helper to create a HashRangeField with entries
    fn make_hash_range_field(entries: &[(&str, &str, &str)], mol_uuid: &str) -> FieldVariant {
        let mut mol = MoleculeHashRange::new("test_schema", "test_key");
        for (hash, range, atom_uuid) in entries {
            mol.set_atom_uuid_from_values(
                hash.to_string(),
                range.to_string(),
                atom_uuid.to_string(),
            );
        }
        let mut field = HashRangeField::new(HashMap::<String, FieldMapper>::new(), Some(mol));
        field.base.inner.set_molecule_uuid(mol_uuid.to_string());
        FieldVariant::HashRange(field)
    }

    #[tokio::test]
    async fn test_rewind_single_field_to_previous_value() {
        // Setup: Sled temp DB
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let db_ops = Arc::new(
            crate::db_operations::DbOperations::from_sled(db)
                .await
                .unwrap(),
        );

        let mol_uuid = "mol-single-1";
        let t0 = Utc::now() - Duration::seconds(10);
        let t1 = t0 + Duration::seconds(2);
        let t2 = t0 + Duration::seconds(5);

        // Store mutation events: v1 at t1, v2 at t2
        let events = vec![
            MutationEvent {
                molecule_uuid: mol_uuid.to_string(),
                timestamp: t1,
                field_key: FieldKey::Single,
                old_atom_uuid: None,
                new_atom_uuid: "atom-v1".to_string(),
                version: 0,
                is_conflict: false,
                conflict_loser_atom: None,
            },
            MutationEvent {
                molecule_uuid: mol_uuid.to_string(),
                timestamp: t2,
                field_key: FieldKey::Single,
                old_atom_uuid: Some("atom-v1".to_string()),
                new_atom_uuid: "atom-v2".to_string(),
                version: 0,
                is_conflict: false,
                conflict_loser_atom: None,
            },
        ];
        db_ops
            .batch_store_mutation_events(events, None)
            .await
            .unwrap();

        // Current state: molecule points to atom-v2
        let mut field = make_single_field("atom-v2", mol_uuid);

        // Rewind to between t1 and t2 -> should get atom-v1
        let as_of = t1 + Duration::seconds(1);
        field.rewind_to(&db_ops, as_of).await.unwrap();

        match &field {
            FieldVariant::Single(f) => {
                let mol = f.base.molecule.as_ref().unwrap();
                assert_eq!(mol.get_atom_uuid(), "atom-v1");
            }
            _ => panic!("Expected Single"),
        }
    }

    #[tokio::test]
    async fn test_rewind_single_field_to_before_any_mutation() {
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let db_ops = Arc::new(
            crate::db_operations::DbOperations::from_sled(db)
                .await
                .unwrap(),
        );

        let mol_uuid = "mol-single-2";
        let t0 = Utc::now() - Duration::seconds(10);
        let t1 = t0 + Duration::seconds(2);

        let events = vec![MutationEvent {
            molecule_uuid: mol_uuid.to_string(),
            timestamp: t1,
            field_key: FieldKey::Single,
            old_atom_uuid: None,
            new_atom_uuid: "atom-v1".to_string(),
            version: 0,
            is_conflict: false,
            conflict_loser_atom: None,
        }];
        db_ops
            .batch_store_mutation_events(events, None)
            .await
            .unwrap();

        let mut field = make_single_field("atom-v1", mol_uuid);

        // Rewind to before t1 -> molecule should be cleared
        field.rewind_to(&db_ops, t0).await.unwrap();

        match &field {
            FieldVariant::Single(f) => {
                assert!(f.base.molecule.is_none());
            }
            _ => panic!("Expected Single"),
        }
    }

    #[tokio::test]
    async fn test_rewind_range_field() {
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let db_ops = Arc::new(
            crate::db_operations::DbOperations::from_sled(db)
                .await
                .unwrap(),
        );

        let mol_uuid = "mol-range-1";
        let t0 = Utc::now() - Duration::seconds(10);
        let t1 = t0 + Duration::seconds(2);
        let t2 = t0 + Duration::seconds(5);

        // key1 added at t1, key2 added at t2
        let events = vec![
            MutationEvent {
                molecule_uuid: mol_uuid.to_string(),
                timestamp: t1,
                field_key: FieldKey::Range {
                    range: "key1".to_string(),
                },
                old_atom_uuid: None,
                new_atom_uuid: "atom-k1".to_string(),
                version: 0,
                is_conflict: false,
                conflict_loser_atom: None,
            },
            MutationEvent {
                molecule_uuid: mol_uuid.to_string(),
                timestamp: t2,
                field_key: FieldKey::Range {
                    range: "key2".to_string(),
                },
                old_atom_uuid: None,
                new_atom_uuid: "atom-k2".to_string(),
                version: 0,
                is_conflict: false,
                conflict_loser_atom: None,
            },
        ];
        db_ops
            .batch_store_mutation_events(events, None)
            .await
            .unwrap();

        // Current state: both keys exist
        let mut field = make_range_field(&[("key1", "atom-k1"), ("key2", "atom-k2")], mol_uuid);

        // Rewind to between t1 and t2 -> only key1 should exist
        let as_of = t1 + Duration::seconds(1);
        field.rewind_to(&db_ops, as_of).await.unwrap();

        match &field {
            FieldVariant::Range(f) => {
                let mol = f.base.molecule.as_ref().unwrap();
                assert_eq!(mol.get_atom_uuid("key1"), Some(&"atom-k1".to_string()));
                assert_eq!(mol.get_atom_uuid("key2"), None);
            }
            _ => panic!("Expected Range"),
        }
    }

    #[tokio::test]
    async fn test_rewind_hash_range_field() {
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let db_ops = Arc::new(
            crate::db_operations::DbOperations::from_sled(db)
                .await
                .unwrap(),
        );

        let mol_uuid = "mol-hr-1";
        let t0 = Utc::now() - Duration::seconds(10);
        let t1 = t0 + Duration::seconds(2);
        let t2 = t0 + Duration::seconds(5);

        // (h1,r1) set to atom-v1 at t1, updated to atom-v2 at t2
        let events = vec![
            MutationEvent {
                molecule_uuid: mol_uuid.to_string(),
                timestamp: t1,
                field_key: FieldKey::HashRange {
                    hash: "h1".to_string(),
                    range: "r1".to_string(),
                },
                old_atom_uuid: None,
                new_atom_uuid: "atom-v1".to_string(),
                version: 0,
                is_conflict: false,
                conflict_loser_atom: None,
            },
            MutationEvent {
                molecule_uuid: mol_uuid.to_string(),
                timestamp: t2,
                field_key: FieldKey::HashRange {
                    hash: "h1".to_string(),
                    range: "r1".to_string(),
                },
                old_atom_uuid: Some("atom-v1".to_string()),
                new_atom_uuid: "atom-v2".to_string(),
                version: 0,
                is_conflict: false,
                conflict_loser_atom: None,
            },
        ];
        db_ops
            .batch_store_mutation_events(events, None)
            .await
            .unwrap();

        let mut field = make_hash_range_field(&[("h1", "r1", "atom-v2")], mol_uuid);

        // Rewind to between t1 and t2 -> should get atom-v1
        let as_of = t1 + Duration::seconds(1);
        field.rewind_to(&db_ops, as_of).await.unwrap();

        match &field {
            FieldVariant::HashRange(f) => {
                let mol = f.base.molecule.as_ref().unwrap();
                assert_eq!(mol.get_atom_uuid("h1", "r1"), Some(&"atom-v1".to_string()));
            }
            _ => panic!("Expected HashRange"),
        }
    }

    #[tokio::test]
    async fn test_rewind_aba_cycle() {
        // A->B->A cycle: the event log correctly preserves intermediate state
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let db_ops = Arc::new(
            crate::db_operations::DbOperations::from_sled(db)
                .await
                .unwrap(),
        );

        let mol_uuid = "mol-aba";
        let t0 = Utc::now() - Duration::seconds(10);
        let t1 = t0 + Duration::seconds(1);
        let t2 = t0 + Duration::seconds(3);
        let t3 = t0 + Duration::seconds(5);

        // v1="hello" at t1, v2="world" at t2, v3="hello" at t3
        // Note: "hello" atoms have same UUID since content-addressed
        let events = vec![
            MutationEvent {
                molecule_uuid: mol_uuid.to_string(),
                timestamp: t1,
                field_key: FieldKey::Single,
                old_atom_uuid: None,
                new_atom_uuid: "atom-hello".to_string(),
                version: 0,
                is_conflict: false,
                conflict_loser_atom: None,
            },
            MutationEvent {
                molecule_uuid: mol_uuid.to_string(),
                timestamp: t2,
                field_key: FieldKey::Single,
                old_atom_uuid: Some("atom-hello".to_string()),
                new_atom_uuid: "atom-world".to_string(),
                version: 0,
                is_conflict: false,
                conflict_loser_atom: None,
            },
            MutationEvent {
                molecule_uuid: mol_uuid.to_string(),
                timestamp: t3,
                field_key: FieldKey::Single,
                old_atom_uuid: Some("atom-world".to_string()),
                new_atom_uuid: "atom-hello".to_string(),
                version: 0,
                is_conflict: false,
                conflict_loser_atom: None,
            },
        ];
        db_ops
            .batch_store_mutation_events(events, None)
            .await
            .unwrap();

        // Current state: "hello" (atom-hello)
        let mut field = make_single_field("atom-hello", mol_uuid);

        // Rewind to between t2 and t3 -> should get "world"
        let as_of = t2 + Duration::seconds(1);
        field.rewind_to(&db_ops, as_of).await.unwrap();

        match &field {
            FieldVariant::Single(f) => {
                let mol = f.base.molecule.as_ref().unwrap();
                assert_eq!(mol.get_atom_uuid(), "atom-world");
            }
            _ => panic!("Expected Single"),
        }
    }

    #[tokio::test]
    async fn test_rewind_no_events() {
        // No events stored — rewind should be a no-op
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let db_ops = Arc::new(
            crate::db_operations::DbOperations::from_sled(db)
                .await
                .unwrap(),
        );

        let mol_uuid = "mol-empty";
        let mut field = make_single_field("atom-current", mol_uuid);

        field.rewind_to(&db_ops, Utc::now()).await.unwrap();

        match &field {
            FieldVariant::Single(f) => {
                let mol = f.base.molecule.as_ref().unwrap();
                assert_eq!(mol.get_atom_uuid(), "atom-current");
            }
            _ => panic!("Expected Single"),
        }
    }
}
