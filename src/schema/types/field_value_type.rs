use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Strongly typed value types for schema fields.
///
/// Types are declared on canonical fields in the schema service and
/// enforced at mutation time. Every field in every schema has a concrete
/// type — `Any` is reserved for backward compatibility only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FieldValueType {
    // ── Primitives ──────────────────────────────────────
    String,
    Integer,
    Float,
    Number, // accepts either integer or float
    Boolean,
    Null,

    // ── Compound ────────────────────────────────────────
    /// Homogeneous typed array: `Array(String)`, `Array(Integer)`
    Array(Box<FieldValueType>),
    /// Typed key-value map (JSON keys are always strings):
    /// `Map(String, Number)` means `{"a": 1, "b": 2}`
    Map(Box<FieldValueType>),
    /// Typed struct with named fields:
    /// `Object({"name": String, "age": Integer})`
    Object(BTreeMap<String, FieldValueType>),

    // ── References ──────────────────────────────────────
    /// Reference to another schema. The string is the schema name.
    /// Enforced: the field value must be a reference object or array
    /// of reference objects pointing to this specific schema.
    SchemaRef(String),

    // ── Union ───────────────────────────────────────────
    /// Union type: matches if the value satisfies any variant.
    /// Common use: `OneOf([String, Null])` for nullable strings.
    OneOf(Vec<FieldValueType>),

    // ── Escape hatch ────────────────────────────────────
    /// Accepts any JSON value. Used for backward compatibility
    /// with existing schemas that have no type declarations.
    Any,
}

impl FieldValueType {
    /// Validate a JSON value against this type. Returns Ok(()) if valid,
    /// Err with a human-readable message if not.
    pub fn validate(&self, value: &serde_json::Value) -> Result<(), String> {
        match self {
            FieldValueType::Any => Ok(()),
            FieldValueType::Null => {
                if value.is_null() {
                    Ok(())
                } else {
                    Err(format!("expected Null, got {}", json_type_name(value)))
                }
            }
            FieldValueType::String => {
                if value.is_string() {
                    Ok(())
                } else {
                    Err(format!("expected String, got {}", json_type_name(value)))
                }
            }
            FieldValueType::Boolean => {
                if value.is_boolean() {
                    Ok(())
                } else {
                    Err(format!("expected Boolean, got {}", json_type_name(value)))
                }
            }
            FieldValueType::Integer => {
                if value.is_number() && value.as_i64().is_some() {
                    Ok(())
                } else {
                    Err(format!("expected Integer, got {}", json_type_name(value)))
                }
            }
            FieldValueType::Float => {
                if value.is_number() && value.as_f64().is_some() {
                    Ok(())
                } else {
                    Err(format!("expected Float, got {}", json_type_name(value)))
                }
            }
            FieldValueType::Number => {
                if value.is_number() {
                    Ok(())
                } else {
                    Err(format!("expected Number, got {}", json_type_name(value)))
                }
            }
            FieldValueType::Array(element_type) => {
                let arr = value
                    .as_array()
                    .ok_or_else(|| format!("expected Array, got {}", json_type_name(value)))?;
                for (i, elem) in arr.iter().enumerate() {
                    element_type
                        .validate(elem)
                        .map_err(|e| format!("Array[{}]: {}", i, e))?;
                }
                Ok(())
            }
            FieldValueType::Map(value_type) => {
                let obj = value
                    .as_object()
                    .ok_or_else(|| format!("expected Map, got {}", json_type_name(value)))?;
                for (k, v) in obj {
                    value_type
                        .validate(v)
                        .map_err(|e| format!("Map[\"{}\"]: {}", k, e))?;
                }
                Ok(())
            }
            FieldValueType::Object(field_types) => {
                let obj = value
                    .as_object()
                    .ok_or_else(|| format!("expected Object, got {}", json_type_name(value)))?;
                for (field_name, field_type) in field_types {
                    let field_value = obj.get(field_name).unwrap_or(&serde_json::Value::Null);
                    field_type
                        .validate(field_value)
                        .map_err(|e| format!(".{}: {}", field_name, e))?;
                }
                Ok(())
            }
            FieldValueType::SchemaRef(schema_name) => {
                // A schema reference must be either:
                // - A single ref object: {"schema": "X", "key": {...}}
                // - An array of ref objects
                if let Some(arr) = value.as_array() {
                    for (i, elem) in arr.iter().enumerate() {
                        validate_ref_object(elem, schema_name)
                            .map_err(|e| format!("SchemaRef[{}]: {}", i, e))?;
                    }
                    Ok(())
                } else if value.is_object() {
                    validate_ref_object(value, schema_name)
                } else {
                    Err(format!(
                        "expected SchemaRef({}), got {}",
                        schema_name,
                        json_type_name(value)
                    ))
                }
            }
            FieldValueType::OneOf(variants) => {
                for variant in variants {
                    if variant.validate(value).is_ok() {
                        return Ok(());
                    }
                }
                let type_names: Vec<String> = variants.iter().map(|v| format!("{}", v)).collect();
                Err(format!(
                    "value does not match any variant of OneOf({}), got {}",
                    type_names.join(" | "),
                    json_type_name(value)
                ))
            }
        }
    }

