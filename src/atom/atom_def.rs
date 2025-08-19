use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// An immutable data container that represents a single version of content in the database.
///
/// Atoms are the fundamental building blocks of the database's immutable data storage system.
/// Each Atom contains:
/// - A unique identifier
/// - The source schema that defines its structure
/// - The public key of the creator
/// - Creation timestamp
/// - Optional reference to a previous version
/// - The actual content data
///
/// Atoms form a chain of versions through their `prev_atom_uuid` references, enabling
/// complete version history tracking. Once created, an Atom's content cannot be modified,
/// ensuring data immutability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    uuid: String,
    source_schema_name: String,
    source_pub_key: String,
    created_at: DateTime<Utc>,
    prev_atom_uuid: Option<String>,
    content: Value,
    status: AtomStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AtomStatus {
    Active,
    Deleted,
}

impl Atom {
    /// Creates a new Atom with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `source_schema_name` - Name of the schema that defines this Atom's structure
    /// * `source_pub_key` - Public key of the entity creating this Atom
    /// * `content` - The actual data content stored in this Atom
    ///
    /// # Returns
    ///
    /// A new Atom instance with a generated UUID and current timestamp
    #[must_use]
    pub fn new(source_schema_name: String, source_pub_key: String, content: Value) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            source_schema_name,
            source_pub_key,
            created_at: Utc::now(),
            prev_atom_uuid: None,
            content,
            status: AtomStatus::Active,
        }
    }

    /// Creates a new Atom that references a previous version
    #[must_use]
    pub fn with_prev_version(mut self, prev_atom_uuid: String) -> Self {
        self.prev_atom_uuid = Some(prev_atom_uuid);
        self
    }

    pub fn with_status(mut self, status: AtomStatus) -> Self {
        self.status = status;
        self
    }

    /// Returns a reference to the Atom's content.
    ///
    /// This method provides read-only access to the stored data,
    /// maintaining the immutability principle.
    #[must_use]
    pub const fn content(&self) -> &Value {
        &self.content
    }

    pub fn set_status(&mut self, status: AtomStatus) {
        self.status = status;
    }

    /// Applies a transformation to the Atom's content and returns the result.
    ///
    /// Currently supports:
    /// - "lowercase": Converts string content to lowercase
    ///
    /// If the transformation is not recognized or cannot be applied,
    /// returns a clone of the original content.
    ///
    /// # Arguments
    ///
    /// * `transform` - The name of the transformation to apply
    #[must_use]
    pub fn get_transformed_content(&self, transform: &str) -> Value {
        match transform {
            "lowercase" => {
                if let Value::String(s) = &self.content {
                    Value::String(s.to_lowercase())
                } else {
                    self.content.clone()
                }
            }
            _ => self.content.clone(),
        }
    }

    /// Returns the unique identifier of this Atom.
    ///
    /// This UUID uniquely identifies this specific version of the data
    /// and is used by Molecules to point to the current version.
    #[must_use]
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    /// Returns the name of the schema that defines this Atom's structure.
    ///
    /// The schema name is used to validate the content structure and
    /// determine applicable permissions and payment requirements.
    #[must_use]
    pub fn source_schema_name(&self) -> &str {
        &self.source_schema_name
    }

    /// Returns the public key of the entity that created this Atom.
    ///
    /// This is used for authentication and permission validation
    /// when accessing or modifying the data.
    #[must_use]
    pub fn source_pub_key(&self) -> &str {
        &self.source_pub_key
    }

    /// Returns the timestamp when this Atom was created.
    ///
    /// This timestamp is used for auditing and version history tracking.
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns the UUID of the previous version of this data, if any.
    ///
    /// This forms the chain of version history, allowing traversal
    /// through all previous versions of the data.
    #[must_use]
    pub const fn prev_atom_uuid(&self) -> Option<&String> {
        self.prev_atom_uuid.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_atom_creation() {
        let content = json!({
            "name": "test",
            "value": 42
        });

        let atom = Atom::new(
            "test_schema".to_string(),
            "test_key".to_string(),
            content.clone(),
        );

        assert_eq!(atom.source_schema_name(), "test_schema");
        assert_eq!(atom.source_pub_key(), "test_key");
        assert_eq!(atom.prev_atom_uuid(), None);
        assert_eq!(atom.content(), &content);
        assert!(!atom.uuid().is_empty());
        assert!(atom.created_at() <= Utc::now());
    }

    #[test]
    fn test_atom_with_prev_reference() {
        let first_atom = Atom::new(
            "test_schema".to_string(),
            "test_key".to_string(),
            json!({"version": 1}),
        );

        let second_atom =
            Atom::with_prev_version(first_atom.clone(), first_atom.uuid().to_string());

        assert_eq!(
            second_atom.prev_atom_uuid(),
            Some(&first_atom.uuid().to_string())
        );
    }

    #[test]
    fn test_molecule_creation_and_update() {
        use crate::atom::Molecule;
        use crate::atom::MoleculeBehavior;

        let atom = Atom::new(
            "test_schema".to_string(),
            "test_key".to_string(),
            json!({"test": true}),
        );

        // Test single molecule
        let molecule = Molecule::new(atom.uuid().to_string(), "test_key".to_string());
        assert_eq!(molecule.get_atom_uuid(), &atom.uuid().to_string());

        let new_atom = Atom::new(
            "test_schema".to_string(),
            "test_key".to_string(),
            json!({"test": false}),
        )
        .with_prev_version(atom.uuid().to_string());

        let mut updated_ref = molecule.clone();
        updated_ref.set_atom_uuid(new_atom.uuid().to_string());

        assert_eq!(updated_ref.get_atom_uuid(), &new_atom.uuid().to_string());
        assert!(updated_ref.updated_at() >= molecule.updated_at());
    }
} 