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
}

impl std::fmt::Display for KeyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(hash) = &self.hash {
            if let Some(range) = &self.range {
                write!(f, "{}:{}", hash, range)
            } else {
                write!(f, "{}", hash)
            }
        } else if let Some(range) = &self.range {
            write!(f, "{}", range)
        } else {
            write!(f, "")
        }
    }
}


