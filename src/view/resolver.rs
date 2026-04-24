use crate::schema::types::errors::SchemaError;
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::operations::Query;
use crate::view::derived_metadata::{compute_derived_metadata, DerivedMetadata};
use crate::view::transform_field_override::TransformFieldOverride;
use crate::view::types::{InputDimension, TransformView, UnavailableReason, ViewCacheState};
use crate::view::wasm_engine::WasmTransformEngine;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Classify a `SchemaError` produced during WASM transform execution into
/// the corresponding [`UnavailableReason`]. Compile-time errors surface as
/// `CompileError`; fuel exhaustion (MDT-E) surfaces as `GasExceeded` with
/// the recorded `input_size`; everything else from the WASM path is
/// `ExecutionError` (including output parse failures and type-validation
/// mismatches, which are downstream of the transform's runtime behavior).
///
/// Registry-fetch classification (`TransformBytesUnavailable`) is wired in
/// by the transform resolver work; that variant exists so the state
/// machine is complete but isn't reachable from today's code path.
fn classify_wasm_failure(err: &SchemaError) -> UnavailableReason {
    match err {
        SchemaError::TransformGasExceeded { input_size } => UnavailableReason::GasExceeded {
            input_size: *input_size,
        },
        SchemaError::InvalidTransform(msg) => {
            if msg.starts_with("Failed to compile WASM module") {
                UnavailableReason::CompileError {
                    message: msg.clone(),
                }
            } else {
                UnavailableReason::ExecutionError {
                    message: msg.clone(),
                }
            }
        }
        other => UnavailableReason::ExecutionError {
            message: other.to_string(),
        },
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
        self.resolve_with_overrides(view, requested_fields, cache_state, source_query, &[])
            .await
    }

    /// Same as `resolve`, but applies any per-(field, key) overrides on top
    /// of the computed output. Overrides are looked up from the override
    /// store by the caller and passed in. When an override exists for a
    /// `(field, key)` it supersedes whatever the WASM/identity path produced
    /// — and the override survives even if the source link is stale, which
    /// is what makes `Overridden` sticky against subsequent source mutations.
    pub async fn resolve_with_overrides(
        &self,
        view: &TransformView,
        requested_fields: &[String],
        cache_state: &ViewCacheState,
        source_query: &dyn SourceQueryFn,
        overrides: &[(String, String, TransformFieldOverride)],
    ) -> Result<
        (
            HashMap<String, HashMap<KeyValue, FieldValue>>,
            ViewCacheState,
        ),
        SchemaError,
    > {
        let (result, state, _derived) = self
            .resolve_with_overrides_and_derived(
                view,
                requested_fields,
                cache_state,
                source_query,
                overrides,
            )
            .await?;
        Ok((result, state))
    }

    /// Like [`resolve_with_overrides`] but additionally returns the
    /// [`DerivedMetadata`] for the just-executed fire.
    ///
    /// `DerivedMetadata` is `Some` only when the resolver actually executed a
    /// WASM transform on fresh input and produced a `ViewCacheState::Cached`
    /// result. It is `None` when we took the cached short-circuit, when the
    /// view is identity pass-through (no derivation happened), or when the
    /// fire ended in `Unavailable`. Callers use the returned metadata to
    /// build `Provenance::Derived` mutations downstream
    /// (`projects/view-compute-as-mutations` PR 2).
    pub async fn resolve_with_overrides_and_derived(
        &self,
        view: &TransformView,
        requested_fields: &[String],
        cache_state: &ViewCacheState,
        source_query: &dyn SourceQueryFn,
        overrides: &[(String, String, TransformFieldOverride)],
    ) -> Result<
        (
            HashMap<String, HashMap<KeyValue, FieldValue>>,
            ViewCacheState,
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

        // Check cache
        if let ViewCacheState::Cached { entries } = cache_state {
            let mut result = HashMap::new();
            for field_name in &fields_to_return {
                let field_entries = entries.get(field_name).cloned().unwrap_or_default();
                result.insert(field_name.clone(), field_entries.into_iter().collect());
            }
            // Cached path still consults overrides — overrides are sticky and
            // must beat anything in the per-view cache too.
            self.apply_overrides(&mut result, overrides);
            return Ok((result, cache_state.clone(), None));
        }

        // Sticky-per-input: if the prior compute on this input already
        // failed, don't retry. Return the same Unavailable state so the
        // caller can surface the reason without re-running the transform.
        // Invalidation (source mutation) clears this back to Empty.
        if let ViewCacheState::Unavailable { reason } = cache_state {
            return Ok((
                HashMap::new(),
                ViewCacheState::Unavailable {
                    reason: reason.clone(),
                },
                None,
            ));
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
                    return Ok((
                        HashMap::new(),
                        ViewCacheState::Unavailable {
                            reason: UnavailableReason::ExceedsCalibratedEnvelope {
                                measured,
                                limit: model.max_input_size,
                            },
                        },
                        None,
                    ));
                }
            }
        }

        // Compute output. WASM failures become an `Unavailable` state
        // rather than a hard error: they are per-input compute failures,
        // so the caller should persist the state (no retry) but callers
        // above the resolver are free to surface it to the user.
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
                    let reason = classify_wasm_failure(&e);
                    return Ok((HashMap::new(), ViewCacheState::Unavailable { reason }, None));
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
        // failure of the transform (the WASM produced a wrongly-shaped value
        // for this input) — same Unavailable semantics as an execution trap.
        for (field_name, field_type) in &view.output_fields {
            if let Some(field_entries) = output.get(field_name) {
                for fv in field_entries.values() {
                    if let Err(e) = field_type.validate(&fv.value) {
                        let reason = UnavailableReason::ExecutionError {
                            message: format!(
                                "View '{}' output field '{}' type validation failed: {}",
                                view.name, field_name, e
                            ),
                        };
                        return Ok((HashMap::new(), ViewCacheState::Unavailable { reason }, None));
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

        Ok((result, new_cache, derived))
    }

    /// Substitute override values into an already-shaped result map.
    /// Used by the cached short-circuit where we don't have the full
    /// `output_fields` typing context and only care about the requested
    /// subset.
    fn apply_overrides(
        &self,
        result: &mut HashMap<String, HashMap<KeyValue, FieldValue>>,
        overrides: &[(String, String, TransformFieldOverride)],
    ) {
        for (field_name, key_str, override_mol) in overrides {
            if let Some(field_entries) = result.get_mut(field_name) {
                let key = parse_key_str(key_str);
                let fv = override_field_value(override_mol);
                field_entries.insert(key, fv);
            }
        }
    }

    /// Substitute overrides into the freshly-computed output map. Unlike
    /// `apply_overrides`, this version honors the view's declared
    /// `output_fields` — overrides for fields not declared on the view are
    /// ignored (defensive: a stale override left over from a removed field
    /// shouldn't materialize). Overrides for declared fields are inserted
    /// even if the source produced no entries, so the override survives a
    /// stale source link.
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
    /// [`UnavailableReason::GasExceeded`] via
    /// [`classify_wasm_failure`].
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
    async fn test_type_validation_failure_becomes_unavailable() {
        // Type-validation failure is a per-input compute failure — the
        // transform (or identity pass-through) produced a wrongly-shaped
        // value for this input. Resolver surfaces that as Unavailable
        // (sticky per input) rather than a hard error.
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

        let (results, new_cache) = resolver
            .resolve(&view, &["count".to_string()], &cache, &mock)
            .await
            .expect("resolve should return Ok with Unavailable on compute failure");
        assert!(results.is_empty());
        let reason = new_cache
            .unavailable_reason()
            .expect("cache should be Unavailable");
        assert!(matches!(reason, UnavailableReason::ExecutionError { .. }));
        assert!(reason.to_string().contains("type validation failed"));
    }

    #[tokio::test]
    async fn test_unavailable_input_does_not_retry() {
        // When the cache is already Unavailable, resolve must not re-execute
        // the source queries or the WASM — it returns the same state so the
        // caller can surface the reason without burning cycles.
        let resolver = make_resolver();
        let view = make_identity_view();

        let reason = UnavailableReason::GasExceeded { input_size: 500 };
        let cache = ViewCacheState::Unavailable {
            reason: reason.clone(),
        };

        // Mock returns NotFound for any schema; if resolve mistakenly
        // touches the source_query, we'd get an Err instead of Ok.
        let mock = MockSourceQuery {
            results: HashMap::new(),
        };

        let (results, new_cache) = resolver
            .resolve(&view, &["content".to_string()], &cache, &mock)
            .await
            .expect("Unavailable should short-circuit to Ok");
        assert!(results.is_empty());
        assert_eq!(new_cache.unavailable_reason(), Some(&reason));
    }
}
