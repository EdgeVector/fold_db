use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use ts_rs::TS;

use crate::schema::SchemaError;

/// Represents the topology (structure) of a JSON field
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JsonTopology {
    /// Root structure definition
    pub root: TopologyNode,
}

/// Represents a node in the JSON topology tree
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "value")]
pub enum TopologyNode {
    /// Primitive type: "string", "number", "boolean", "null"
    Primitive(PrimitiveType),
    /// Object with named fields and their topologies
    Object(HashMap<String, TopologyNode>),
    /// Array of a specific type
    Array(Box<TopologyNode>),
    /// Any type (no validation)
    Any,
}

/// Primitive JSON types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum PrimitiveType {
    String,
    Number,
    Boolean,
    Null,
}

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
}

impl TopologyNode {
    /// Validate that a JSON value conforms to this topology node
    pub fn validate(&self, value: &JsonValue, path: &str) -> Result<(), SchemaError> {
        match (self, value) {
            // Any accepts everything
            (TopologyNode::Any, _) => Ok(()),

            // Primitive validations
            (TopologyNode::Primitive(PrimitiveType::String), JsonValue::String(_)) => Ok(()),
            (TopologyNode::Primitive(PrimitiveType::Number), JsonValue::Number(_)) => Ok(()),
            (TopologyNode::Primitive(PrimitiveType::Boolean), JsonValue::Bool(_)) => Ok(()),
            (TopologyNode::Primitive(PrimitiveType::Null), JsonValue::Null) => Ok(()),
            (TopologyNode::Primitive(expected), _) => Err(SchemaError::InvalidData(format!(
                "Topology validation failed at '{}': expected {:?}, got {:?}",
                path,
                expected,
                value_type_name(value)
            ))),

            // Object validation
            (TopologyNode::Object(expected_fields), JsonValue::Object(obj)) => {
                for (field_name, field_topology) in expected_fields {
                    if let Some(field_value) = obj.get(field_name) {
                        let field_path = format!("{}.{}", path, field_name);
                        field_topology.validate(field_value, &field_path)?;
                    }
                    // Missing fields are allowed - this enables partial updates
                }
                Ok(())
            }
            (TopologyNode::Object(_), _) => Err(SchemaError::InvalidData(format!(
                "Topology validation failed at '{}': expected object, got {:?}",
                path,
                value_type_name(value)
            ))),

            // Array validation
            (TopologyNode::Array(element_topology), JsonValue::Array(arr)) => {
                for (idx, element) in arr.iter().enumerate() {
                    let element_path = format!("{}[{}]", path, idx);
                    element_topology.validate(element, &element_path)?;
                }
                Ok(())
            }
            (TopologyNode::Array(_), _) => Err(SchemaError::InvalidData(format!(
                "Topology validation failed at '{}': expected array, got {:?}",
                path,
                value_type_name(value)
            ))),
        }
    }

    /// Infer topology from a sample JSON value
    pub fn infer_from_value(value: &JsonValue) -> Self {
        match value {
            JsonValue::String(_) => TopologyNode::Primitive(PrimitiveType::String),
            JsonValue::Number(_) => TopologyNode::Primitive(PrimitiveType::Number),
            JsonValue::Bool(_) => TopologyNode::Primitive(PrimitiveType::Boolean),
            JsonValue::Null => TopologyNode::Primitive(PrimitiveType::Null),
            JsonValue::Array(arr) => {
                // Infer from first element, or use Any if empty
                let element_topology = arr
                    .first()
                    .map(Self::infer_from_value)
                    .unwrap_or(TopologyNode::Any);
                TopologyNode::Array(Box::new(element_topology))
            }
            JsonValue::Object(obj) => {
                let mut fields = HashMap::new();
                for (key, val) in obj {
                    fields.insert(key.clone(), Self::infer_from_value(val));
                }
                TopologyNode::Object(fields)
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
        let topology = JsonTopology::new(TopologyNode::Primitive(PrimitiveType::String));
        
        assert!(topology.validate(&json!("hello")).is_ok());
        assert!(topology.validate(&json!(42)).is_err());
        assert!(topology.validate(&json!(true)).is_err());
    }

    #[test]
    fn test_object_validation() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), TopologyNode::Primitive(PrimitiveType::String));
        fields.insert("age".to_string(), TopologyNode::Primitive(PrimitiveType::Number));
        
        let topology = JsonTopology::new(TopologyNode::Object(fields));
        
        // Valid object
        assert!(topology.validate(&json!({"name": "Alice", "age": 30})).is_ok());
        
        // Partial object (missing fields allowed)
        assert!(topology.validate(&json!({"name": "Bob"})).is_ok());
        
        // Wrong type
        assert!(topology.validate(&json!({"name": "Alice", "age": "thirty"})).is_err());
        
        // Not an object
        assert!(topology.validate(&json!("string")).is_err());
    }

    #[test]
    fn test_array_validation() {
        let topology = JsonTopology::new(TopologyNode::Array(Box::new(
            TopologyNode::Primitive(PrimitiveType::Number)
        )));
        
        assert!(topology.validate(&json!([1, 2, 3])).is_ok());
        assert!(topology.validate(&json!([])).is_ok());
        assert!(topology.validate(&json!([1, "two", 3])).is_err());
    }

    #[test]
    fn test_nested_validation() {
        let mut user_fields = HashMap::new();
        user_fields.insert("id".to_string(), TopologyNode::Primitive(PrimitiveType::Number));
        user_fields.insert("name".to_string(), TopologyNode::Primitive(PrimitiveType::String));
        
        let mut root_fields = HashMap::new();
        root_fields.insert("user".to_string(), TopologyNode::Object(user_fields));
        root_fields.insert("active".to_string(), TopologyNode::Primitive(PrimitiveType::Boolean));
        
        let topology = JsonTopology::new(TopologyNode::Object(root_fields));
        
        // Valid nested structure
        assert!(topology.validate(&json!({
            "user": {"id": 1, "name": "Alice"},
            "active": true
        })).is_ok());
        
        // Invalid nested field
        assert!(topology.validate(&json!({
            "user": {"id": "not a number", "name": "Alice"},
            "active": true
        })).is_err());
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
        assert!(topology.validate(&json!({
            "name": "Bob",
            "age": 25,
            "active": false,
            "tags": ["python"]
        })).is_ok());
        
        // Should reject different structure
        assert!(topology.validate(&json!({
            "name": "Charlie",
            "age": "thirty"
        })).is_err());
    }
}

