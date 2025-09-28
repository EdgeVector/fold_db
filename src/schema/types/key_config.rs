use serde::{Serialize, Deserialize};
use ts_rs::TS;

// Forward declarations for types that need to be defined
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "src/datafold_node/static-react/src/types/generated.ts")]
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

    /// Creates a KeyConfig from a HashMap of string key-value pairs
    pub fn from_map(map: std::collections::HashMap<String, String>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            hash_field: map.get("hash_field").cloned(),
            range_field: map.get("range_field").cloned(),
        })
    }
}