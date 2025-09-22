use std::{collections::HashMap, sync::Arc};

use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::{
    db_operations::DbOperations,
    schema::SchemaError,
    transform::native::{FieldDefinition, FieldType, FieldValue},
};

const STORAGE_PREFIX: &str = "native";

/// Provides schema metadata required for native persistence validation.
pub trait NativeSchemaProvider: Send + Sync {
    /// Lookup schema description by name.
    fn schema_for(&self, schema_name: &str) -> Result<SchemaDescription, PersistenceError>;
}

/// Describes the typed structure of a native schema.
#[derive(Clone, Debug)]
pub struct SchemaDescription {
    /// Name of the schema.
    pub name: String,
    /// Key configuration used to derive unique record keys.
    pub key: KeyConfig,
    /// Field definitions keyed by field name.
    pub fields: HashMap<String, FieldDefinition>,
}

/// Supported key configurations for native persistence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyConfig {
    /// Single-field key.
    Single { key_field: String },
    /// Composite hash/range key pair used for ordered storage.
    Range {
        hash_field: String,
        range_field: String,
    },
    /// Hash-range configuration used by declarative schemas.
    HashRange {
        hash_field: String,
        range_field: String,
    },
}

/// Identifier for a persisted native record.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeRecordKey {
    /// Single-field key value.
    Single(String),
    /// Composite hash/range key values.
    Composite { hash: String, range: String },
}

impl NativeRecordKey {
    /// Create a new single-field key.
    #[must_use]
    pub fn single(value: impl Into<String>) -> Self {
        Self::Single(value.into())
    }

    /// Create a new composite hash/range key.
    #[must_use]
    pub fn composite(hash: impl Into<String>, range: impl Into<String>) -> Self {
        Self::Composite {
            hash: hash.into(),
            range: range.into(),
        }
    }

    /// Serialize the key into a stable string representation.
    #[must_use]
    pub fn serialize(&self) -> String {
        match self {
            Self::Single(value) => format!("single:{}", encode_segment(value)),
            Self::Composite { hash, range } => format!(
                "composite:{}:{}",
                encode_segment(hash),
                encode_segment(range)
            ),
        }
    }

    /// Reconstruct a [`NativeRecordKey`] from its serialized representation.
    pub fn from_serialized(serialized: &str) -> Result<Self, PersistenceError> {
        let mut parts = serialized.split(':');
        match parts.next() {
            Some("single") => {
                let encoded = parts.next().ok_or_else(|| {
                    PersistenceError::KeySerialization("missing single key payload".into())
                })?;
                let value = decode_segment(encoded)?;
                Ok(Self::Single(value))
            }
            Some("composite") => {
                let hash_encoded = parts.next().ok_or_else(|| {
                    PersistenceError::KeySerialization("missing hash component".into())
                })?;
                let range_encoded = parts.next().ok_or_else(|| {
                    PersistenceError::KeySerialization("missing range component".into())
                })?;
                let hash = decode_segment(hash_encoded)?;
                let range = decode_segment(range_encoded)?;
                Ok(Self::Composite { hash, range })
            }
            _ => Err(PersistenceError::KeySerialization(format!(
                "unrecognised key format: {serialized}"
            ))),
        }
    }

    /// Build the sled storage key for the given schema.
    #[must_use]
    pub fn to_storage_key(&self, schema_name: &str) -> String {
        format!("{}:{}:{}", STORAGE_PREFIX, schema_name, self.serialize())
    }

    /// Returns a user-friendly description of the key for diagnostics.
    #[must_use]
    pub fn debug_summary(&self) -> String {
        match self {
            Self::Single(value) => format!("single({value})"),
            Self::Composite { hash, range } => format!("composite(hash={hash}, range={range})"),
        }
    }
}

/// Persistence-specific error type that captures validation and storage failures.
#[derive(Debug, Error)]
pub enum PersistenceError {
    /// Requested schema is not registered with the provider.
    #[error("schema '{schema}' not found")]
    SchemaNotFound { schema: String },

    /// Input payload references a field that is not part of the schema.
    #[error("unknown field '{field}' for schema '{schema}'")]
    UnknownField { schema: String, field: String },

    /// Required field is missing from the payload and cannot be defaulted.
    #[error("missing required field '{field}' for schema '{schema}'")]
    MissingRequiredField { schema: String, field: String },

