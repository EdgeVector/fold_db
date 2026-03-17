use crate::schema::types::field::FieldValue;
use crate::schema::types::key_config::KeyConfig;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;

/// Reference to a source schema field, parsed from "Schema.field" format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldRef {
    pub schema: String,
    pub field: String,
}

impl FieldRef {
    pub fn new(schema: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
            field: field.into(),
        }
    }
}

impl TryFrom<&str> for FieldRef {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("FieldRef cannot be empty".to_string());
        }
        let dot_pos = trimmed
            .find('.')
            .ok_or_else(|| format!("FieldRef '{}' must contain a dot separator", trimmed))?;
        let schema = &trimmed[..dot_pos];
        let field = &trimmed[dot_pos + 1..];
        if schema.is_empty() {
            return Err(format!("FieldRef '{}' has empty schema name", trimmed));
        }
        if field.is_empty() {
            return Err(format!("FieldRef '{}' has empty field name", trimmed));
        }
        Ok(Self {
            schema: schema.to_string(),
            field: field.to_string(),
        })
    }
}

impl TryFrom<String> for FieldRef {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        FieldRef::try_from(value.as_str())
    }
}

impl std::fmt::Display for FieldRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.schema, self.field)
    }
}

/// Definition of a single transform field within a view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformFieldDef {
    pub source: FieldRef,
    /// Forward WASM transform bytes. None = identity (pass-through).
    pub wasm_forward: Option<Vec<u8>>,
    /// Inverse WASM transform bytes. None = irreversible (if forward exists).
    pub wasm_inverse: Option<Vec<u8>>,
}

/// Write mode determined at registration, not declared by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransformWriteMode {
    /// No WASM — reads/writes pass through directly to source.
    Identity,
    /// Forward + inverse WASM verified — writes go through inverse to source.
    Reversible,
    /// Forward only — writes store directly on the view field.
    Irreversible,
}

/// Three-state machine for each view field's cached value.
/// Stores keyed results to preserve the source's key structure.
/// Uses Vec<(K,V)> instead of HashMap for JSON serialization compatibility
/// (KeyValue is a struct and can't be a JSON map key).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformFieldState {
    /// Never accessed, nothing computed.
    Empty,
    /// Computed from source, will be invalidated on source change.
    Cached {
        entries: Vec<(KeyValue, FieldValue)>,
    },
    /// Directly written by user, source link is stale.
    Overridden {
        entries: Vec<(KeyValue, FieldValue)>,
    },
}

impl TransformFieldState {
    /// Invalidate a cached value back to Empty. Overridden values are not affected.
    pub fn invalidate(&mut self) {
        if matches!(self, TransformFieldState::Cached { .. }) {
            *self = TransformFieldState::Empty;
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, TransformFieldState::Empty)
    }
}

/// The view definition — a separate type from Schema.
/// Declares its own schema_type and key configuration, just like a schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformView {
    pub name: String,
    /// The schema type determines how fields are keyed (Single, Hash, Range, HashRange).
    pub schema_type: SchemaType,
    /// Key configuration — which fields serve as hash/range keys.
    #[serde(default)]
    pub key: Option<KeyConfig>,
    pub fields: HashMap<String, TransformFieldDef>,
    /// Computed at registration — write mode per field.
    #[serde(default)]
    pub write_modes: HashMap<String, TransformWriteMode>,
}

impl TransformView {
    pub fn new(
        name: impl Into<String>,
        schema_type: SchemaType,
        key: Option<KeyConfig>,
        fields: HashMap<String, TransformFieldDef>,
    ) -> Self {
        Self {
            name: name.into(),
            schema_type,
            key,
            fields,
            write_modes: HashMap::new(),
        }
    }

    /// Get all unique source schemas referenced by this view's fields.
    pub fn source_schemas(&self) -> Vec<String> {
        let mut schemas: Vec<String> = self
            .fields
            .values()
            .map(|f| f.source.schema.clone())
            .collect();
        schemas.sort();
        schemas.dedup();
        schemas
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_ref_parse() {
        let fr = FieldRef::try_from("BlogPost.content").unwrap();
        assert_eq!(fr.schema, "BlogPost");
        assert_eq!(fr.field, "content");
    }

    #[test]
    fn test_field_ref_parse_errors() {
        assert!(FieldRef::try_from("").is_err());
        assert!(FieldRef::try_from("nodot").is_err());
        assert!(FieldRef::try_from(".field").is_err());
        assert!(FieldRef::try_from("schema.").is_err());
    }

    #[test]
    fn test_field_ref_display() {
        let fr = FieldRef::new("Weather", "temp");
        assert_eq!(fr.to_string(), "Weather.temp");
    }

    #[test]
    fn test_transform_field_state_invalidate() {
        let mut cached = TransformFieldState::Cached {
            entries: Vec::new(),
        };
        cached.invalidate();
        assert!(cached.is_empty());

        let mut overridden = TransformFieldState::Overridden {
            entries: Vec::new(),
        };
        overridden.invalidate();
        assert!(!overridden.is_empty()); // Overridden not affected
    }

    #[test]
    fn test_transform_view_source_schemas() {
        let mut fields = HashMap::new();
        fields.insert(
            "a".into(),
            TransformFieldDef {
                source: FieldRef::new("S1", "f1"),
                wasm_forward: None,
                wasm_inverse: None,
            },
        );
        fields.insert(
            "b".into(),
            TransformFieldDef {
                source: FieldRef::new("S2", "f2"),
                wasm_forward: None,
                wasm_inverse: None,
            },
        );
        fields.insert(
            "c".into(),
            TransformFieldDef {
                source: FieldRef::new("S1", "f3"),
                wasm_forward: None,
                wasm_inverse: None,
            },
        );
        let view = TransformView::new("test_view", SchemaType::Single, None, fields);
        let schemas = view.source_schemas();
        assert_eq!(schemas, vec!["S1", "S2"]);
    }
}
