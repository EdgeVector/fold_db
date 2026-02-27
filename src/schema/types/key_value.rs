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
        export_to = "bindings/src/fold_node/static-react/src/types/generated.ts"
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
    /// based on the key configuration. Supports dotted nested paths (e.g., "departure.date").
    pub fn from_mutation(mutation_fields: &HashMap<String, Value>, key_config: &KeyConfig) -> Self {
        let mut key_value = Self::new(None, None);

        if let Some(hash_field) = &key_config.hash_field {
            key_value.hash = resolve_field_as_string(mutation_fields, hash_field);
        }

        if let Some(range_field) = &key_config.range_field {
            key_value.range = resolve_field_as_string(mutation_fields, range_field);
        }

        key_value
    }
}

/// Resolve a field value as a string, supporting dotted nested paths (e.g., "departure.date").
fn resolve_field_as_string(fields: &HashMap<String, Value>, field_name: &str) -> Option<String> {
    // 1. Try direct field access
    if let Some(value) = fields.get(field_name) {
        return value_to_string(value);
    }
    // 2. Try dotted path (e.g., "parent.child")
    if let Some(dot) = field_name.find('.') {
        let (parent, child) = (&field_name[..dot], &field_name[dot + 1..]);
        if let Some(parent_val) = fields.get(parent) {
            if let Some(obj) = parent_val.as_object() {
                if let Some(child_val) = obj.get(child) {
                    return value_to_string(child_val);
                }
            }
        }
    }
    None
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_key_config(hash_field: Option<&str>, range_field: Option<&str>) -> KeyConfig {
        KeyConfig {
            hash_field: hash_field.map(String::from),
            range_field: range_field.map(String::from),
        }
    }

    #[test]
    fn test_from_mutation_direct_field() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), json!("Alice"));
        fields.insert("date".to_string(), json!("2025-03-15"));

        let kv = KeyValue::from_mutation(&fields, &make_key_config(Some("name"), Some("date")));
        assert_eq!(kv.hash, Some("Alice".to_string()));
        assert_eq!(kv.range, Some("2025-03-15".to_string()));
    }

    #[test]
    fn test_from_mutation_nested_dotted_path() {
        let mut fields = HashMap::new();
        fields.insert("departure".to_string(), json!({"date": "2025-03-15", "city": "NYC"}));
        fields.insert("booking_id".to_string(), json!("BK-001"));

        let kv = KeyValue::from_mutation(&fields, &make_key_config(Some("booking_id"), Some("departure.date")));
        assert_eq!(kv.hash, Some("BK-001".to_string()));
        assert_eq!(kv.range, Some("2025-03-15".to_string()));
    }

    #[test]
    fn test_from_mutation_missing_field() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), json!("Alice"));

        let kv = KeyValue::from_mutation(&fields, &make_key_config(Some("name"), Some("nonexistent.field")));
        assert_eq!(kv.hash, Some("Alice".to_string()));
        assert_eq!(kv.range, None);
    }

    #[test]
    fn test_from_mutation_numeric_value() {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), json!(42));
        fields.insert("details".to_string(), json!({"count": 7}));

        let kv = KeyValue::from_mutation(&fields, &make_key_config(Some("id"), Some("details.count")));
        assert_eq!(kv.hash, Some("42".to_string()));
        assert_eq!(kv.range, Some("7".to_string()));
    }

    #[test]
    fn test_from_mutation_bool_value() {
        let mut fields = HashMap::new();
        fields.insert("active".to_string(), json!(true));

        let kv = KeyValue::from_mutation(&fields, &make_key_config(Some("active"), None));
        assert_eq!(kv.hash, Some("true".to_string()));
    }

    #[test]
    fn test_from_mutation_object_value_returns_none() {
        let mut fields = HashMap::new();
        fields.insert("departure".to_string(), json!({"date": "2025-03-15"}));

        // Requesting the object itself (not a leaf) should return None
        let kv = KeyValue::from_mutation(&fields, &make_key_config(Some("departure"), None));
        assert_eq!(kv.hash, None);
    }
}
