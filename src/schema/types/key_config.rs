use serde::{Serialize, Deserialize};

// Forward declarations for types that need to be defined
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]

// Used in two ways, one to set the field_names which will be used to resolve the keys, and one to pass key values in a mutation.
pub struct KeyConfig {
    pub hash_field: Option<String>,
    pub range_field: Option<String>,
}

impl KeyConfig {
    /// Creates a new KeyConfig with the specified hash and range field names
    pub fn new(hash_field: Option<String>, range_field: Option<String>) -> Self {
        Self {
            hash_field,
            range_field,
        }
    }
}