    /// Infer a FieldValueType from a sample JSON value.
    /// Used as fallback when the AI doesn't provide types.
    pub fn infer(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => FieldValueType::Null,
            serde_json::Value::Bool(_) => FieldValueType::Boolean,
            serde_json::Value::Number(n) => {
                if n.is_i64() {
                    FieldValueType::Integer
                } else {
                    FieldValueType::Float
                }
            }
            serde_json::Value::String(_) => FieldValueType::String,
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    FieldValueType::Array(Box::new(FieldValueType::Any))
                } else {
                    // Infer from first element
                    FieldValueType::Array(Box::new(Self::infer(&arr[0])))
                }
            }
            serde_json::Value::Object(obj) => {
                let mut fields = BTreeMap::new();
                for (k, v) in obj {
                    fields.insert(k.clone(), Self::infer(v));
                }
                FieldValueType::Object(fields)
            }
        }
    }
}

fn validate_ref_object(value: &serde_json::Value, expected_schema: &str) -> Result<(), String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "expected reference object".to_string())?;
    let schema = obj
        .get("schema")
        .and_then(|s| s.as_str())
        .ok_or_else(|| "reference object missing 'schema' field".to_string())?;
    if schema != expected_schema {
        return Err(format!(
            "expected reference to schema '{}', got '{}'",
            expected_schema, schema
        ));
    }
    if !obj.contains_key("key") {
        return Err("reference object missing 'key' field".to_string());
    }
    Ok(())
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "Null",
        serde_json::Value::Bool(_) => "Boolean",
        serde_json::Value::Number(_) => "Number",
        serde_json::Value::String(_) => "String",
        serde_json::Value::Array(_) => "Array",
        serde_json::Value::Object(_) => "Object",
    }
}

