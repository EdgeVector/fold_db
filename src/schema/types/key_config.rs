use serde::{Serialize, Deserialize};

// Forward declarations for types that need to be defined
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyConfig {
    pub hash_field: String,
    pub range_field: String,
}