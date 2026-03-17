use crate::schema::types::field::FieldValue;
use crate::schema::types::field_value_type::FieldValueType;
use crate::schema::types::key_config::KeyConfig;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::operations::Query;
use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cache state for an entire view's computed output.
/// Per-view (not per-field) since the WASM transform is holistic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewCacheState {
    /// Never computed or invalidated.
    Empty,
    /// Computed output: field_name → (key → value).
    Cached {
        entries: HashMap<String, Vec<(KeyValue, FieldValue)>>,
    },
}

impl ViewCacheState {
    /// Reset cache to Empty.
    pub fn invalidate(&mut self) {
        *self = ViewCacheState::Empty;
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, ViewCacheState::Empty)
    }
}

/// The view definition — a multi-query typed view.
/// Declares input queries (potentially across multiple schemas),
/// an optional WASM transform, and a typed output schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformView {
    pub name: String,
    /// The schema type determines how output fields are keyed.
    pub schema_type: SchemaType,
    /// Key configuration — which fields serve as hash/range keys in output.
    #[serde(default)]
    pub key_config: Option<KeyConfig>,
    /// Queries to execute against source schemas.
    pub input_queries: Vec<Query>,
    /// WASM module bytes. None = identity pass-through.
    #[serde(default)]
    pub wasm_transform: Option<Vec<u8>>,
    /// Typed output schema: field_name → type.
    pub output_fields: HashMap<String, FieldValueType>,
}

impl TransformView {
    pub fn new(
        name: impl Into<String>,
        schema_type: SchemaType,
        key_config: Option<KeyConfig>,
        input_queries: Vec<Query>,
        wasm_transform: Option<Vec<u8>>,
        output_fields: HashMap<String, FieldValueType>,
    ) -> Self {
        Self {
            name: name.into(),
            schema_type,
            key_config,
            input_queries,
            wasm_transform,
            output_fields,
        }
    }

    /// Get all unique source schema names referenced by this view's input queries.
    pub fn source_schemas(&self) -> Vec<String> {
        let mut schemas: Vec<String> = self
            .input_queries
            .iter()
            .map(|q| q.schema_name.clone())
            .collect();
        schemas.sort();
        schemas.dedup();
        schemas
    }

    /// Get all (schema_name, field_name) pairs from input queries for dependency tracking.
    pub fn source_dependencies(&self) -> Vec<(String, String)> {
        let mut deps = Vec::new();
        for query in &self.input_queries {
            for field_name in &query.fields {
                deps.push((query.schema_name.clone(), field_name.clone()));
            }
        }
        deps
    }

    /// Check if this is an identity view (no WASM transform).
    pub fn is_identity(&self) -> bool {
        self.wasm_transform.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_cache_state_invalidate() {
        let mut cached = ViewCacheState::Cached {
            entries: HashMap::new(),
        };
        cached.invalidate();
        assert!(cached.is_empty());

        let mut empty = ViewCacheState::Empty;
        empty.invalidate();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_transform_view_source_schemas() {
        let view = TransformView::new(
            "test_view",
            SchemaType::Single,
            None,
            vec![
                Query::new("S1".to_string(), vec!["f1".to_string()]),
                Query::new("S2".to_string(), vec!["f2".to_string()]),
                Query::new("S1".to_string(), vec!["f3".to_string()]),
            ],
            None,
            HashMap::from([
                ("f1".to_string(), FieldValueType::Any),
                ("f2".to_string(), FieldValueType::Any),
                ("f3".to_string(), FieldValueType::Any),
            ]),
        );
        let schemas = view.source_schemas();
        assert_eq!(schemas, vec!["S1", "S2"]);
    }

    #[test]
    fn test_source_dependencies() {
        let view = TransformView::new(
            "test_view",
            SchemaType::Single,
            None,
            vec![
                Query::new("BlogPost".to_string(), vec!["title".to_string(), "content".to_string()]),
                Query::new("Author".to_string(), vec!["name".to_string()]),
            ],
            None,
            HashMap::from([
                ("enriched_title".to_string(), FieldValueType::String),
                ("word_count".to_string(), FieldValueType::Integer),
            ]),
        );
        let deps = view.source_dependencies();
        assert_eq!(deps.len(), 3);
        assert!(deps.contains(&("BlogPost".to_string(), "title".to_string())));
        assert!(deps.contains(&("BlogPost".to_string(), "content".to_string())));
        assert!(deps.contains(&("Author".to_string(), "name".to_string())));
    }

    #[test]
    fn test_is_identity() {
        let identity = TransformView::new(
            "id_view", SchemaType::Single, None, vec![], None,
            HashMap::new(),
        );
        assert!(identity.is_identity());

        let wasm_view = TransformView::new(
            "wasm_view", SchemaType::Single, None, vec![], Some(vec![0, 1, 2]),
            HashMap::new(),
        );
        assert!(!wasm_view.is_identity());
    }
}
