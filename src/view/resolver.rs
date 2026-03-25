use crate::schema::types::errors::SchemaError;
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::operations::Query;
use crate::view::types::{TransformView, ViewCacheState};
use crate::view::wasm_engine::WasmTransformEngine;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for querying source schemas — breaks circular dependency
/// between QueryExecutor and ViewResolver.
///
/// Accepts a full Query and returns all field results keyed by field name.
#[async_trait]
pub trait SourceQueryFn: Send + Sync {
    async fn execute_query(
        &self,
        query: &Query,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError>;
}

/// Resolves view output by executing input queries, optionally running WASM,
/// and validating output types.
#[derive(Debug)]
pub struct ViewResolver {
    wasm_engine: Arc<WasmTransformEngine>,
}

impl ViewResolver {
    pub fn new(wasm_engine: Arc<WasmTransformEngine>) -> Self {
        Self { wasm_engine }
    }

    /// Resolve a view's output fields.
    ///
    /// 1. If cached → validate requested fields exist, return from cache
    /// 2. Execute each input query via SourceQueryFn
    /// 3. If WASM: assemble input JSON, pass to WASM, parse output
    /// 4. If no WASM (identity): pass through query results directly
    /// 5. Validate output against output_fields types
    /// 6. Cache entire output, return requested fields
    pub async fn resolve(
        &self,
        view: &TransformView,
        requested_fields: &[String],
        cache_state: &ViewCacheState,
        source_query: &dyn SourceQueryFn,
    ) -> Result<
        (
            HashMap<String, HashMap<KeyValue, FieldValue>>,
            ViewCacheState,
        ),
        SchemaError,
    > {
        // Determine which fields to return
        let fields_to_return: Vec<String> = if requested_fields.is_empty() {
            view.output_fields.keys().cloned().collect()
        } else {
            // Validate requested fields exist in output schema
            for field_name in requested_fields {
                if !view.output_fields.contains_key(field_name) {
                    return Err(SchemaError::InvalidField(format!(
                        "Field '{}' not found in view '{}'",
                        field_name, view.name
                    )));
                }
            }
            requested_fields.to_vec()
        };

        // Check cache
        if let ViewCacheState::Cached { entries } = cache_state {
            let mut result = HashMap::new();
            for field_name in &fields_to_return {
                let field_entries = entries.get(field_name).cloned().unwrap_or_default();
                result.insert(field_name.clone(), field_entries.into_iter().collect());
            }
            return Ok((result, cache_state.clone()));
        }

        // Execute all input queries, merging results when multiple queries target the same schema
        let mut all_query_results: HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>> =
            HashMap::new();
        for query in &view.input_queries {
            let query_results = source_query.execute_query(query).await?;
            all_query_results
                .entry(query.schema_name.clone())
                .or_default()
                .extend(query_results);
        }

        // Compute output
        let output = if let Some(wasm_bytes) = &view.wasm_transform {
            self.execute_wasm_transform(wasm_bytes, &all_query_results)?
        } else {
            self.identity_pass_through(&all_query_results, view)?
        };

        // Validate output against declared types
        for (field_name, field_type) in &view.output_fields {
            if let Some(field_entries) = output.get(field_name) {
                for fv in field_entries.values() {
                    if let Err(e) = field_type.validate(&fv.value) {
                        return Err(SchemaError::InvalidData(format!(
                            "View '{}' output field '{}' type validation failed: {}",
                            view.name, field_name, e
                        )));
                    }
                }
            }
        }

        // Build cache state
        let cache_entries: HashMap<String, Vec<(KeyValue, FieldValue)>> = output
            .iter()
            .map(|(field_name, entries)| {
                (
                    field_name.clone(),
                    entries
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                )
            })
            .collect();
        let new_cache = ViewCacheState::Cached {
            entries: cache_entries,
        };

        // Return only requested fields
        let mut result = HashMap::new();
        for field_name in &fields_to_return {
            let field_entries = output.get(field_name).cloned().unwrap_or_default();
            result.insert(field_name.clone(), field_entries);
        }

        Ok((result, new_cache))
    }

