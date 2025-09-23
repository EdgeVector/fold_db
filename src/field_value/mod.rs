//! Type-safe wrappers for field values exchanged at API boundaries.
//!
//! The [`TypedFieldValue`] type provides ergonomic accessors that validate
//! the underlying [`serde_json::Value`] before exposing it as a strongly-typed
//! value. This enables callers to surface clear error messages whenever a
//! payload contains an unexpected type while preserving the flexibility of the
//! existing JSON-based transport between services.

use crate::schema::types::SchemaError;
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;

/// Supported field value classifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldType {
    Null,
    Boolean,
    String,
    Integer,
    UnsignedInteger,
    Float,
    Object,
    Array,
}

impl FieldType {
    /// Determine the [`FieldType`] that matches the provided JSON value.
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Null => FieldType::Null,
            Value::Bool(_) => FieldType::Boolean,
            Value::String(_) => FieldType::String,
            Value::Number(number) => {
                if number.is_i64() {
                    FieldType::Integer
                } else if number.is_u64() {
                    FieldType::UnsignedInteger
                } else {
                    FieldType::Float
                }
            }
            Value::Object(_) => FieldType::Object,
            Value::Array(_) => FieldType::Array,
        }
    }

    /// Human readable name used in error messages.
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::Null => "null",
            FieldType::Boolean => "boolean",
            FieldType::String => "string",
            FieldType::Integer => "integer",
            FieldType::UnsignedInteger => "unsigned integer",
            FieldType::Float => "float",
            FieldType::Object => "object",
            FieldType::Array => "array",
        }
    }
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Wrapper that exposes type-safe accessors over [`serde_json::Value`].
#[derive(Debug, Clone, PartialEq)]
pub struct TypedFieldValue {
    value: Value,
}

impl TypedFieldValue {
    /// Creates a new wrapper around the provided JSON value.
    pub fn new(value: Value) -> Self {
        Self { value }
    }

    /// Returns the detected [`FieldType`] for the wrapped value.
    pub fn field_type(&self) -> FieldType {
        FieldType::from_value(&self.value)
    }

    /// Returns a reference to the raw JSON value.
    pub fn as_json(&self) -> &Value {
        &self.value
    }

    /// Consumes the wrapper and returns the raw JSON value.
    pub fn into_inner(self) -> Value {
        self.value
    }

    /// Validates that the wrapped value matches the expected [`FieldType`].
    pub fn ensure_type(&self, expected: FieldType) -> Result<(), SchemaError> {
        self.ensure_type_with_context(expected, None)
    }

    /// Validates the wrapped value and includes the provided context in errors.
    pub fn ensure_type_with_context(
        &self,
        expected: FieldType,
        context: Option<&str>,
    ) -> Result<(), SchemaError> {
        let actual = self.field_type();
        if actual == expected {
            Ok(())
        } else {
            Err(Self::type_mismatch_error(
                expected.as_str(),
                actual,
                context,
            ))
        }
    }

    /// Returns the wrapped string if present.
    pub fn as_string(&self) -> Result<&str, SchemaError> {
        match &self.value {
            Value::String(value) => Ok(value.as_str()),
            other => Err(Self::type_mismatch_error(
                FieldType::String.as_str(),
                FieldType::from_value(other),
                None,
            )),
        }
    }

    /// Returns the wrapped boolean if present.
    pub fn as_bool(&self) -> Result<bool, SchemaError> {
        match &self.value {
            Value::Bool(value) => Ok(*value),
            other => Err(Self::type_mismatch_error(
                FieldType::Boolean.as_str(),
                FieldType::from_value(other),
                None,
            )),
        }
    }

    /// Returns the wrapped [`serde_json::Number`] if present.
    pub fn as_number(&self) -> Result<&Number, SchemaError> {
        match &self.value {
            Value::Number(number) => Ok(number),
            other => Err(Self::type_mismatch_error(
                "number",
                FieldType::from_value(other),
                None,
            )),
        }
    }

