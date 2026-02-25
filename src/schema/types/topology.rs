use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

use crate::schema::SchemaError;

/// Represents the topology (structure) of a JSON field
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export))]
pub struct JsonTopology {
    /// Root structure definition
    pub root: TopologyNode,
}

/// Represents a node in the JSON topology tree
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export))]
#[serde(tag = "type")]
pub enum TopologyNode {
    /// Primitive type with optional classifications (required for index schemas)
    #[serde(rename = "Primitive")]
    Primitive {
        value: PrimitiveValueType,
        #[serde(skip_serializing_if = "Option::is_none")]
        classifications: Option<Vec<String>>,
    },
    /// Object with named fields and their topologies
    Object {
        value: HashMap<String, TopologyNode>,
    },
    /// Array of a specific type
    Array { value: Box<TopologyNode> },
    /// Reference to records in another schema (created during decomposition).
    /// Mirrors the indexing system's (schema, key) reference pattern.
    Reference {
        schema_name: String,
    },
    /// Any type (no validation)
    Any,
}

/// Primitive JSON value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export))]
pub enum PrimitiveValueType {
    String,
    Number,
    Boolean,
    Null,
}

/// Legacy type alias for backward compatibility
pub type PrimitiveType = PrimitiveValueType;

impl JsonTopology {
    /// Create a new topology with a root node
    pub fn new(root: TopologyNode) -> Self {
        Self { root }
    }

    /// Create a topology that accepts any structure
    pub fn any() -> Self {
        Self {
            root: TopologyNode::Any,
        }
    }

    /// Validate that a JSON value conforms to this topology
    pub fn validate(&self, value: &JsonValue) -> Result<(), SchemaError> {
        self.root.validate(value, "root")
    }

    /// Infer topology from a sample JSON value
    pub fn infer_from_value(value: &JsonValue) -> Self {
        Self {
            root: TopologyNode::infer_from_value(value),
        }
    }

    /// Compute a SHA256 hash of this topology
    /// This creates a unique fingerprint of the topology structure.
    /// Classifications are stripped before hashing because they are
    /// semantic annotations, not structural shape.
    /// Keys are sorted recursively to ensure deterministic hashing
    /// regardless of HashMap iteration order.
    pub fn compute_hash(&self) -> String {
        let value = serde_json::to_value(&self.root)
            .unwrap_or_else(|_| serde_json::Value::Object(Default::default()));
        let stripped = strip_classifications(&value);
        let sorted = sort_json_keys(&stripped);
        let canonical =
            serde_json::to_string(&sorted).unwrap_or_else(|_| "{}".to_string());
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Recursively strip "classifications" keys from a JSON value so that
/// semantic annotations do not affect the topology hash.
fn strip_classifications(value: &JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(map) => {
            let filtered: serde_json::Map<String, JsonValue> = map
                .iter()
                .filter(|(k, _)| k.as_str() != "classifications")
                .map(|(k, v)| (k.clone(), strip_classifications(v)))
                .collect();
            JsonValue::Object(filtered)
        }
        JsonValue::Array(arr) => JsonValue::Array(arr.iter().map(strip_classifications).collect()),
        other => other.clone(),
    }
}

/// Recursively sort all object keys in a JSON value so that serialization
/// is deterministic regardless of HashMap iteration order.
fn sort_json_keys(value: &JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(map) => {
            let mut entries: Vec<_> = map
                .iter()
                .map(|(k, v)| (k.clone(), sort_json_keys(v)))
                .collect();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));
            JsonValue::Object(entries.into_iter().collect())
        }
        JsonValue::Array(arr) => {
            JsonValue::Array(arr.iter().map(sort_json_keys).collect())
        }
        other => other.clone(),
    }
}