    /// Identity pass-through: collect all query results and map to output fields.
    /// Output field names must match source field names from input queries.
    /// Errors if the same field name appears from multiple source schemas.
    fn identity_pass_through(
        &self,
        query_results: &HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>>,
        view: &TransformView,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        let mut output = HashMap::new();

        for (schema_name, fields) in query_results {
            for (field_name, entries) in fields {
                // Only include fields that are in the output schema
                if view.output_fields.contains_key(field_name) {
                    if output.contains_key(field_name) {
                        return Err(SchemaError::InvalidData(format!(
                            "Identity view '{}': field '{}' returned by multiple source schemas (including '{}')",
                            view.name, field_name, schema_name
                        )));
                    }
                    output.insert(field_name.clone(), entries.clone());
                }
            }
        }

        Ok(output)
    }

    /// Execute the WASM transform with assembled input from all queries.
    fn execute_wasm_transform(
        &self,
        wasm_bytes: &[u8],
        query_results: &HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>>,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        // Assemble input JSON: { "inputs": { schema_name: { field: { key: value } } } }
        let mut inputs = serde_json::Map::new();
        for (schema_name, fields) in query_results {
            let mut schema_fields = serde_json::Map::new();
            for (field_name, entries) in fields {
                let mut field_map = serde_json::Map::new();
                for (key, fv) in entries {
                    let key_str = key.to_string();
                    field_map.insert(key_str, fv.value.clone());
                }
                schema_fields.insert(field_name.clone(), serde_json::Value::Object(field_map));
            }
            inputs.insert(
                schema_name.clone(),
                serde_json::Value::Object(schema_fields),
            );
        }

        let input_json = serde_json::json!({ "inputs": inputs });
        let output_json = self.wasm_engine.execute(wasm_bytes, &input_json)?;

        // Parse output JSON: { "fields": { field_name: { key: value } } }
        let fields_obj = output_json
            .get("fields")
            .and_then(|f| f.as_object())
            .ok_or_else(|| {
                SchemaError::InvalidData(
                    "WASM transform output must have a 'fields' object".to_string(),
                )
            })?;

        let mut output = HashMap::new();
        for (field_name, entries_value) in fields_obj {
            let entries_obj = entries_value.as_object().ok_or_else(|| {
                SchemaError::InvalidData(format!(
                    "WASM output field '{}' must be an object mapping keys to values",
                    field_name
                ))
            })?;

            let mut field_entries = HashMap::new();
            for (key_str, value) in entries_obj {
                let key = KeyValue::new(None, Some(key_str.clone()));
                let fv = FieldValue {
                    value: value.clone(),
                    atom_uuid: String::new(),
                    source_file_name: None,
                    metadata: None,
                    molecule_uuid: None,
                    molecule_version: None,
                };
                field_entries.insert(key, fv);
            }
            output.insert(field_name.clone(), field_entries);
        }

        Ok(output)
    }

