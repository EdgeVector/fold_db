use serde::{Deserialize, Serialize};
use serde_json::{Number, Value as JsonValue};
use std::collections::HashMap;

const DEFAULT_FLOAT_FALLBACK: f64 = 0.0;
const DEFAULT_INTEGER_FALLBACK: i64 = 0;

/// Native representation of a field value flowing through the transform system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldValue {
    String(String),
    Integer(i64),
    Number(f64),
    Boolean(bool),
    Array(Vec<FieldValue>),
    Object(HashMap<String, FieldValue>),
    Null,
}

impl FieldValue {
    /// Return the [`FieldType`] that best describes this value.
    #[must_use]
    pub fn field_type(&self) -> FieldType {
        match self {
            FieldValue::String(_) => FieldType::String,
            FieldValue::Integer(_) => FieldType::Integer,
            FieldValue::Number(_) => FieldType::Number,
            FieldValue::Boolean(_) => FieldType::Boolean,
            FieldValue::Array(values) => FieldType::Array {
                element_type: Box::new(Self::infer_array_element_type(values)),
            },
            FieldValue::Object(entries) => FieldType::Object {
                fields: entries
                    .iter()
                    .map(|(key, value)| (key.clone(), value.field_type()))
                    .collect(),
            },
            FieldValue::Null => FieldType::Null,
        }
    }

    /// Convert the native value into a [`serde_json::Value`] for boundary operations.
    #[must_use]
    pub fn to_json_value(&self) -> JsonValue {
        match self {
            FieldValue::String(value) => JsonValue::String(value.clone()),
            FieldValue::Integer(value) => JsonValue::Number(Number::from(*value)),
            FieldValue::Number(value) => JsonValue::Number(Self::safe_number_from_f64(*value)),
            FieldValue::Boolean(value) => JsonValue::Bool(*value),
            FieldValue::Array(values) => {
                JsonValue::Array(values.iter().map(FieldValue::to_json_value).collect())
            }
            FieldValue::Object(entries) => JsonValue::Object(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_json_value()))
                    .collect(),
            ),
            FieldValue::Null => JsonValue::Null,
        }
    }

    /// Construct a native value from a [`serde_json::Value`].
    #[must_use]
    pub fn from_json_value(value: JsonValue) -> Self {
        match value {
            JsonValue::String(s) => FieldValue::String(s),
            JsonValue::Number(number) => {
                if let Some(int_value) = number.as_i64() {
                    FieldValue::Integer(int_value)
                } else if let Some(float_value) = number.as_f64() {
                    FieldValue::Number(float_value)
                } else {
                    FieldValue::Number(DEFAULT_FLOAT_FALLBACK)
                }
            }
            JsonValue::Bool(flag) => FieldValue::Boolean(flag),
            JsonValue::Array(values) => FieldValue::Array(
                values
                    .into_iter()
                    .map(FieldValue::from_json_value)
                    .collect(),
            ),
            JsonValue::Object(map) => FieldValue::Object(
                map.into_iter()
                    .map(|(key, value)| (key, FieldValue::from_json_value(value)))
                    .collect(),
            ),
            JsonValue::Null => FieldValue::Null,
        }
    }

    fn safe_number_from_f64(value: f64) -> Number {
        Number::from_f64(value).unwrap_or_else(|| Number::from(DEFAULT_INTEGER_FALLBACK))
    }

    fn infer_array_element_type(values: &[FieldValue]) -> FieldType {
        let mut inferred: Option<FieldType> = None;

        for value in values {
            if matches!(value, FieldValue::Null) {
                continue;
            }

            let current_type = value.field_type();
            match &inferred {
                Some(existing) if existing == &current_type => {}
                Some(_) => return FieldType::Null,
                None => inferred = Some(current_type),
            }
        }

        inferred.unwrap_or(FieldType::Null)
    }
}

/// Declarative type information for schema fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldType {
    String,
    Number,
    Integer,
    Boolean,
    Null,
    Array {
        #[serde(rename = "element_type")]
        element_type: Box<FieldType>,
    },
    Object {
        fields: HashMap<String, FieldType>,
    },
}

impl FieldType {
    /// Determine whether the provided [`FieldValue`] satisfies this type definition.
    #[must_use]
    pub fn matches(&self, value: &FieldValue) -> bool {
        if matches!(value, FieldValue::Null) {
            return true;
        }

        match self {
            FieldType::String => matches!(value, FieldValue::String(_)),
            FieldType::Number => matches!(value, FieldValue::Number(_)),
            FieldType::Integer => matches!(value, FieldValue::Integer(_)),
            FieldType::Boolean => matches!(value, FieldValue::Boolean(_)),
            FieldType::Null => false,
            FieldType::Array { element_type } => match value {
                FieldValue::Array(values) => values.iter().all(|item| element_type.matches(item)),
                _ => false,
            },
            FieldType::Object { fields } => match value {
                FieldValue::Object(entries) => fields.iter().all(|(field_name, field_type)| {
                    entries
                        .get(field_name)
                        .map(|field_value| field_type.matches(field_value))
                        .unwrap_or(false)
                }),
                _ => false,
            },
        }
    }
}
