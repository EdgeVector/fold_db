use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// An immutable data container that represents a single version of content in the database.
///
/// Atoms are the fundamental building blocks of the database's immutable data storage system.
/// Each Atom contains:
/// - A unique identifier (content-addressed)
/// - The source schema that defines its structure
/// - The public key of the creator
/// - Creation timestamp
/// - The actual content data
///
/// Version history is tracked via the delta event log (`MutationEvent`),
/// not via atom-level chaining. Once created, an Atom's content cannot be modified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    uuid: String,
    source_schema_name: String,
    source_pub_key: String,
    source_file_name: Option<String>,
    created_at: DateTime<Utc>,
    content: Value,
}

impl Atom {
    /// Generates a deterministic UUID based on atom content.
    /// This enables content-based deduplication at the atom level.
    ///
    /// # Arguments
    ///
    /// * `source_schema_name` - Name of the schema that defines this Atom's structure
    /// * `content` - The actual data content stored in this Atom
    ///
    /// # Returns
    ///
    /// A deterministic UUID string based on SHA256 hash of schema name and content
    fn generate_content_uuid(source_schema_name: &str, content: &Value) -> String {
        let mut hasher = Sha256::new();
        hasher.update(source_schema_name.as_bytes());
        hasher.update(content.to_string().as_bytes());
        let hash = hasher.finalize();
        format!("{:x}", hash)
    }

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
    /// A new Atom instance with a content-based UUID and current timestamp
    #[must_use]
    pub fn new(source_schema_name: String, source_pub_key: String, content: Value) -> Self {
        let uuid = Self::generate_content_uuid(&source_schema_name, &content);
        Self {
            uuid,
            source_schema_name,
            source_pub_key,
            source_file_name: None,
            created_at: Utc::now(),
            content,
        }
    }

    /// Sets the source file name for atoms created from file uploads
    #[must_use]
    pub fn with_source_file_name(mut self, file_name: String) -> Self {
        self.source_file_name = Some(file_name);
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

    /// Returns the original filename if this atom was created from a file upload.
    ///
    /// This is used for tracking data provenance and auditing purposes.
    #[must_use]
    pub fn source_file_name(&self) -> Option<&String> {
        self.source_file_name.as_ref()
    }

    /// Returns the timestamp when this Atom was created.
    ///
    /// This timestamp is used for auditing and version history tracking.
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
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
        assert_eq!(atom.content(), &content);
        assert!(!atom.uuid().is_empty());
        assert!(atom.created_at() <= Utc::now());
    }

    #[test]
    fn test_molecule_creation_and_update() {
        use crate::atom::Molecule;

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
        );

        let mut updated_ref = molecule.clone();
        updated_ref.set_atom_uuid(new_atom.uuid().to_string());

        assert_eq!(updated_ref.get_atom_uuid(), &new_atom.uuid().to_string());
        assert!(updated_ref.updated_at() >= molecule.updated_at());
    }
}