impl TopologyNode {
    /// Validate that a JSON value conforms to this topology node
    pub fn validate(&self, value: &JsonValue, path: &str) -> Result<(), SchemaError> {
        match self {
            // Any accepts everything
            TopologyNode::Any => Ok(()),

            // Primitive validations
            TopologyNode::Primitive {
                value: prim_type, ..
            } => {
                match (prim_type, value) {
                    (PrimitiveValueType::String, JsonValue::String(_)) => Ok(()),
                    (PrimitiveValueType::Number, JsonValue::Number(_)) => Ok(()),
                    (PrimitiveValueType::Boolean, JsonValue::Bool(_)) => Ok(()),
                    // Null is always acceptable for any primitive type (nullable fields)
                    (_, JsonValue::Null) => Ok(()),
                    _ => Err(SchemaError::InvalidData(format!(
                        "Topology validation failed at '{}': expected {:?}, got {:?}",
                        path,
                        prim_type,
                        value_type_name(value)
                    ))),
                }
            }

            // Object validation
            TopologyNode::Object {
                value: expected_fields,
            } => {
                if let JsonValue::Object(obj) = value {
                    for (field_name, field_topology) in expected_fields {
                        if let Some(field_value) = obj.get(field_name) {
                            let field_path = format!("{}.{}", path, field_name);
                            field_topology.validate(field_value, &field_path)?;
                        }
                        // Missing fields are allowed - this enables partial updates
                    }
                    Ok(())
                } else {
                    Err(SchemaError::InvalidData(format!(
                        "Topology validation failed at '{}': expected object, got {:?}",
                        path,
                        value_type_name(value)
                    )))
                }
            }

            // Reference fields accept any value (the reference JSON objects)
            TopologyNode::Reference { .. } => Ok(()),

            // Array validation
            TopologyNode::Array {
                value: element_topology,
            } => {
                if let JsonValue::Array(arr) = value {
                    for (idx, element) in arr.iter().enumerate() {
                        let element_path = format!("{}[{}]", path, idx);
                        element_topology.validate(element, &element_path)?;
                    }
                    Ok(())
                } else {
                    Err(SchemaError::InvalidData(format!(
                        "Topology validation failed at '{}': expected array, got {:?}",
                        path,
                        value_type_name(value)
                    )))
                }
            }
        }
    }

    /// Infer topology from a sample JSON value
    pub fn infer_from_value(value: &JsonValue) -> Self {
        match value {
            JsonValue::String(_) => TopologyNode::Primitive {
                value: PrimitiveValueType::String,
                classifications: None,
            },
            JsonValue::Number(_) => TopologyNode::Primitive {
                value: PrimitiveValueType::Number,
                classifications: None,
            },
            JsonValue::Bool(_) => TopologyNode::Primitive {
                value: PrimitiveValueType::Boolean,
                classifications: None,
            },
            // Null values don't provide type information - use Any to accept any type later
            JsonValue::Null => TopologyNode::Any,
            JsonValue::Array(arr) => {
                // Infer from first element, or use Any if empty
                let element_topology = arr
                    .first()
                    .map(Self::infer_from_value)
                    .unwrap_or(TopologyNode::Any);
                TopologyNode::Array {
                    value: Box::new(element_topology),
                }
            }
            JsonValue::Object(obj) => {
                let mut fields = HashMap::new();
                for (key, val) in obj {
                    fields.insert(key.clone(), Self::infer_from_value(val));
                }
                TopologyNode::Object { value: fields }
            }
        }
    }
}

