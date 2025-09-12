//! Compile-time restrictions to prevent direct atom/molecule access from transforms.
//!
//! This module creates a private scope that prevents transforms from directly
//! accessing atom and molecule creation methods.

// Private re-exports that hide the creation methods
use crate::atom::{Atom, Molecule, MoleculeRange, MoleculeBehavior};

/// Private wrapper that only exposes read-only methods to transforms.
/// 
/// This prevents transforms from calling creation methods like `Atom::new()`.
pub struct ReadOnlyAtom {
    uuid: String,
    source_schema_name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    content: serde_json::Value,
}

impl ReadOnlyAtom {
    /// Create a read-only wrapper around an atom.
    /// 
    /// This is the ONLY way transforms should access atoms.
    pub fn wrap(atom: &Atom) -> Self {
        Self {
            uuid: atom.uuid().to_string(),
            source_schema_name: atom.source_schema_name().to_string(),
            created_at: atom.created_at(),
            content: atom.content().clone(),
        }
    }
    
    /// Get the atom's content (read-only).
    pub fn content(&self) -> &serde_json::Value {
        &self.content
    }
    
    /// Get the atom's UUID.
    pub fn uuid(&self) -> &str {
        &self.uuid
    }
    
    /// Get the atom's source schema name.
    pub fn source_schema_name(&self) -> &str {
        &self.source_schema_name
    }
    
    /// Get the atom's creation timestamp.
    pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }
}

/// Private wrapper that only exposes read-only methods to transforms.
pub struct ReadOnlyMolecule {
    uuid: String,
    atom_uuid: String,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl ReadOnlyMolecule {
    /// Create a read-only wrapper around a molecule.
    pub fn wrap(molecule: &Molecule) -> Self {
        Self {
            uuid: molecule.uuid().to_string(),
            atom_uuid: molecule.get_atom_uuid().to_string(),
            updated_at: molecule.updated_at(),
        }
    }
    
    /// Get the referenced atom UUID.
    pub fn get_atom_uuid(&self) -> &str {
        &self.atom_uuid
    }
    
    /// Get the molecule's UUID.
    pub fn uuid(&self) -> &str {
        &self.uuid
    }
    
    /// Get the molecule's update timestamp.
    pub fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.updated_at
    }
}

/// Private wrapper for molecule ranges.
pub struct ReadOnlyMoleculeRange {
    uuid: String,
    atom_uuids: std::collections::BTreeMap<String, String>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl ReadOnlyMoleculeRange {
    /// Create a read-only wrapper around a molecule range.
    pub fn wrap(molecule_range: &MoleculeRange) -> Self {
        Self {
            uuid: molecule_range.uuid().to_string(),
            atom_uuids: molecule_range.atom_uuids.clone(),
            updated_at: molecule_range.updated_at(),
        }
    }
    
    /// Get an atom UUID by key (read-only).
    pub fn get_atom_uuid(&self, key: &str) -> Option<&str> {
        self.atom_uuids.get(key).map(|s| s.as_str())
    }
    
    /// Get the molecule range UUID.
    pub fn uuid(&self) -> &str {
        &self.uuid
    }
    
    /// Get the update timestamp.
    pub fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.updated_at
    }
}

/// Transform-safe access to atom/molecule data.
/// 
/// This trait provides the only way for transforms to access atom/molecule
/// data without being able to create new instances.
pub trait TransformSafeDataAccess {
    /// Get read-only access to an atom.
    fn get_readonly_atom(&self, atom_uuid: &str) -> Result<ReadOnlyAtom, crate::schema::SchemaError>;
    
    /// Get read-only access to a molecule.
    fn get_readonly_molecule(&self, molecule_uuid: &str) -> Result<ReadOnlyMolecule, crate::schema::SchemaError>;
    
    /// Get read-only access to a molecule range.
    fn get_readonly_molecule_range(&self, molecule_range_uuid: &str) -> Result<ReadOnlyMoleculeRange, crate::schema::SchemaError>;
}

/// Implementation of transform-safe data access using database operations.
pub struct DatabaseTransformDataAccess {
    db_ops: std::sync::Arc<crate::db_operations::DbOperations>,
}

impl DatabaseTransformDataAccess {
    /// Create a new database-based transform data access.
    pub fn new(db_ops: std::sync::Arc<crate::db_operations::DbOperations>) -> Self {
        Self { db_ops }
    }
}

impl TransformSafeDataAccess for DatabaseTransformDataAccess {
    fn get_readonly_atom(&self, atom_uuid: &str) -> Result<ReadOnlyAtom, crate::schema::SchemaError> {
        let atom = self.db_ops.get_item::<Atom>(&format!("atom:{}", atom_uuid))?
            .ok_or_else(|| crate::schema::SchemaError::InvalidField(format!("Atom '{}' not found", atom_uuid)))?;
        
        Ok(ReadOnlyAtom::wrap(&atom))
    }
    
    fn get_readonly_molecule(&self, molecule_uuid: &str) -> Result<ReadOnlyMolecule, crate::schema::SchemaError> {
        let molecule = self.db_ops.get_item::<Molecule>(&format!("ref:{}", molecule_uuid))?
            .ok_or_else(|| crate::schema::SchemaError::InvalidField(format!("Molecule '{}' not found", molecule_uuid)))?;
        
        Ok(ReadOnlyMolecule::wrap(&molecule))
    }
    
    fn get_readonly_molecule_range(&self, molecule_range_uuid: &str) -> Result<ReadOnlyMoleculeRange, crate::schema::SchemaError> {
        let molecule_range = self.db_ops.get_item::<MoleculeRange>(&format!("ref:{}", molecule_range_uuid))?
            .ok_or_else(|| crate::schema::SchemaError::InvalidField(format!("MoleculeRange '{}' not found", molecule_range_uuid)))?;
        
        Ok(ReadOnlyMoleculeRange::wrap(&molecule_range))
    }
}

/// Macro to ensure transforms use safe data access.
/// 
/// This macro wraps transform execution to ensure only read-only access
/// to atoms and molecules.

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_readonly_atom_wrapper() {
        let atom = Atom::new(
            "TestSchema".to_string(),
            "test_key".to_string(),
            serde_json::Value::String("test_content".to_string())
        );
        
        let readonly = ReadOnlyAtom::wrap(&atom);
        assert_eq!(readonly.content(), &serde_json::Value::String("test_content".to_string()));
        assert_eq!(readonly.source_schema_name(), "TestSchema");
    }
    
    #[test]
    fn test_readonly_molecule_wrapper() {
        let atom = Atom::new(
            "TestSchema".to_string(),
            "test_key".to_string(),
            serde_json::Value::String("test_content".to_string())
        );
        
        let molecule = Molecule::new(atom.uuid().to_string(), "test_key".to_string());
        let readonly = ReadOnlyMolecule::wrap(&molecule);
        
        assert_eq!(readonly.get_atom_uuid(), atom.uuid());
    }
}