    /// Returns the wrapped number as `i64` when available.
    pub fn as_i64(&self) -> Result<i64, SchemaError> {
        let number = self.as_number()?;
        let actual_type = self.field_type();
        number.as_i64().ok_or_else(|| {
            Self::type_mismatch_error(FieldType::Integer.as_str(), actual_type, None)
        })
    }

    /// Returns the wrapped number as `u64` when available.
    pub fn as_u64(&self) -> Result<u64, SchemaError> {
        let number = self.as_number()?;
        let actual_type = self.field_type();
        number.as_u64().ok_or_else(|| {
            Self::type_mismatch_error(FieldType::UnsignedInteger.as_str(), actual_type, None)
        })
    }

    /// Returns the wrapped number as `f64` when available.
    pub fn as_f64(&self) -> Result<f64, SchemaError> {
        let number = self.as_number()?;
        let actual_type = self.field_type();
        number
            .as_f64()
            .ok_or_else(|| Self::type_mismatch_error(FieldType::Float.as_str(), actual_type, None))
    }

    /// Returns the wrapped JSON object if present.
    pub fn as_object(&self) -> Result<&Map<String, Value>, SchemaError> {
        match &self.value {
            Value::Object(map) => Ok(map),
            other => Err(Self::type_mismatch_error(
                FieldType::Object.as_str(),
                FieldType::from_value(other),
                None,
            )),
        }
    }

    /// Returns the wrapped JSON array if present.
    pub fn as_array(&self) -> Result<&Vec<Value>, SchemaError> {
        match &self.value {
            Value::Array(values) => Ok(values),
            other => Err(Self::type_mismatch_error(
                FieldType::Array.as_str(),
                FieldType::from_value(other),
                None,
            )),
        }
    }

    /// Converts the wrapped JSON array into a vector of [`TypedFieldValue`].
    pub fn into_typed_array(self) -> Result<Vec<TypedFieldValue>, SchemaError> {
        match self.value {
            Value::Array(values) => Ok(values.into_iter().map(TypedFieldValue::from).collect()),
            other => Err(Self::type_mismatch_error(
                FieldType::Array.as_str(),
                FieldType::from_value(&other),
                None,
            )),
        }
    }

    /// Converts the wrapped JSON object into a typed map.
    pub fn into_typed_object(self) -> Result<BTreeMap<String, TypedFieldValue>, SchemaError> {
        match self.value {
            Value::Object(values) => Ok(values
                .into_iter()
                .map(|(key, value)| (key, TypedFieldValue::from(value)))
                .collect::<BTreeMap<_, _>>()),
            other => Err(Self::type_mismatch_error(
                FieldType::Object.as_str(),
                FieldType::from_value(&other),
                None,
            )),
        }
    }

    fn type_mismatch_error(
        expected_label: &str,
        actual: FieldType,
        context: Option<&str>,
    ) -> SchemaError {
        let message = match context {
            Some(field_name) => format!(
                "Field '{}' expected {} but received {}",
                field_name, expected_label, actual
            ),
            None => format!("Expected {} but received {}", expected_label, actual),
        };
        SchemaError::InvalidData(message)
    }
}

impl From<Value> for TypedFieldValue {
    fn from(value: Value) -> Self {
        Self::new(value)
    }
}

impl From<&Value> for TypedFieldValue {
    fn from(value: &Value) -> Self {
        Self::new(value.clone())
    }
}

impl From<String> for TypedFieldValue {
    fn from(value: String) -> Self {
        Self::new(Value::String(value))
    }
}

impl From<&str> for TypedFieldValue {
    fn from(value: &str) -> Self {
        Self::new(Value::String(value.to_string()))
    }
}

impl From<bool> for TypedFieldValue {
    fn from(value: bool) -> Self {
        Self::new(Value::Bool(value))
    }
}

impl From<i64> for TypedFieldValue {
    fn from(value: i64) -> Self {
        Self::new(Value::Number(Number::from(value)))
    }
}

impl From<u64> for TypedFieldValue {
    fn from(value: u64) -> Self {
        Self::new(Value::Number(Number::from(value)))
    }
}

impl From<f64> for TypedFieldValue {
    fn from(value: f64) -> Self {
        Self::new(Value::Number(Number::from_f64(value).unwrap()))
    }
}