impl std::fmt::Display for FieldValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldValueType::String => write!(f, "String"),
            FieldValueType::Integer => write!(f, "Integer"),
            FieldValueType::Float => write!(f, "Float"),
            FieldValueType::Number => write!(f, "Number"),
            FieldValueType::Boolean => write!(f, "Boolean"),
            FieldValueType::Null => write!(f, "Null"),
            FieldValueType::Array(t) => write!(f, "Array<{}>", t),
            FieldValueType::Map(v) => write!(f, "Map<String, {}>", v),
            FieldValueType::Object(fields) => {
                write!(f, "{{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            FieldValueType::SchemaRef(s) => write!(f, "Ref<{}>", s),
            FieldValueType::OneOf(variants) => {
                for (i, v) in variants.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", v)?;
                }
                Ok(())
            }
            FieldValueType::Any => write!(f, "Any"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_primitive_validation() {
        assert!(FieldValueType::String.validate(&json!("hello")).is_ok());
        assert!(FieldValueType::String.validate(&json!(42)).is_err());

        assert!(FieldValueType::Integer.validate(&json!(42)).is_ok());
        assert!(FieldValueType::Integer.validate(&json!(2.5)).is_err());

        assert!(FieldValueType::Float.validate(&json!(2.5)).is_ok());
        assert!(FieldValueType::Float.validate(&json!(42)).is_ok()); // i64 has f64 repr

        assert!(FieldValueType::Number.validate(&json!(42)).is_ok());
        assert!(FieldValueType::Number.validate(&json!(2.5)).is_ok());
        assert!(FieldValueType::Number.validate(&json!("nope")).is_err());

        assert!(FieldValueType::Boolean.validate(&json!(true)).is_ok());
        assert!(FieldValueType::Boolean.validate(&json!(1)).is_err());

        assert!(FieldValueType::Null.validate(&json!(null)).is_ok());
        assert!(FieldValueType::Null.validate(&json!("")).is_err());
    }

    #[test]
    fn test_array_validation() {
        let t = FieldValueType::Array(Box::new(FieldValueType::String));
        assert!(t.validate(&json!(["a", "b", "c"])).is_ok());
        assert!(t.validate(&json!([])).is_ok());
        assert!(t.validate(&json!([1, 2])).is_err());
        assert!(t.validate(&json!("not array")).is_err());

        // Mixed array fails
        assert!(t.validate(&json!(["a", 1])).is_err());
    }

    #[test]
    fn test_map_validation() {
        let t = FieldValueType::Map(Box::new(FieldValueType::Number));
        assert!(t.validate(&json!({"a": 1, "b": 2.5})).is_ok());
        assert!(t.validate(&json!({})).is_ok());
        assert!(t.validate(&json!({"a": "string"})).is_err());
    }

    #[test]
    fn test_object_validation() {
        let mut fields = BTreeMap::new();
        fields.insert("name".to_string(), FieldValueType::String);
        fields.insert("age".to_string(), FieldValueType::Integer);
        let t = FieldValueType::Object(fields);

        assert!(t.validate(&json!({"name": "Tom", "age": 30})).is_ok());
        // Extra fields are allowed (permissive)
        assert!(t
            .validate(&json!({"name": "Tom", "age": 30, "extra": true}))
            .is_ok());
        // Wrong type fails
        assert!(t
            .validate(&json!({"name": "Tom", "age": "thirty"}))
            .is_err());
        // Missing field gets Null → fails if not nullable
        assert!(t.validate(&json!({"name": "Tom"})).is_err());
    }

    #[test]
    fn test_nullable_via_oneof() {
        let t = FieldValueType::OneOf(vec![FieldValueType::String, FieldValueType::Null]);
        assert!(t.validate(&json!("hello")).is_ok());
        assert!(t.validate(&json!(null)).is_ok());
        assert!(t.validate(&json!(42)).is_err());
    }

    #[test]
    fn test_schema_ref_validation() {
        let t = FieldValueType::SchemaRef("PostSchema".to_string());

        // Single ref object
        assert!(t
            .validate(&json!({"schema": "PostSchema", "key": {"hash": "abc"}}))
            .is_ok());

        // Array of ref objects
        assert!(t
            .validate(&json!([
                {"schema": "PostSchema", "key": {"hash": "a"}},
                {"schema": "PostSchema", "key": {"hash": "b"}}
            ]))
            .is_ok());

        // Wrong schema name
        assert!(t
            .validate(&json!({"schema": "WrongSchema", "key": {"hash": "abc"}}))
            .is_err());

        // Missing key
        assert!(t.validate(&json!({"schema": "PostSchema"})).is_err());

        // Not an object
        assert!(t.validate(&json!("PostSchema")).is_err());
    }

    #[test]
    fn test_any_accepts_everything() {
        assert!(FieldValueType::Any.validate(&json!(null)).is_ok());
        assert!(FieldValueType::Any.validate(&json!("str")).is_ok());
        assert!(FieldValueType::Any.validate(&json!(42)).is_ok());
        assert!(FieldValueType::Any.validate(&json!([1, 2, 3])).is_ok());
        assert!(FieldValueType::Any.validate(&json!({"a": 1})).is_ok());
    }

    #[test]
    fn test_nested_validation_error_path() {
        let t = FieldValueType::Array(Box::new(FieldValueType::Object({
            let mut f = BTreeMap::new();
            f.insert("name".to_string(), FieldValueType::String);
            f
        })));

        let err = t
            .validate(&json!([{"name": "ok"}, {"name": 42}]))
            .unwrap_err();
        assert!(
            err.contains("Array[1]"),
            "Error should include index: {}",
            err
        );
        assert!(err.contains(".name"), "Error should include field: {}", err);
    }

    #[test]
    fn test_infer_from_value() {
        assert_eq!(
            FieldValueType::infer(&json!("hello")),
            FieldValueType::String
        );
        assert_eq!(FieldValueType::infer(&json!(42)), FieldValueType::Integer);
        assert_eq!(FieldValueType::infer(&json!(2.5)), FieldValueType::Float);
        assert_eq!(FieldValueType::infer(&json!(true)), FieldValueType::Boolean);
        assert_eq!(FieldValueType::infer(&json!(null)), FieldValueType::Null);
        assert_eq!(
            FieldValueType::infer(&json!(["a", "b"])),
            FieldValueType::Array(Box::new(FieldValueType::String))
        );
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", FieldValueType::String), "String");
        assert_eq!(
            format!(
                "{}",
                FieldValueType::Array(Box::new(FieldValueType::Integer))
            ),
            "Array<Integer>"
        );
        assert_eq!(
            format!("{}", FieldValueType::SchemaRef("Post".into())),
            "Ref<Post>"
        );
        let nullable = FieldValueType::OneOf(vec![FieldValueType::String, FieldValueType::Null]);
        assert_eq!(format!("{}", nullable), "String | Null");
    }

    #[test]
    fn test_serde_roundtrip() {
        let types = vec![
            FieldValueType::String,
            FieldValueType::Array(Box::new(FieldValueType::Integer)),
            FieldValueType::Map(Box::new(FieldValueType::Number)),
            FieldValueType::SchemaRef("PostSchema".into()),
            FieldValueType::OneOf(vec![FieldValueType::String, FieldValueType::Null]),
            FieldValueType::Object({
                let mut f = BTreeMap::new();
                f.insert("x".into(), FieldValueType::Float);
                f
            }),
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let back: FieldValueType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, back, "Roundtrip failed for: {}", json);
        }
    }
}
