use serde::{Deserialize, Serialize};

/// Represents resolved key values for hash and range components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyValue {
    pub hash: Option<String>,
    pub range: Option<String>,
}

impl KeyValue {
    pub fn new(hash: Option<String>, range: Option<String>) -> Self {
        Self { hash, range }
    }
}