impl From<Map<String, Value>> for TypedFieldValue {
    fn from(map: Map<String, Value>) -> Self {
        Self::new(Value::Object(map))
    }
}

impl From<Vec<Value>> for TypedFieldValue {
    fn from(values: Vec<Value>) -> Self {
        Self::new(Value::Array(values))
    }
}

impl AsRef<Value> for TypedFieldValue {
    fn as_ref(&self) -> &Value {
        self.as_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_field_types_correctly() {
        assert_eq!(FieldType::from_value(&Value::Null), FieldType::Null);
        assert_eq!(
            FieldType::from_value(&Value::Bool(true)),
            FieldType::Boolean
        );
        assert_eq!(
            FieldType::from_value(&Value::String("abc".into())),
            FieldType::String
        );
        assert_eq!(
            FieldType::from_value(&Value::Number(Number::from(123))),
            FieldType::Integer
        );
        assert_eq!(
            FieldType::from_value(&Value::Number(Number::from(123_u64))),
            FieldType::Integer
        );
        assert_eq!(
            FieldType::from_value(&Value::Number(Number::from(u64::MAX))),
            FieldType::UnsignedInteger
        );
        assert_eq!(
            FieldType::from_value(&Value::Number(Number::from_f64(1.5).unwrap())),
            FieldType::Float
        );
        assert_eq!(
            FieldType::from_value(&Value::Array(vec![])),
            FieldType::Array
        );
        assert_eq!(
            FieldType::from_value(&Value::Object(Map::new())),
            FieldType::Object
        );
    }

    #[test]
    fn accessor_methods_return_expected_types() {
        let string_value = TypedFieldValue::from("hello");
        assert_eq!(string_value.as_string().unwrap(), "hello");

        let bool_value = TypedFieldValue::from(true);
        assert!(bool_value.as_bool().unwrap());

        let int_value = TypedFieldValue::from(42_i64);
        assert_eq!(int_value.as_number().unwrap().as_i64().unwrap(), 42);
        assert_eq!(int_value.as_i64().unwrap(), 42);

        let float_value = TypedFieldValue::from(3.14_f64);
        assert_eq!(float_value.as_f64().unwrap(), 3.14_f64);

        let array_value = TypedFieldValue::from(vec![json!(1), json!(2)]);
        assert_eq!(array_value.as_array().unwrap().len(), 2);

        let mut object = Map::new();
        object.insert("key".into(), json!("value"));
        let object_value = TypedFieldValue::from(object);
        assert!(object_value.as_object().unwrap().contains_key("key"));
    }

    #[test]
    fn accessor_methods_fail_with_clear_errors() {
        let value = TypedFieldValue::from("text");
        let err = value.as_bool().unwrap_err();
        assert!(matches!(err, SchemaError::InvalidData(message) if message.contains("boolean")));

        let number_value = TypedFieldValue::from(10_i64);
        let err = number_value.as_array().unwrap_err();
        assert!(matches!(err, SchemaError::InvalidData(message) if message.contains("array")));
    }

    #[test]
    fn ensure_type_includes_context() {
        let value = TypedFieldValue::from(json!(42));
        let error = value
            .ensure_type_with_context(FieldType::String, Some("user.age"))
            .unwrap_err();

        match error {
            SchemaError::InvalidData(message) => {
                assert!(message.contains("user.age"));
                assert!(message.contains("string"));
            }
            _ => panic!("Unexpected error variant"),
        }
    }

    #[test]
    fn into_typed_collections_convert_recursively() {
        let array_value = TypedFieldValue::from(json!(["a", 1]));
        let typed_array = array_value.into_typed_array().unwrap();
        assert_eq!(typed_array[0].as_string().unwrap(), "a");
        assert_eq!(typed_array[1].as_i64().unwrap(), 1);

        let map_value = TypedFieldValue::from(json!({"key": true}));
        let typed_map = map_value.into_typed_object().unwrap();
        assert!(typed_map.get("key").unwrap().as_bool().unwrap());
    }
}
