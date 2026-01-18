use super::key_config::KeyConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Represents resolved key values for hash and range components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(
        export,
        export_to = "bindings/src/datafold_node/static-react/src/types/generated.ts"
    )
)]
pub struct KeyValue {
    pub hash: Option<String>,
    pub range: Option<String>,
}

impl KeyValue {
    pub fn new(hash: Option<String>, range: Option<String>) -> Self {
        Self { hash, range }
    }

    /// Creates a KeyValue from a mutation by extracting hash and range values
    /// based on the key configuration
    pub fn from_mutation(mutation_fields: &HashMap<String, Value>, key_config: &KeyConfig) -> Self {
        let mut key_value = Self::new(None, None);

        if let Some(hash_field) = &key_config.hash_field {
            if let Some(value) = mutation_fields.get(hash_field) {
                if let Some(s) = value.as_str() {
                    key_value.hash = Some(s.to_string());
                }
            }
        }

        if let Some(range_field) = &key_config.range_field {
            if let Some(value) = mutation_fields.get(range_field) {
                if let Some(s) = value.as_str() {
                    key_value.range = Some(s.to_string());
                }
            }
        }

        key_value
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
