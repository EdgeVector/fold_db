use crate::schema::types::errors::SchemaError;
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::operations::Query;
use crate::view::derived_metadata::{compute_derived_metadata, DerivedMetadata};
use crate::view::transform_field_override::TransformFieldOverride;
use crate::view::types::{InputDimension, TransformView};
use crate::view::wasm_engine::WasmTransformEngine;
use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Per-firing input envelope captured by the resolver alongside the
/// view's output. Used by the trigger runner's `TriggerFiring` audit
/// row to record the exact input the WASM saw at firing time (TH6a).
///
/// The shape mirrors the resolver's intermediate `all_query_results`
/// (`schema -> field -> KeyValue -> FieldValue`) so the orchestrator
/// can serialize it into the spec's `BTreeMap<String, Vec<Value>>`
/// per-record snapshot form without needing the resolver to know
/// about that representation.
pub type CapturedEnvelope = HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>>;

/// Successful resolver output: per-field results plus optional
/// derived-mutation metadata (only populated for WASM views).
type ResolveResult = (
    HashMap<String, HashMap<KeyValue, FieldValue>>,
    Option<DerivedMetadata>,
);

/// Result returned from [`ViewResolver::resolve_with_snapshot`].
///
/// Always carries the captured input envelope (gathered before any
/// WASM execution) so a failed transform still records what input was
/// seen — that's the whole point of capturing on errors. The inner
/// `result` is the same `Result` shape `resolve_with_overrides_and_derived`
/// returns, so callers that don't care about the snapshot can ignore
/// `envelope` and treat this exactly like the older entry point.
pub struct SnapshotResolveOutcome {
    /// Inputs assembled from `view.input_queries` before WASM ran.
    pub envelope: CapturedEnvelope,
    /// The resolver's output and (for WASM views) derived-metadata.
    /// `Err` when WASM/identity/type-validation failed; the envelope
    /// is still populated.
    pub result: Result<ResolveResult, SchemaError>,
}

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
    ) -> Result<ResolveResult, SchemaError> {
        let outcome = self
            .resolve_with_snapshot(view, requested_fields, source_query, overrides)
            .await;
        outcome.result
    }

    /// Like [`resolve_with_overrides_and_derived`], but additionally
    /// returns the per-firing input envelope assembled from
    /// `view.input_queries`. TH6a spec §1, §3 — the envelope is what the
    /// `TriggerFiring` audit row's `input_snapshot` field captures.
    ///
    /// The envelope is gathered BEFORE WASM runs, so callers always get
    /// it back — even when the inner `result` is `Err` (gas exceeded,
    /// trap, type validation failure, calibrated envelope rejection).
    /// That's the whole point: error capture needs the input the WASM
    /// saw, regardless of whether WASM succeeded.
    ///
    /// Existing call sites stay on [`resolve_with_overrides_and_derived`]
    /// (and friends) since they don't need the envelope; only the
    /// firing-capture path on `view_orchestrator::fire_view` calls this
    /// new entry point.
    pub async fn resolve_with_snapshot(
        &self,
        view: &TransformView,
        requested_fields: &[String],
        source_query: &dyn SourceQueryFn,
        overrides: &[(String, String, TransformFieldOverride)],
    ) -> SnapshotResolveOutcome {
        // Pre-query validation of requested fields. Has to land in
        // SnapshotResolveOutcome.result rather than panicking — even
        // though there's no envelope to capture yet at this point, the
        // contract says we always return an outcome.
        let fields_to_return: Vec<String> = if requested_fields.is_empty() {
            view.output_fields.keys().cloned().collect()
        } else {
            for field_name in requested_fields {
                if !view.output_fields.contains_key(field_name) {
                    return SnapshotResolveOutcome {
                        envelope: HashMap::new(),
                        result: Err(SchemaError::InvalidField(format!(
                            "Field '{}' not found in view '{}'",
                            field_name, view.name
                        ))),
                    };
                }
            }
            requested_fields.to_vec()
        };

        // Execute all input queries, merging results when multiple
        // queries target the same schema. This is the envelope.
        let mut envelope: CapturedEnvelope = HashMap::new();
        for query in &view.input_queries {
            match source_query.execute_query(query).await {
                Ok(query_results) => {
                    envelope
                        .entry(query.schema_name.clone())
                        .or_default()
                        .extend(query_results);
                }
                Err(e) => {
                    return SnapshotResolveOutcome {
                        envelope,
                        result: Err(e),
                    };
                }
            }
        }

        let result =
            self.compute_output_from_envelope(view, &fields_to_return, &envelope, overrides);
        SnapshotResolveOutcome { envelope, result }
    }

    /// Run envelope check, WASM/identity, override merge, and type
    /// validation against an already-gathered envelope. Extracted so
    /// both the legacy and snapshot resolve paths share the same logic.
    fn compute_output_from_envelope(
        &self,
        view: &TransformView,
        fields_to_return: &[String],
        envelope: &CapturedEnvelope,
        overrides: &[(String, String, TransformFieldOverride)],
    ) -> Result<ResolveResult, SchemaError> {
        // MDT-F Phase 2 — runtime envelope rejection.
        if let Some(spec) = &view.wasm_transform {
            if let Some(model) = &spec.gas_model {
                let measured = measure_input(envelope, &model.coefficients);
                if measured > model.max_input_size {
                    return Err(SchemaError::InvalidTransform(format!(
                        "View '{}' unavailable: input exceeds calibrated envelope (measured={}, limit={})",
                        view.name, measured, model.max_input_size
                    )));
                }
            }
        }

        // Compute output. WASM failures surface as `InvalidTransform`.
        let (mut output, derived) = if let Some(spec) = &view.wasm_transform {
            match self.execute_wasm_transform(&spec.bytes, spec.max_gas, envelope) {
                Ok(output) => {
                    let metadata = compute_derived_metadata(&spec.bytes, envelope);
                    (output, Some(metadata))
                }
                Err(e) => {
                    return Err(SchemaError::InvalidTransform(wasm_failure_cause(
                        &view.name, &e,
                    )));
                }
            }
        } else {
            (self.identity_pass_through(envelope, view)?, None)
        };

        Self::apply_overrides_to_field_map(&mut output, view, overrides);

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

        let mut result = HashMap::new();
        for field_name in fields_to_return {
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

/// TH6a spec §1 — global cap on the snapshot's serialized size.
/// 10 MiB. Exposed for tests that want to override the cap.
pub const SNAPSHOT_MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;

/// Build the per-firing snapshot envelope from a [`CapturedEnvelope`],
/// applying TH6a spec §1 truncation: serialize records in deterministic
/// order (schemas sorted lexicographically, records within each schema
/// sorted by their primary key), drop whole records once `max_bytes`
/// would be exceeded.
///
/// The output shape is `{ schema_name: [ record1, record2, ... ] }`
/// where each record is a JSON object mapping field names to values
/// for a single primary key. This is the spec's
/// `BTreeMap<String, Vec<Value>>` shape, distinct from the WASM input
/// shape (which is `schema -> field -> { key: value }`).
///
/// Returns `(snapshot_value, truncated)` where `truncated` is `true`
/// iff at least one record was dropped to fit `max_bytes`.
pub fn build_snapshot_envelope(
    envelope: &CapturedEnvelope,
    max_bytes: usize,
) -> (serde_json::Value, bool) {
    // 1. Pivot field -> key -> value into key -> { field: value } per
    //    schema, so each "record" is the row's JSON form.
    // 2. Sort schemas lexicographically; sort records within each
    //    schema by `KeyValue::to_string`. Both are spec-required for
    //    deterministic truncation reproducibility.
    // 3. Append records in order, accumulating serialized bytes;
    //    drop the rest when the next record would exceed the cap.
    let mut schemas_sorted: Vec<&String> = envelope.keys().collect();
    schemas_sorted.sort();

    let mut snapshot = serde_json::Map::new();
    let mut total_bytes: usize = 0;
    let mut truncated = false;
    let mut overflowed = false;

    for schema_name in schemas_sorted {
        let fields_map = envelope.get(schema_name).expect("schema in sorted list");

        // Pivot to records: key -> { field_name: value }
        let mut records_by_key: BTreeMap<String, serde_json::Map<String, serde_json::Value>> =
            BTreeMap::new();
        for (field_name, entries) in fields_map {
            for (key, fv) in entries {
                let key_str = key.to_string();
                records_by_key
                    .entry(key_str)
                    .or_default()
                    .insert(field_name.clone(), fv.value.clone());
            }
        }

        let mut records_for_schema: Vec<serde_json::Value> = Vec::new();
        for (_key, record) in records_by_key.into_iter() {
            if overflowed {
                truncated = true;
                continue;
            }
            let record_value = serde_json::Value::Object(record);
            let record_bytes = serde_json::to_vec(&record_value)
                .map(|v| v.len())
                .unwrap_or(0);
            // Conservative: account for record bytes plus a few
            // bytes of separator/structural overhead. The check is
            // approximate — we want to leave headroom rather than
            // produce a snapshot that overshoots the cap.
            if total_bytes.saturating_add(record_bytes) > max_bytes {
                truncated = true;
                overflowed = true;
                continue;
            }
            total_bytes = total_bytes.saturating_add(record_bytes);
            records_for_schema.push(record_value);
        }

        // Always emit the per-schema list — empty arrays are still
        // valid JSON and preserve the schema's name in the snapshot.
        snapshot.insert(
            schema_name.clone(),
            serde_json::Value::Array(records_for_schema),
        );
    }

    (serde_json::Value::Object(snapshot), truncated)
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

    #[test]
    fn build_snapshot_envelope_orders_schemas_lexicographically() {
        // TH6a spec §1 — schemas sorted by name; records sorted by
        // primary key. Determinism makes truncation reproducible.
        let mut envelope: CapturedEnvelope = HashMap::new();
        for schema in ["Z_late", "A_early", "M_mid"] {
            let mut field_entries = HashMap::new();
            field_entries.insert(
                KeyValue::new(None, Some(format!("{}_k", schema))),
                make_field_value(serde_json::json!({"v": schema})),
            );
            let mut fields = HashMap::new();
            fields.insert("v".to_string(), field_entries);
            envelope.insert(schema.to_string(), fields);
        }
        let (snapshot, truncated) = build_snapshot_envelope(&envelope, SNAPSHOT_MAX_INPUT_BYTES);
        assert!(!truncated);

        let obj = snapshot.as_object().expect("snapshot is JSON object");
        let keys: Vec<&String> = obj.keys().collect();
        assert_eq!(keys, vec!["A_early", "M_mid", "Z_late"]);
    }

    #[test]
    fn build_snapshot_envelope_orders_records_by_primary_key() {
        let mut envelope: CapturedEnvelope = HashMap::new();
        let mut field_entries = HashMap::new();
        for k in ["c_third", "a_first", "b_second"] {
            field_entries.insert(
                KeyValue::new(None, Some(k.into())),
                make_field_value(serde_json::json!({"k": k})),
            );
        }
        let mut fields = HashMap::new();
        fields.insert("rec".to_string(), field_entries);
        envelope.insert("S".to_string(), fields);

        let (snapshot, truncated) = build_snapshot_envelope(&envelope, SNAPSHOT_MAX_INPUT_BYTES);
        assert!(!truncated);

        let arr = snapshot["S"]
            .as_array()
            .expect("schema entries are JSON array");
        assert_eq!(arr.len(), 3);
        // Records appear in sorted-by-key order; values match.
        let key_order: Vec<&str> = arr
            .iter()
            .map(|r| r["rec"].as_object().unwrap()["k"].as_str().unwrap())
            .collect();
        assert_eq!(key_order, vec!["a_first", "b_second", "c_third"]);
    }

    #[test]
    fn build_snapshot_envelope_truncates_whole_records_at_cap() {
        // Per spec §1: drop whole records once the next record would
        // exceed `max_bytes`. The snapshot must remain valid JSON
        // (no mid-string truncation) and `truncated = true`.
        let mut envelope: CapturedEnvelope = HashMap::new();
        let mut field_entries = HashMap::new();
        let big_string = "x".repeat(2_000); // ~2KB serialized record
        for i in 0..20 {
            // Use zero-padded keys so sort order is stable and predictable.
            field_entries.insert(
                KeyValue::new(None, Some(format!("k{:02}", i))),
                make_field_value(serde_json::json!({"data": big_string})),
            );
        }
        let mut fields = HashMap::new();
        fields.insert("data".to_string(), field_entries);
        envelope.insert("S".to_string(), fields);

        // Cap at ~5KB — should fit roughly 2 records before truncating.
        let (snapshot, truncated) = build_snapshot_envelope(&envelope, 5_000);
        assert!(truncated, "expected truncation flag");

        let arr = snapshot["S"].as_array().unwrap();
        assert!(
            arr.len() < 20,
            "some records must be dropped; got {}",
            arr.len()
        );
        // Snapshot must serialize as valid JSON — confirms no mid-record
        // truncation happened.
        let _round_trip: serde_json::Value =
            serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
    }

    #[test]
    fn build_snapshot_envelope_no_truncation_when_under_cap() {
        let mut envelope: CapturedEnvelope = HashMap::new();
        let mut entries = HashMap::new();
        entries.insert(
            KeyValue::new(None, Some("k1".into())),
            make_field_value(serde_json::json!("v1")),
        );
        let mut fields = HashMap::new();
        fields.insert("f".to_string(), entries);
        envelope.insert("S".to_string(), fields);

        let (_snapshot, truncated) = build_snapshot_envelope(&envelope, SNAPSHOT_MAX_INPUT_BYTES);
        assert!(!truncated);
    }

    #[tokio::test]
    async fn resolve_with_snapshot_returns_envelope_on_success() {
        // TH6a — the snapshot path returns the captured envelope plus
        // the regular result. Identity views go through the same path.
        let resolver = make_resolver();
        let view = make_identity_view();

        let mut entries = HashMap::new();
        entries.insert(
            KeyValue::new(None, Some("2026-01-01".into())),
            make_field_value(serde_json::json!("hello")),
        );
        let mut blogpost_fields = HashMap::new();
        blogpost_fields.insert("content".to_string(), entries);
        let mut results_map = HashMap::new();
        results_map.insert("BlogPost".to_string(), blogpost_fields);

        let mock = MockSourceQuery {
            results: results_map,
        };

        let outcome = resolver
            .resolve_with_snapshot(&view, &["content".to_string()], &mock, &[])
            .await;
        assert!(outcome.result.is_ok());
        // Envelope carries the source data we provided.
        assert!(outcome.envelope.contains_key("BlogPost"));
        let bp = &outcome.envelope["BlogPost"];
        assert!(bp.contains_key("content"));
    }

    #[tokio::test]
    async fn resolve_with_snapshot_returns_envelope_even_on_error() {
        // Type-validation failure happens AFTER queries — the envelope
        // is still populated when the inner result is `Err`. This is
        // what enables ErrorsOnly capture.
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
            make_field_value(serde_json::json!("not_a_number")),
        );
        let mut blogpost_fields = HashMap::new();
        blogpost_fields.insert("count".to_string(), count_values);
        let mut results_map = HashMap::new();
        results_map.insert("BlogPost".to_string(), blogpost_fields);

        let mock = MockSourceQuery {
            results: results_map,
        };

        let outcome = resolver
            .resolve_with_snapshot(&view, &["count".to_string()], &mock, &[])
            .await;
        assert!(outcome.result.is_err());
        assert!(
            outcome.envelope.contains_key("BlogPost"),
            "envelope must be populated even when WASM/type validation fails"
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