    pub fn wasm_engine(&self) -> &Arc<WasmTransformEngine> {
        &self.wasm_engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::field_value_type::FieldValueType;
    use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;

    struct MockSourceQuery {
        results: HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>>,
    }

    #[async_trait]
    impl SourceQueryFn for MockSourceQuery {
        async fn execute_query(
            &self,
            query: &Query,
        ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
            self.results
                .get(&query.schema_name)
                .cloned()
                .ok_or_else(|| {
                    SchemaError::NotFound(format!("Schema '{}' not found", query.schema_name))
                })
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

    fn make_resolver() -> ViewResolver {
        let engine = Arc::new(WasmTransformEngine::new().unwrap());
        ViewResolver::new(engine)
    }

    fn make_identity_view() -> TransformView {
        TransformView::new(
            "TestView",
            SchemaType::Range,
            None,
            vec![Query::new(
                "BlogPost".to_string(),
                vec!["content".to_string()],
            )],
            None,
            HashMap::from([("content".to_string(), FieldValueType::Any)]),
        )
    }

    #[tokio::test]
    async fn test_resolve_cached() {
        let resolver = make_resolver();
        let view = make_identity_view();

        let mut entries = HashMap::new();
        entries.insert(
            "content".to_string(),
            vec![(
                KeyValue::new(None, Some("r1".into())),
                make_field_value(serde_json::json!("cached_val")),
            )],
        );
        let cache = ViewCacheState::Cached { entries };
        let mock = MockSourceQuery {
            results: HashMap::new(),
        };

        let (results, _) = resolver
            .resolve(&view, &["content".to_string()], &cache, &mock)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        let fv = results["content"].values().next().unwrap();
        assert_eq!(fv.value, serde_json::json!("cached_val"));
    }

    #[tokio::test]
    async fn test_resolve_empty_queries_source() {
        let resolver = make_resolver();
        let view = make_identity_view();
        let cache = ViewCacheState::Empty;

        let mut source_field_values = HashMap::new();
        source_field_values.insert(
            KeyValue::new(None, Some("2026-01-01".into())),
            make_field_value(serde_json::json!("hello world")),
        );

        let mut blogpost_fields = HashMap::new();
        blogpost_fields.insert("content".to_string(), source_field_values);

        let mut results_map = HashMap::new();
        results_map.insert("BlogPost".to_string(), blogpost_fields);

        let mock = MockSourceQuery {
            results: results_map,
        };

        let (results, new_cache) = resolver
            .resolve(&view, &["content".to_string()], &cache, &mock)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        let (key, fv) = results["content"].iter().next().unwrap();
        assert_eq!(key.range.as_deref(), Some("2026-01-01"));
        assert_eq!(fv.value, serde_json::json!("hello world"));
        assert!(matches!(new_cache, ViewCacheState::Cached { .. }));
    }

    #[tokio::test]
    async fn test_resolve_unknown_field_errors() {
        let resolver = make_resolver();
        let view = make_identity_view();
        let cache = ViewCacheState::Empty;
        let mock = MockSourceQuery {
            results: HashMap::new(),
        };

        let result = resolver
            .resolve(&view, &["nonexistent".to_string()], &cache, &mock)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_empty_fields_returns_all() {
        let resolver = make_resolver();
        let view = TransformView::new(
            "MultiView",
            SchemaType::Range,
            None,
            vec![Query::new(
                "BlogPost".to_string(),
                vec!["title".to_string(), "content".to_string()],
            )],
            None,
            HashMap::from([
                ("title".to_string(), FieldValueType::Any),
                ("content".to_string(), FieldValueType::Any),
            ]),
        );
        let cache = ViewCacheState::Empty;

        let mut title_values = HashMap::new();
        title_values.insert(
            KeyValue::new(None, Some("r1".into())),
            make_field_value(serde_json::json!("Title")),
        );
        let mut content_values = HashMap::new();
        content_values.insert(
            KeyValue::new(None, Some("r1".into())),
            make_field_value(serde_json::json!("Content")),
        );

        let mut blogpost_fields = HashMap::new();
        blogpost_fields.insert("title".to_string(), title_values);
        blogpost_fields.insert("content".to_string(), content_values);

        let mut results_map = HashMap::new();
        results_map.insert("BlogPost".to_string(), blogpost_fields);

        let mock = MockSourceQuery {
            results: results_map,
        };

        let (results, _) = resolver.resolve(&view, &[], &cache, &mock).await.unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.contains_key("title"));
        assert!(results.contains_key("content"));
    }

    #[tokio::test]
    async fn test_multi_query_identity_view() {
        let resolver = make_resolver();
        let view = TransformView::new(
            "MultiSourceView",
            SchemaType::Range,
            None,
            vec![
                Query::new("BlogPost".to_string(), vec!["title".to_string()]),
                Query::new("Author".to_string(), vec!["name".to_string()]),
            ],
            None,
            HashMap::from([
                ("title".to_string(), FieldValueType::Any),
                ("name".to_string(), FieldValueType::Any),
            ]),
        );
        let cache = ViewCacheState::Empty;

        let mut title_values = HashMap::new();
        title_values.insert(
            KeyValue::new(None, Some("r1".into())),
            make_field_value(serde_json::json!("Hello")),
        );
        let mut name_values = HashMap::new();
        name_values.insert(
            KeyValue::new(None, Some("a1".into())),
            make_field_value(serde_json::json!("Tom")),
        );

        let mut blogpost_fields = HashMap::new();
        blogpost_fields.insert("title".to_string(), title_values);
        let mut author_fields = HashMap::new();
        author_fields.insert("name".to_string(), name_values);

        let mut results_map = HashMap::new();
        results_map.insert("BlogPost".to_string(), blogpost_fields);
        results_map.insert("Author".to_string(), author_fields);

        let mock = MockSourceQuery {
            results: results_map,
        };

        let (results, _) = resolver.resolve(&view, &[], &cache, &mock).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(
            results["title"].values().next().unwrap().value,
            serde_json::json!("Hello")
        );
        assert_eq!(
            results["name"].values().next().unwrap().value,
            serde_json::json!("Tom")
        );
    }

    #[tokio::test]
    async fn test_empty_source_data() {
        let resolver = make_resolver();
        let view = make_identity_view();
        let cache = ViewCacheState::Empty;

        // Source returns empty field results
        let mut blogpost_fields = HashMap::new();
        blogpost_fields.insert("content".to_string(), HashMap::new());

        let mut results_map = HashMap::new();
        results_map.insert("BlogPost".to_string(), blogpost_fields);

        let mock = MockSourceQuery {
            results: results_map,
        };

        let (results, new_cache) = resolver
            .resolve(&view, &["content".to_string()], &cache, &mock)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results["content"].is_empty());
        assert!(matches!(new_cache, ViewCacheState::Cached { .. }));
    }

    #[tokio::test]
    async fn test_same_schema_multi_query_merges() {
        let resolver = make_resolver();
        // Two queries against same schema, different fields
        let view = TransformView::new(
            "SplitView",
            SchemaType::Range,
            None,
            vec![
                Query::new("BlogPost".to_string(), vec!["title".to_string()]),
                Query::new("BlogPost".to_string(), vec!["content".to_string()]),
            ],
            None,
            HashMap::from([
                ("title".to_string(), FieldValueType::Any),
                ("content".to_string(), FieldValueType::Any),
            ]),
        );
        let cache = ViewCacheState::Empty;

        let mut title_values = HashMap::new();
        title_values.insert(
            KeyValue::new(None, Some("r1".into())),
            make_field_value(serde_json::json!("Hello")),
        );
        let mut content_values = HashMap::new();
        content_values.insert(
            KeyValue::new(None, Some("r1".into())),
            make_field_value(serde_json::json!("World")),
        );

        // MockSourceQuery returns different fields for each query
        let mut blogpost_all = HashMap::new();
        blogpost_all.insert("title".to_string(), title_values);
        blogpost_all.insert("content".to_string(), content_values);

        let mut results_map = HashMap::new();
        results_map.insert("BlogPost".to_string(), blogpost_all);

        let mock = MockSourceQuery {
            results: results_map,
        };

        let (results, _) = resolver.resolve(&view, &[], &cache, &mock).await.unwrap();

        // Both fields should be present (not overwritten)
        assert_eq!(results.len(), 2);
        assert_eq!(
            results["title"].values().next().unwrap().value,
            serde_json::json!("Hello")
        );
        assert_eq!(
            results["content"].values().next().unwrap().value,
            serde_json::json!("World")
        );
    }

    #[tokio::test]
    async fn test_type_validation_failure() {
        let resolver = make_resolver();
        let view = TransformView::new(
            "TypedView",
            SchemaType::Range,
            None,
            vec![Query::new(
                "BlogPost".to_string(),
                vec!["count".to_string()],
            )],
            None,
            HashMap::from([("count".to_string(), FieldValueType::Integer)]),
        );
        let cache = ViewCacheState::Empty;

        let mut count_values = HashMap::new();
        count_values.insert(
            KeyValue::new(None, Some("r1".into())),
            make_field_value(serde_json::json!("not_a_number")), // Wrong type
        );

        let mut blogpost_fields = HashMap::new();
        blogpost_fields.insert("count".to_string(), count_values);

        let mut results_map = HashMap::new();
        results_map.insert("BlogPost".to_string(), blogpost_fields);

        let mock = MockSourceQuery {
            results: results_map,
        };

        let result = resolver
            .resolve(&view, &["count".to_string()], &cache, &mock)
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("type validation failed"));
    }
}
