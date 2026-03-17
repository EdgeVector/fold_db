use crate::schema::types::errors::SchemaError;
use crate::schema::types::field::{FieldValue, HashRangeFilter};
use crate::schema::types::key_value::KeyValue;
use crate::view::types::{TransformFieldState, TransformView};
use crate::view::wasm_engine::WasmTransformEngine;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for querying source schema fields — breaks circular dependency
/// between QueryExecutor and ViewFieldResolver.
#[async_trait]
pub trait SourceQueryFn: Send + Sync {
    /// Query a single field from a source schema, returning keyed results
    /// with the source's key structure preserved.
    async fn query_field(
        &self,
        schema_name: &str,
        field_name: &str,
        filter: Option<HashRangeFilter>,
        as_of: Option<DateTime<Utc>>,
    ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError>;
}

/// Resolves view fields using the three-state machine (Empty → Cached, Overridden).
#[derive(Debug)]
pub struct ViewFieldResolver {
    wasm_engine: Arc<WasmTransformEngine>,
}

impl ViewFieldResolver {
    pub fn new(wasm_engine: Arc<WasmTransformEngine>) -> Self {
        Self { wasm_engine }
    }

    /// Resolve a single view field's value.
    ///
    /// State machine:
    /// - `Overridden` → return stored values
    /// - `Cached` → return cached values
    /// - `Empty` → query source, apply forward WASM, cache, return
    ///
    /// Filter is only applied on the `Empty` path (passed through to source).
    /// Cached/Overridden results are returned as-is.
    pub async fn resolve_field(
        &self,
        view: &TransformView,
        field_name: &str,
        field_state: &TransformFieldState,
        source_query: &dyn SourceQueryFn,
        filter: Option<HashRangeFilter>,
        as_of: Option<DateTime<Utc>>,
    ) -> Result<(HashMap<KeyValue, FieldValue>, TransformFieldState), SchemaError> {
        let field_def = view.fields.get(field_name).ok_or_else(|| {
            SchemaError::InvalidField(format!(
                "Field '{}' not found in view '{}'",
                field_name, view.name
            ))
        })?;

        match field_state {
            TransformFieldState::Overridden { entries } => {
                Ok((entries.iter().cloned().collect(), field_state.clone()))
            }
            TransformFieldState::Cached { entries } => {
                Ok((entries.iter().cloned().collect(), field_state.clone()))
            }
            TransformFieldState::Empty => {
                // Query source with filter pass-through
                let source_results = source_query
                    .query_field(
                        &field_def.source.schema,
                        &field_def.source.field,
                        filter,
                        as_of,
                    )
                    .await?;

                // Apply forward WASM if present, preserving keys
                let result = if let Some(wasm_forward) = &field_def.wasm_forward {
                    let mut transformed = HashMap::with_capacity(source_results.len());
                    for (key, fv) in source_results {
                        let new_value = self.wasm_engine.execute(wasm_forward, &fv.value)?;
                        transformed.insert(
                            key,
                            FieldValue {
                                value: new_value,
                                atom_uuid: fv.atom_uuid,
                                source_file_name: fv.source_file_name,
                                metadata: fv.metadata,
                                molecule_uuid: fv.molecule_uuid,
                                molecule_version: fv.molecule_version,
                            },
                        );
                    }
                    transformed
                } else {
                    source_results
                };

                let new_state = TransformFieldState::Cached {
                    entries: result.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                };

                Ok((result, new_state))
            }
        }
    }

    pub fn wasm_engine(&self) -> &Arc<WasmTransformEngine> {
        &self.wasm_engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
    use crate::view::types::{FieldRef, TransformFieldDef};

    /// Mock source query that returns fixed keyed values.
    struct MockSourceQuery {
        values: HashMap<(String, String), HashMap<KeyValue, FieldValue>>,
    }

    #[async_trait]
    impl SourceQueryFn for MockSourceQuery {
        async fn query_field(
            &self,
            schema_name: &str,
            field_name: &str,
            _filter: Option<HashRangeFilter>,
            _as_of: Option<DateTime<Utc>>,
        ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
            let key = (schema_name.to_string(), field_name.to_string());
            self.values
                .get(&key)
                .cloned()
                .ok_or_else(|| SchemaError::NotFound(format!("{}.{}", schema_name, field_name)))
        }
    }

    fn make_field_value(val: serde_json::Value) -> FieldValue {
        FieldValue {
            value: val,
            atom_uuid: String::new(),
            source_file_name: None,
            metadata: None,
            molecule_uuid: None,
            molecule_version: None,
        }
    }

    fn make_view() -> TransformView {
        let mut fields = HashMap::new();
        fields.insert(
            "content".into(),
            TransformFieldDef {
                source: FieldRef::new("BlogPost", "content"),
                wasm_forward: None,
                wasm_inverse: None,
            },
        );
        TransformView::new("TestView", SchemaType::Range, None, fields)
    }

    fn make_resolver() -> ViewFieldResolver {
        let engine = Arc::new(WasmTransformEngine::new().unwrap());
        ViewFieldResolver::new(engine)
    }

    #[tokio::test]
    async fn test_resolve_overridden() {
        let resolver = make_resolver();
        let view = make_view();
        let mut vals = HashMap::new();
        vals.insert(
            KeyValue::new(None, Some("r1".into())),
            make_field_value(serde_json::json!("custom")),
        );
        let state = TransformFieldState::Overridden { entries: vals.into_iter().collect() };
        let mock = MockSourceQuery {
            values: HashMap::new(),
        };

        let (results, new_state) = resolver
            .resolve_field(&view, "content", &state, &mock, None, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(matches!(new_state, TransformFieldState::Overridden { .. }));
    }

    #[tokio::test]
    async fn test_resolve_cached() {
        let resolver = make_resolver();
        let view = make_view();
        let mut vals = HashMap::new();
        vals.insert(
            KeyValue::new(None, Some("r1".into())),
            make_field_value(serde_json::json!("cached_val")),
        );
        let state = TransformFieldState::Cached { entries: vals.into_iter().collect() };
        let mock = MockSourceQuery {
            values: HashMap::new(),
        };

        let (results, _) = resolver
            .resolve_field(&view, "content", &state, &mock, None, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        let fv = results.values().next().unwrap();
        assert_eq!(fv.value, serde_json::json!("cached_val"));
    }

    #[tokio::test]
    async fn test_resolve_empty_queries_source() {
        let resolver = make_resolver();
        let view = make_view();
        let state = TransformFieldState::Empty;

        let mut source_field_values = HashMap::new();
        source_field_values.insert(
            KeyValue::new(None, Some("2026-01-01".into())),
            make_field_value(serde_json::json!("hello world")),
        );

        let mut source_values = HashMap::new();
        source_values.insert(
            ("BlogPost".into(), "content".into()),
            source_field_values,
        );
        let mock = MockSourceQuery {
            values: source_values,
        };

        let (results, new_state) = resolver
            .resolve_field(&view, "content", &state, &mock, None, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        let (key, fv) = results.iter().next().unwrap();
        assert_eq!(key.range.as_deref(), Some("2026-01-01"));
        assert_eq!(fv.value, serde_json::json!("hello world"));
        assert!(matches!(new_state, TransformFieldState::Cached { .. }));
    }

    #[tokio::test]
    async fn test_resolve_unknown_field_errors() {
        let resolver = make_resolver();
        let view = make_view();
        let state = TransformFieldState::Empty;
        let mock = MockSourceQuery {
            values: HashMap::new(),
        };

        let result = resolver
            .resolve_field(&view, "nonexistent", &state, &mock, None, None)
            .await;
        assert!(result.is_err());
    }
}
