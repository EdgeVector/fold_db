use serde::{Deserialize, Serialize};

/// Represents resolved key values for hash and range components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct KeyValue {
    pub hash: Option<String>,
    pub range: Option<String>,
}

impl KeyValue {
    pub fn new(hash: Option<String>, range: Option<String>) -> Self {
        Self { hash, range }
    }

    pub fn to_string(&self) -> String {
        if let Some(hash) = &self.hash {
            if let Some(range) = &self.range {
                format!("{}:{}", hash, range)
            } else {
                hash.clone()
            }
        } else {
            self.range.clone().unwrap_or_default()
        }
    }
}