/// Get a human-readable name for a JSON value type
fn value_type_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::String(_) => "string",
        JsonValue::Number(_) => "number",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Null => "null",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_primitive_validation() {
        let topology = JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::String,
            classifications: None,
        });

        assert!(topology.validate(&json!("hello")).is_ok());
        assert!(topology.validate(&json!(42)).is_err());
        assert!(topology.validate(&json!(true)).is_err());
    }

    #[test]
    fn test_object_validation() {
        let mut fields = HashMap::new();
        fields.insert(
            "name".to_string(),
            TopologyNode::Primitive {
                value: PrimitiveValueType::String,
                classifications: None,
            },
        );
        fields.insert(
            "age".to_string(),
            TopologyNode::Primitive {
                value: PrimitiveValueType::Number,
                classifications: None,
            },
        );

        let topology = JsonTopology::new(TopologyNode::Object { value: fields });

        // Valid object
        assert!(topology
            .validate(&json!({"name": "Alice", "age": 30}))
            .is_ok());

        // Partial object (missing fields allowed)
        assert!(topology.validate(&json!({"name": "Bob"})).is_ok());

        // Wrong type
        assert!(topology
            .validate(&json!({"name": "Alice", "age": "thirty"}))
            .is_err());

        // Not an object
        assert!(topology.validate(&json!("string")).is_err());
    }

    #[test]
    fn test_array_validation() {
        let topology = JsonTopology::new(TopologyNode::Array {
            value: Box::new(TopologyNode::Primitive {
                value: PrimitiveValueType::Number,
                classifications: None,
            }),
        });

        assert!(topology.validate(&json!([1, 2, 3])).is_ok());
        assert!(topology.validate(&json!([])).is_ok());
        assert!(topology.validate(&json!([1, "two", 3])).is_err());
    }

    #[test]
    fn test_nested_validation() {
        let mut user_fields = HashMap::new();
        user_fields.insert(
            "id".to_string(),
            TopologyNode::Primitive {
                value: PrimitiveValueType::Number,
                classifications: None,
            },
        );
        user_fields.insert(
            "name".to_string(),
            TopologyNode::Primitive {
                value: PrimitiveValueType::String,
                classifications: None,
            },
        );

        let mut root_fields = HashMap::new();
        root_fields.insert(
            "user".to_string(),
            TopologyNode::Object { value: user_fields },
        );
        root_fields.insert(
            "active".to_string(),
            TopologyNode::Primitive {
                value: PrimitiveValueType::Boolean,
                classifications: None,
            },
        );

        let topology = JsonTopology::new(TopologyNode::Object { value: root_fields });

        // Valid nested structure
        assert!(topology
            .validate(&json!({
                "user": {"id": 1, "name": "Alice"},
                "active": true
            }))
            .is_ok());

        // Invalid nested field
        assert!(topology
            .validate(&json!({
                "user": {"id": "not a number", "name": "Alice"},
                "active": true
            }))
            .is_err());
    }

    #[test]
    fn test_any_topology() {
        let topology = JsonTopology::any();

        assert!(topology.validate(&json!("string")).is_ok());
        assert!(topology.validate(&json!(42)).is_ok());
        assert!(topology.validate(&json!({"any": "structure"})).is_ok());
        assert!(topology.validate(&json!([1, "two", true])).is_ok());
    }

    #[test]
    fn test_infer_from_value() {
        let value = json!({
            "name": "Alice",
            "age": 30,
            "active": true,
            "tags": ["rust", "database"]
        });

        let topology = JsonTopology::infer_from_value(&value);

        // Should validate the original value
        assert!(topology.validate(&value).is_ok());

        // Should validate similar structure
        assert!(topology
            .validate(&json!({
                "name": "Bob",
                "age": 25,
                "active": false,
                "tags": ["python"]
            }))
            .is_ok());

        // Should reject different structure
        assert!(topology
            .validate(&json!({
                "name": "Charlie",
                "age": "thirty"
            }))
            .is_err());
    }

    #[test]
    fn test_nullable_primitives() {
        // All primitive types should accept null values
        let string_topology = JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::String,
            classifications: None,
        });
        assert!(string_topology.validate(&json!(null)).is_ok());
        assert!(string_topology.validate(&json!("hello")).is_ok());

        let number_topology = JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::Number,
            classifications: None,
        });
        assert!(number_topology.validate(&json!(null)).is_ok());
        assert!(number_topology.validate(&json!(42)).is_ok());

        let bool_topology = JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::Boolean,
            classifications: None,
        });
        assert!(bool_topology.validate(&json!(null)).is_ok());
        assert!(bool_topology.validate(&json!(true)).is_ok());
    }

    #[test]
    fn test_nullable_fields_in_object() {
        let mut fields = HashMap::new();
        fields.insert(
            "thread_position".to_string(),
            TopologyNode::Primitive {
                value: PrimitiveValueType::Number,
                classifications: None,
            },
        );
        fields.insert(
            "reply_to".to_string(),
            TopologyNode::Primitive {
                value: PrimitiveValueType::String,
                classifications: None,
            },
        );

        let topology = JsonTopology::new(TopologyNode::Object { value: fields });

        // Should accept null for numeric field
        assert!(topology
            .validate(&json!({"thread_position": null, "reply_to": "tweet_123"}))
            .is_ok());

        // Should accept null for string field
        assert!(topology
            .validate(&json!({"thread_position": 1, "reply_to": null}))
            .is_ok());

        // Should accept proper types
        assert!(topology
            .validate(&json!({"thread_position": 1, "reply_to": "tweet_123"}))
            .is_ok());
    }

    #[test]
    fn test_infer_from_null_uses_any() {
        // When inferring from null, should use Any type (not Null type)
        let topology = JsonTopology::infer_from_value(&json!(null));

        // Should accept any value type
        assert!(topology.validate(&json!(null)).is_ok());
        assert!(topology.validate(&json!("string")).is_ok());
        assert!(topology.validate(&json!(42)).is_ok());
        assert!(topology.validate(&json!(true)).is_ok());
        assert!(topology.validate(&json!({"key": "value"})).is_ok());
        assert!(topology.validate(&json!([1, 2, 3])).is_ok());
    }

    #[test]
    fn test_reference_validation_accepts_any_value() {
        let topology = JsonTopology::new(TopologyNode::Reference {
            schema_name: "abc123".to_string(),
        });

        assert!(topology.validate(&json!("string")).is_ok());
        assert!(topology.validate(&json!(42)).is_ok());
        assert!(topology.validate(&json!(null)).is_ok());
        assert!(topology.validate(&json!({"schema": "abc", "key": {"hash": "x"}})).is_ok());
        assert!(topology.validate(&json!([1, 2, 3])).is_ok());
    }

    #[test]
    fn test_reference_serde_roundtrip() {
        let topology = JsonTopology::new(TopologyNode::Reference {
            schema_name: "my_schema_hash".to_string(),
        });

        let json_str = serde_json::to_string(&topology).unwrap();
        let deserialized: JsonTopology = serde_json::from_str(&json_str).unwrap();
        assert_eq!(topology, deserialized);

        if let TopologyNode::Reference { schema_name } = &deserialized.root {
            assert_eq!(schema_name, "my_schema_hash");
        } else {
            panic!("Expected Reference variant");
        }
    }

    #[test]
    fn test_reference_compute_hash_deterministic() {
        let t1 = JsonTopology::new(TopologyNode::Reference {
            schema_name: "schema_a".to_string(),
        });
        let t2 = JsonTopology::new(TopologyNode::Reference {
            schema_name: "schema_a".to_string(),
        });
        let t3 = JsonTopology::new(TopologyNode::Reference {
            schema_name: "schema_b".to_string(),
        });

        assert_eq!(t1.compute_hash(), t2.compute_hash());
        assert_ne!(t1.compute_hash(), t3.compute_hash());
    }

    #[test]
    fn test_compute_hash_ignores_classifications() {
        // Same type, different classifications → same hash
        let t1 = JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::String,
            classifications: Some(vec!["word".to_string()]),
        });
        let t2 = JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::String,
            classifications: Some(vec!["word".to_string(), "name:person".to_string()]),
        });
        let t3 = JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::String,
            classifications: None,
        });

        assert_eq!(t1.compute_hash(), t2.compute_hash());
        assert_eq!(t1.compute_hash(), t3.compute_hash());
    }

    #[test]
    fn test_compute_hash_different_types_still_differ() {
        let t_string = JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::String,
            classifications: Some(vec!["word".to_string()]),
        });
        let t_number = JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::Number,
            classifications: Some(vec!["word".to_string()]),
        });

        assert_ne!(t_string.compute_hash(), t_number.compute_hash());
    }

    #[test]
    fn test_compute_hash_nested_object_ignores_classifications() {
        let mut fields1 = HashMap::new();
        fields1.insert(
            "name".to_string(),
            TopologyNode::Primitive {
                value: PrimitiveValueType::String,
                classifications: Some(vec!["word".to_string()]),
            },
        );
        fields1.insert(
            "age".to_string(),
            TopologyNode::Primitive {
                value: PrimitiveValueType::Number,
                classifications: Some(vec!["number:age".to_string()]),
            },
        );

        let mut fields2 = HashMap::new();
        fields2.insert(
            "name".to_string(),
            TopologyNode::Primitive {
                value: PrimitiveValueType::String,
                classifications: Some(vec!["word".to_string(), "name:person".to_string()]),
            },
        );
        fields2.insert(
            "age".to_string(),
            TopologyNode::Primitive {
                value: PrimitiveValueType::Number,
                classifications: None,
            },
        );

        let t1 = JsonTopology::new(TopologyNode::Object { value: fields1 });
        let t2 = JsonTopology::new(TopologyNode::Object { value: fields2 });

        assert_eq!(t1.compute_hash(), t2.compute_hash());
    }

    #[test]
    fn test_compute_hash_array_ignores_classifications() {
        let t1 = JsonTopology::new(TopologyNode::Array {
            value: Box::new(TopologyNode::Primitive {
                value: PrimitiveValueType::String,
                classifications: Some(vec!["tag".to_string()]),
            }),
        });
        let t2 = JsonTopology::new(TopologyNode::Array {
            value: Box::new(TopologyNode::Primitive {
                value: PrimitiveValueType::String,
                classifications: Some(vec!["tag".to_string(), "keyword".to_string()]),
            }),
        });

        assert_eq!(t1.compute_hash(), t2.compute_hash());
    }

    #[test]
    fn test_infer_from_object_with_null_fields() {
        // Object with null field should infer that field as Any
        let sample = json!({
            "name": "Alice",
            "optional_field": null
        });

        let topology = JsonTopology::infer_from_value(&sample);

        // Should accept the original
        assert!(topology.validate(&sample).is_ok());

        // Should accept when optional_field becomes a string
        assert!(topology
            .validate(&json!({
                "name": "Bob",
                "optional_field": "now a string"
            }))
            .is_ok());

        // Should accept when optional_field becomes a number
        assert!(topology
            .validate(&json!({
                "name": "Charlie",
                "optional_field": 42
            }))
            .is_ok());
    }
}
