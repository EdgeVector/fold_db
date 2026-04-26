use crate::schema::types::errors::SchemaError;
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::operations::Query;
use crate::view::derived_metadata::{compute_derived_metadata, DerivedMetadata};
use crate::view::transform_field_override::TransformFieldOverride;
use crate::view::types::{InputDimension, TransformView};
use crate::view::wasm_engine::WasmTransformEngine;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Map a `SchemaError` produced by the WASM execution path into a
/// human-readable cause string. Used to enrich the `InvalidTransform`
/// error returned to callers (which gets surfaced through the trigger
/// runner's `TriggerFiring` audit row) when fuel is exhausted, the
/// module fails to compile, or the runtime traps. Compile failures keep
/// their leading `"Failed to compile WASM module"` prefix so log
/// downstream can tell them apart from execution traps.
fn wasm_failure_cause(view_name: &str, err: &SchemaError) -> String {
    match err {
        SchemaError::TransformGasExceeded { input_size } => {
            format!(
                "View '{}' unavailable: gas exceeded (input_size={})",
                view_name, input_size
            )
        }
        SchemaError::InvalidTransform(msg) => {
            format!("View '{}' unavailable: {}", view_name, msg)
        }
        other => format!("View '{}' unavailable: {}", view_name, other),
    }
}

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
///
/// Post-cache cleanup (`projects/view-compute-as-mutations` cache-deletion
/// PR), the resolver is purely functional: given a view definition,
/// requested fields, a source-query callback, and any user overrides, it
/// returns the computed output (and metadata for the derived-mutation
/// dual-write). Callers that need to persist the output write derived
/// mutations themselves; the resolver no longer owns any cache lifecycle.
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
    /// 1. Execute each input query via SourceQueryFn
    /// 2. If WASM: assemble input JSON, pass to WASM, parse output
    /// 3. If no WASM (identity): pass through query results directly
    /// 4. Validate output against output_fields types
    /// 5. Return only requested fields
    pub async fn resolve(
        &self,
        view: &TransformView,
        requested_fields: &[String],
        source_query: &dyn SourceQueryFn,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        self.resolve_with_overrides(view, requested_fields, source_query, &[])
            .await
    }

    /// Same as `resolve`, but applies any per-(field, key) overrides on top
    /// of the computed output. Overrides are looked up from the override
    /// store by the caller and passed in. When an override exists for a
    /// `(field, key)` it supersedes whatever the WASM/identity path
    /// produced — and the override survives even if the source link is
    /// stale, which is what makes `Overridden` sticky against subsequent
    /// source mutations.
    pub async fn resolve_with_overrides(
        &self,
        view: &TransformView,
        requested_fields: &[String],
        source_query: &dyn SourceQueryFn,
        overrides: &[(String, String, TransformFieldOverride)],
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        let (results, _derived) = self
            .resolve_with_overrides_and_derived(view, requested_fields, source_query, overrides)
            .await?;
        Ok(results)
    }

    /// Like [`resolve_with_overrides`] but additionally returns the
    /// [`DerivedMetadata`] for the just-executed fire.
    ///
    /// `DerivedMetadata` is `Some` only when the resolver actually executed
    /// a WASM transform on fresh input. It is `None` when the view is an
    /// identity pass-through (no derivation happened). Callers use the
    /// returned metadata to build `Provenance::Derived` mutations
    /// downstream (`projects/view-compute-as-mutations` PR 2).
    ///
    /// WASM failures (gas exceeded, compile error, execution trap, type
    /// validation failure, calibrated envelope rejection) are returned as
    /// `SchemaError::InvalidTransform`. Sticky-per-input behavior used to
    /// live in the resolver via `ViewCacheState::Unavailable`; with the
    /// cache gone, the trigger runner's `TriggerFiring` audit row records
    /// the failure (status="error", error_message=cause). The next fire
    /// (whether trigger-driven or read-driven) re-runs WASM on the latest
    /// input — which is the right behavior since the input may have
    /// changed.
    pub async fn resolve_with_overrides_and_derived(
        &self,
        view: &TransformView,
        requested_fields: &[String],
        source_query: &dyn SourceQueryFn,
        overrides: &[(String, String, TransformFieldOverride)],
    ) -> Result<
        (
            HashMap<String, HashMap<KeyValue, FieldValue>>,
            Option<DerivedMetadata>,
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

        // MDT-F Phase 2 — runtime envelope rejection.
        //
        // If the transform has a calibrated gas model, reject inputs that
        // fall outside the envelope BEFORE entering the WASM. This is
        // distinct from `GasExceeded` (which is a runtime fuel trap): no
        // fuel is burned here, and the failure is deterministic across
        // devices because the measurement is over the input JSON shape,
        // which every replayer sees identically.
        if let Some(spec) = &view.wasm_transform {
            if let Some(model) = &spec.gas_model {
                let measured = measure_input(&all_query_results, &model.coefficients);
                if measured > model.max_input_size {
                    return Err(SchemaError::InvalidTransform(format!(
                        "View '{}' unavailable: input exceeds calibrated envelope (measured={}, limit={})",
                        view.name, measured, model.max_input_size
                    )));
                }
            }
        }

        // Compute output. WASM failures surface as `InvalidTransform` so
        // the trigger runner records them as `status="error"` rows in the
        // `TriggerFiring` audit log; downstream (cron / re-fire) decides
        // whether to retry.
        //
        // Only WASM views produce `DerivedMetadata` — identity views are
        // pass-through and have no derivation to record.
        let (mut output, derived) = if let Some(spec) = &view.wasm_transform {
            match self.execute_wasm_transform(&spec.bytes, spec.max_gas, &all_query_results) {
                Ok(output) => {
                    let metadata = compute_derived_metadata(&spec.bytes, &all_query_results);
                    (output, Some(metadata))
                }
                Err(e) => {
                    return Err(SchemaError::InvalidTransform(wasm_failure_cause(
                        &view.name, &e,
                    )));
                }
            }
        } else {
            (self.identity_pass_through(&all_query_results, view)?, None)
        };

        // Apply overrides BEFORE type validation so the user-supplied value
        // is what we validate. Overrides also extend the output to fields
        // that the source path produced no entries for (overrides are
        // sticky regardless of source state).
        Self::apply_overrides_to_field_map(&mut output, view, overrides);

        // Validate output against declared types. A mismatch is a per-input
        // failure of the transform (the WASM produced a wrongly-shaped
        // value for this input) — same `InvalidTransform` error path as a
        // runtime trap.
        for (field_name, field_type) in &view.output_fields {
            if let Some(field_entries) = output.get(field_name) {
                for fv in field_entries.values() {
                    if let Err(e) = field_type.validate(&fv.value) {
                        return Err(SchemaError::InvalidTransform(format!(
                            "View '{}' unavailable: output field '{}' type validation failed: {}",
                            view.name, field_name, e
                        )));
                    }
                }
            }
        }

        // Return only requested fields
        let mut result = HashMap::new();
        for field_name in &fields_to_return {
            let field_entries = output.get(field_name).cloned().unwrap_or_default();
            result.insert(field_name.clone(), field_entries);
        }

        Ok((result, derived))
    }

    /// Substitute overrides into the freshly-computed output map.
    /// Honors the view's declared `output_fields` — overrides for fields
    /// not declared on the view are ignored (defensive: a stale override
    /// left over from a removed field shouldn't materialize). Overrides
    /// for declared fields are inserted even if the source produced no
    /// entries, so the override survives a stale source link.
    fn apply_overrides_to_field_map(
        output: &mut HashMap<String, HashMap<KeyValue, FieldValue>>,
        view: &TransformView,
        overrides: &[(String, String, TransformFieldOverride)],
    ) {
        for (field_name, key_str, override_mol) in overrides {
            if !view.output_fields.contains_key(field_name) {
                continue;
            }
            let key = parse_key_str(key_str);
            let fv = override_field_value(override_mol);
            output
                .entry(field_name.clone())
                .or_default()
                .insert(key, fv);
        }
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
    ///
    /// `max_gas` is the system-wide per-invocation fuel ceiling (MDT-E):
    /// the engine seeds the `Store`'s fuel counter to exactly this value
    /// before entering the guest, and fuel exhaustion surfaces as
    /// [`SchemaError::TransformGasExceeded`] which the caller maps to
    /// the `gas exceeded` cause string via [`wasm_failure_cause`].
    fn execute_wasm_transform(
        &self,
        wasm_bytes: &[u8],
        max_gas: u64,
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
        let output_json = self.wasm_engine.execute(wasm_bytes, &input_json, max_gas)?;

        // Parse output JSON: { "fields": { field_name: { key: value } } }
        let fields_obj = output_json
            .get("fields")
            .and_then(|f| f.as_object())
            .ok_or_else(|| {
                SchemaError::InvalidData(
                    "WASM transform output must have a 'fields' object".to_string(),
                )
            })?;

        // Build a transient FieldValue carrying just the WASM-produced
        // value. The orchestrator's dual-write code reads `fv.value` to
        // build a `Provenance::Derived` mutation; the rest of the FieldValue
        // shape (atom_uuid / molecule_uuid / writer_pubkey / written_at) is
        // populated downstream by `MutationManager::write_mutations_batch_async`
        // when atoms land. The blank fields here NEVER reach a reader —
        // PR 5's reader flip serves real atom-store FieldValues.
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
                    writer_pubkey: None,
                    written_at: None,
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

/// Compute the measured input size for the runtime envelope check
/// (MDT-F Phase 2).
///
/// Iterates the gas-model coefficient list and sums a per-dimension size
/// across `all_query_results`. The coefficient *weights* are irrelevant
/// for the envelope check — only the dimension identity matters — so
/// they are ignored here; budget derivation (Phase 2 task 3/3) is a
/// separate calculation that does use them.
///
/// Per-dimension measurement:
/// * [`InputDimension::FieldBytes`]: sum of `serde_json::to_vec(value).len()`
///   over every `(key, value)` entry on the named `(schema, field)`.
///   Serialization failures fall back to `0` rather than panicking — the
///   envelope check must stay permissive for values that can still be
///   passed to the WASM.
/// * [`InputDimension::FieldCount`]: number of `(key, value)` entries on
///   the named `(schema, field)`. A plain row count is the intuitive
///   meaning of "count" at the view-input level and composes with
///   `FieldBytes` when a transform is priced on both axes.
///
/// The return type is `u64` so the sum can be compared directly against
/// [`crate::view::types::GasModel::max_input_size`].
fn measure_input(
    all_query_results: &HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>>,
    coefficients: &[(InputDimension, f64)],
) -> u64 {
    let mut total: u64 = 0;
    for (dim, _weight) in coefficients {
        let (field_id, is_bytes) = match dim {
            InputDimension::FieldBytes(id) => (id, true),
            InputDimension::FieldCount(id) => (id, false),
        };
        let Some(schema_map) = all_query_results.get(&field_id.schema) else {
            continue;
        };
        let Some(field_entries) = schema_map.get(&field_id.field) else {
            continue;
        };
        if is_bytes {
            for fv in field_entries.values() {
                let n = serde_json::to_vec(&fv.value)
                    .map(|v| v.len() as u64)
                    .unwrap_or(0);
                total = total.saturating_add(n);
            }
        } else {
            total = total.saturating_add(field_entries.len() as u64);
        }
    }
    total
}

/// Reverse of `KeyValue::Display`: `"hash:range"` → hash+range,
/// `"hash"` → hash-only, `""` → both `None`. Range-only encoding (the
/// default for view output) is the common case here.
fn parse_key_str(s: &str) -> KeyValue {
    if s.is_empty() {
        KeyValue::new(None, None)
    } else if let Some((hash, range)) = s.split_once(':') {
        KeyValue::new(Some(hash.to_string()), Some(range.to_string()))
    } else {
        // Single-segment keys are ambiguous between hash-only and range-only;
        // view output is keyed by range, and override writes are stored using
        // `KeyValue::Display` which omits the colon when only `range` is set.
        // So a bare segment here came from a range-only key.
        KeyValue::new(None, Some(s.to_string()))
    }
}

/// Convert an override molecule into a `FieldValue` suitable for view output.
/// Override molecules don't have an underlying atom so atom-level provenance
/// fields stay empty; the override's `writer_pubkey` is preserved so callers
/// can attribute the value.
fn override_field_value(o: &TransformFieldOverride) -> FieldValue {
    FieldValue {
        value: o.value.clone(),
        atom_uuid: String::new(),
        source_file_name: None,
        metadata: None,
        molecule_uuid: None,
        molecule_version: None,
        writer_pubkey: if o.writer_pubkey.is_empty() {
            None
        } else {
            Some(o.writer_pubkey.clone())
        },
        written_at: None,
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
            writer_pubkey: None,
            written_at: None,
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
    async fn test_resolve_identity_pass_through() {
        let resolver = make_resolver();
        let view = make_identity_view();

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

        let results = resolver
            .resolve(&view, &["content".to_string()], &mock)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        let (key, fv) = results["content"].iter().next().unwrap();
        assert_eq!(key.range.as_deref(), Some("2026-01-01"));
        assert_eq!(fv.value, serde_json::json!("hello world"));
    }

    #[tokio::test]
    async fn test_resolve_unknown_field_errors() {
        let resolver = make_resolver();
        let view = make_identity_view();
        let mock = MockSourceQuery {
            results: HashMap::new(),
        };

        let result = resolver
            .resolve(&view, &["nonexistent".to_string()], &mock)
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

        let results = resolver.resolve(&view, &[], &mock).await.unwrap();

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

        let results = resolver.resolve(&view, &[], &mock).await.unwrap();

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

        // Source returns empty field results
        let mut blogpost_fields = HashMap::new();
        blogpost_fields.insert("content".to_string(), HashMap::new());

        let mut results_map = HashMap::new();
        results_map.insert("BlogPost".to_string(), blogpost_fields);

        let mock = MockSourceQuery {
            results: results_map,
        };

        let results = resolver
            .resolve(&view, &["content".to_string()], &mock)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results["content"].is_empty());
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

        let results = resolver.resolve(&view, &[], &mock).await.unwrap();

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
    async fn test_type_validation_failure_is_invalid_transform() {
        // Type-validation failure surfaces as `InvalidTransform` so the
        // trigger runner records `status="error"` in the audit log.
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

        let err = resolver
            .resolve(&view, &["count".to_string()], &mock)
            .await
            .expect_err("type validation failure should error");
        let msg = err.to_string();
        assert!(
            msg.contains("type validation failed"),
            "expected type validation failure, got: {}",
            msg
        );
    }
}
