use super::core::DbOperations;
use crate::atom::{Atom, AtomRef, AtomRefRange, AtomStatus};
use crate::schema::SchemaError;
use crate::logging::features::{log_feature, LogFeature};
use serde_json::Value;

impl DbOperations {
    /// Creates a new atom and stores it in the database
    pub fn create_atom(
        &self,
        schema_name: &str,
        source_pub_key: String,
        prev_atom_uuid: Option<String>,
        content: Value,
        status: Option<AtomStatus>,
    ) -> Result<Atom, SchemaError> {
        let mut atom = Atom::new(schema_name.to_string(), source_pub_key, content);

        // Only set prev_atom_uuid if it's Some
        if let Some(prev_uuid) = prev_atom_uuid {
            if !prev_uuid.is_empty() {
                atom = atom.with_prev_version(prev_uuid);
            }
        }

        atom = atom.with_status(status.unwrap_or(AtomStatus::Active));
        // Persist with an "atom:" prefix so we can easily distinguish entries
        // when reloading from disk
        self.store_item(&format!("atom:{}", atom.uuid()), &atom)?;
        Ok(atom)
    }

    /// Creates or updates a single atom reference
    pub fn update_atom_ref(
        &self,
        aref_uuid: &str,
        atom_uuid: String,
        source_pub_key: String,
    ) -> Result<AtomRef, SchemaError> {
        // DIAGNOSTIC: Log the update attempt
        log_feature!(LogFeature::Database, info, "🔍 DIAGNOSTIC: update_atom_ref called - aref_uuid: {}, atom_uuid: {}", aref_uuid, atom_uuid);
        
        let mut aref = match self.get_item::<AtomRef>(&format!("ref:{}", aref_uuid))? {
            Some(existing_aref) => {
                log_feature!(LogFeature::Database, info, "🔍 DIAGNOSTIC: Found existing AtomRef - current atom_uuid: {}", existing_aref.get_atom_uuid());
                existing_aref
            }
            None => {
                log_feature!(LogFeature::Database, info, "🔍 DIAGNOSTIC: Creating new AtomRef");
                AtomRef::new(atom_uuid.clone(), source_pub_key)
            }
        };

        // DIAGNOSTIC: Log before update
        log_feature!(LogFeature::Database, info, "🔍 DIAGNOSTIC: Before set_atom_uuid - current: {}, new: {}", aref.get_atom_uuid(), atom_uuid);
        
        aref.set_atom_uuid(atom_uuid.clone());
        
        // DIAGNOSTIC: Log after update
        log_feature!(LogFeature::Database, info, "🔍 DIAGNOSTIC: After set_atom_uuid - updated to: {}", aref.get_atom_uuid());
        
        // DIAGNOSTIC: Log before persistence
        log_feature!(LogFeature::Database, info, "🔍 DIAGNOSTIC: About to persist AtomRef with key: ref:{}", aref_uuid);
        
        self.store_item(&format!("ref:{}", aref_uuid), &aref)?;
        
        // DIAGNOSTIC: Verify persistence by reading back
        match self.get_item::<AtomRef>(&format!("ref:{}", aref_uuid))? {
            Some(persisted_aref) => {
                log_feature!(LogFeature::Database, info, "🔍 DIAGNOSTIC: Persistence verification - stored atom_uuid: {}", persisted_aref.get_atom_uuid());
                if persisted_aref.get_atom_uuid() != &atom_uuid {
                    log_feature!(LogFeature::Database, error, "❌ DIAGNOSTIC: PERSISTENCE MISMATCH! Expected: {}, Got: {}", atom_uuid, persisted_aref.get_atom_uuid());
                }
            }
            None => {
                log_feature!(LogFeature::Database, error, "❌ DIAGNOSTIC: PERSISTENCE FAILED! Could not read back stored AtomRef");
            }
        }
        
        Ok(aref)
    }


    /// Creates or updates a range of atom references
    pub fn update_atom_ref_range(
        &self,
        aref_uuid: &str,
        atom_uuid: String,
        key: String,
        source_pub_key: String,
    ) -> Result<AtomRefRange, SchemaError> {
        let mut aref = match self.get_item::<AtomRefRange>(&format!("ref:{}", aref_uuid))? {
            Some(existing_aref) => existing_aref,
            None => AtomRefRange::new(source_pub_key),
        };

        aref.set_atom_uuid(key, atom_uuid);
        self.store_item(&format!("ref:{}", aref_uuid), &aref)?;
        Ok(aref)
    }
}