    /// Key-defining field is missing from the payload.
    #[error("key field '{field}' missing for schema '{schema}'")]
    MissingKeyField { schema: String, field: String },

    /// Key-defining field contains a value that cannot be serialised into a stable key.
    #[error("key field '{field}' for schema '{schema}' contains unsupported value")]
    InvalidKeyValue { schema: String, field: String },

    /// Field value does not match the declared field type.
    #[error(
        "field '{field}' in schema '{schema}' type mismatch: expected {expected:?}, got {actual:?}"
    )]
    FieldTypeMismatch {
        schema: String,
        field: String,
        expected: Box<FieldType>,
        actual: Box<FieldType>,
    },

    /// Record was not found for the specified key.
    #[error("record not found for schema '{schema}' and key '{key}'")]
    RecordNotFound { schema: String, key: String },

    /// Conversion to/from storage key failed.
    #[error("key serialization failed: {0}")]
    KeySerialization(String),

    /// Underlying database layer produced an error.
    #[error("database error: {0}")]
    Database(#[from] SchemaError),
}

/// Internal representation of a persisted record.
#[derive(Debug, Serialize, Deserialize)]
struct NativeStoredRecord {
    schema: String,
    key: String,
    data: HashMap<String, JsonValue>,
}

/// Provides typed persistence over [`DbOperations`] without leaking JSON usage into callers.
#[derive(Clone)]
pub struct NativePersistence {
    db_ops: Arc<DbOperations>,
    schema_provider: Arc<dyn NativeSchemaProvider>,
}

impl NativePersistence {
    /// Create a new persistence helper.
    #[must_use]
    pub fn new(db_ops: Arc<DbOperations>, schema_provider: Arc<dyn NativeSchemaProvider>) -> Self {
        Self {
            db_ops,
            schema_provider,
        }
    }

    /// Persist native data for the provided schema, returning the computed record key.
    pub fn store_data(
        &self,
        schema_name: &str,
        data: &HashMap<String, FieldValue>,
    ) -> Result<NativeRecordKey, PersistenceError> {
        let schema = self.schema_provider.schema_for(schema_name)?;
        let normalized = Self::normalize_and_validate(schema_name, &schema, data)?;
        let record_key = Self::extract_record_key(schema_name, &schema.key, &normalized)?;

        let storage_key = record_key.to_storage_key(schema_name);
        let stored_record = NativeStoredRecord {
            schema: schema.name,
            key: record_key.serialize(),
            data: Self::convert_to_db_format(&normalized),
        };

        self.db_ops
            .store_item(&storage_key, &stored_record)
            .map_err(PersistenceError::from)?;

        Ok(record_key)
    }

    /// Load a record for the provided key, returning native values after schema validation.
    pub fn load_data(
        &self,
        schema_name: &str,
        key: &NativeRecordKey,
    ) -> Result<HashMap<String, FieldValue>, PersistenceError> {
        let schema = self.schema_provider.schema_for(schema_name)?;
        let storage_key = key.to_storage_key(schema_name);

        let stored = self
            .db_ops
            .get_item::<NativeStoredRecord>(&storage_key)
            .map_err(PersistenceError::from)?
            .ok_or_else(|| PersistenceError::RecordNotFound {
                schema: schema_name.to_string(),
                key: key.serialize(),
            })?;

        let field_values = Self::convert_from_db_format(&stored.data);
        Self::normalize_and_validate(schema_name, &schema, &field_values)
    }

    /// Convenience helper to load data using the serialized key string.
    pub fn load_data_by_serialized_key(
        &self,
        schema_name: &str,
        serialized_key: &str,
    ) -> Result<HashMap<String, FieldValue>, PersistenceError> {
        let key = NativeRecordKey::from_serialized(serialized_key)?;
        self.load_data(schema_name, &key)
    }

    fn normalize_and_validate(
        schema_name: &str,
        schema: &SchemaDescription,
        data: &HashMap<String, FieldValue>,
    ) -> Result<HashMap<String, FieldValue>, PersistenceError> {
        let mut normalized = HashMap::new();

        for (field_name, definition) in &schema.fields {
            if let Some(value) = data.get(field_name) {
                if !definition.field_type.matches(value) {
                    return Err(PersistenceError::FieldTypeMismatch {
                        schema: schema_name.to_string(),
                        field: field_name.clone(),
                        expected: Box::new(definition.field_type.clone()),
                        actual: Box::new(value.field_type()),
                    });
                }
                normalized.insert(field_name.clone(), value.clone());
            } else if let Some(default_value) = definition.effective_default() {
                normalized.insert(field_name.clone(), default_value);
            } else {
                return Err(PersistenceError::MissingRequiredField {
                    schema: schema_name.to_string(),
                    field: field_name.clone(),
                });
            }
        }

        for field_name in data.keys() {
            if !schema.fields.contains_key(field_name) {
                return Err(PersistenceError::UnknownField {
                    schema: schema_name.to_string(),
                    field: field_name.clone(),
                });
            }
        }

        Ok(normalized)
    }

    fn extract_record_key(
        schema_name: &str,
        key_config: &KeyConfig,
        data: &HashMap<String, FieldValue>,
    ) -> Result<NativeRecordKey, PersistenceError> {
        match key_config {
            KeyConfig::Single { key_field } => {
                let value =
                    data.get(key_field)
                        .ok_or_else(|| PersistenceError::MissingKeyField {
                            schema: schema_name.to_string(),
                            field: key_field.clone(),
                        })?;
                let segment = Self::key_segment(schema_name, key_field, value)?;
                Ok(NativeRecordKey::single(segment))
            }
            KeyConfig::Range {
                hash_field,
                range_field,
            }
            | KeyConfig::HashRange {
                hash_field,
                range_field,
            } => {
                let hash_value =
                    data.get(hash_field)
                        .ok_or_else(|| PersistenceError::MissingKeyField {
                            schema: schema_name.to_string(),
                            field: hash_field.clone(),
                        })?;
                let range_value =
                    data.get(range_field)
                        .ok_or_else(|| PersistenceError::MissingKeyField {
                            schema: schema_name.to_string(),
                            field: range_field.clone(),
                        })?;

                let hash_segment = Self::key_segment(schema_name, hash_field, hash_value)?;
                let range_segment = Self::key_segment(schema_name, range_field, range_value)?;
                Ok(NativeRecordKey::composite(hash_segment, range_segment))
            }
        }
    }

    fn key_segment(
        schema_name: &str,
        field_name: &str,
        value: &FieldValue,
    ) -> Result<String, PersistenceError> {
        match value {
            FieldValue::String(v) => Ok(v.clone()),
            FieldValue::Integer(v) => Ok(v.to_string()),
            FieldValue::Number(v) => Ok(v.to_string()),
            FieldValue::Boolean(v) => Ok(v.to_string()),
            FieldValue::Null | FieldValue::Array(_) | FieldValue::Object(_) => {
                Err(PersistenceError::InvalidKeyValue {
                    schema: schema_name.to_string(),
                    field: field_name.to_string(),
                })
            }
        }
    }

    fn convert_to_db_format(data: &HashMap<String, FieldValue>) -> HashMap<String, JsonValue> {
        data.iter()
            .map(|(field, value)| (field.clone(), value.to_json_value()))
            .collect()
    }

    fn convert_from_db_format(data: &HashMap<String, JsonValue>) -> HashMap<String, FieldValue> {
        data.iter()
            .map(|(field, value)| (field.clone(), FieldValue::from_json_value(value.clone())))
            .collect()
    }
}

fn encode_segment(segment: &str) -> String {
    STANDARD_NO_PAD.encode(segment.as_bytes())
}

fn decode_segment(encoded: &str) -> Result<String, PersistenceError> {
    let bytes = STANDARD_NO_PAD
        .decode(encoded)
        .map_err(|err| PersistenceError::KeySerialization(err.to_string()))?;
    String::from_utf8(bytes).map_err(|err| PersistenceError::KeySerialization(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_key_round_trip_single() {
        let original = NativeRecordKey::single("identifier-123");
        let serialized = original.serialize();
        let decoded = NativeRecordKey::from_serialized(&serialized).unwrap();
        assert_eq!(original, decoded);
        assert_eq!(
            original.to_storage_key("schema"),
            decoded.to_storage_key("schema")
        );
    }

    #[test]
    fn record_key_round_trip_composite() {
        let original = NativeRecordKey::composite("customer_42", "2025-09-24");
        let serialized = original.serialize();
        let decoded = NativeRecordKey::from_serialized(&serialized).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn record_key_invalid_format() {
        let error = NativeRecordKey::from_serialized("invalid").unwrap_err();
        assert!(matches!(error, PersistenceError::KeySerialization(_)));
    }
}
