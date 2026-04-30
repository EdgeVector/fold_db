use crate::schema::types::field_value_type::FieldValueType;
use crate::schema::types::key_config::KeyConfig;
use crate::schema::types::operations::Query;
use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
use crate::triggers::types::Trigger;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Fully-qualified reference to a field on a source schema. Used by
/// [`InputDimension`] to name the input slice a gas-model coefficient is
/// measured against.
///
/// Mirrored from the schema-service gas-model fit output (MDT-F Phase 1);
/// fold_db owns the canonical shape now and schema_service will adopt it in
/// a follow-up so a future merge is trivial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldId {
    pub schema: String,
    pub field: String,
}

/// A single input dimension the gas-model fit measures against. The
/// coefficient weights themselves are irrelevant for the runtime envelope
/// check (MDT-F Phase 2) — we only need to know which `(schema, field)`
/// slices of input to size — but they are carried through for the later
/// fuel-budget derivation (MDT-F Phase 2 task 3/3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputDimension {
    /// Sum of JSON-encoded byte length over every entry in the named
    /// field. Used when the transform's cost scales with content bulk.
    FieldBytes(FieldId),
    /// Row count (number of `(key, value)` entries) on the named field.
    /// Used when the transform's cost scales with cardinality.
    FieldCount(FieldId),
}

/// Calibrated gas model fit for a transform. Produced once by the
/// schema-service fit harness (MDT-F Phase 1) and carried forward on the
/// [`WasmTransformSpec`]. The runtime uses it two ways:
///
/// 1. **Envelope rejection** (Phase 2 task 2/3 — this PR): before executing
///    the WASM, sum the measured sizes along each [`InputDimension`] and
///    reject with `InvalidTransform("... exceeds calibrated envelope ...")`
///    if the total exceeds `max_input_size`. No fuel is burned on rejection.
/// 2. **Budget derivation** (Phase 2 task 3/3 — follow-up): derive the
///    per-invocation `max_gas` from `base + Σ coefficient_i * size_i`.
///
/// `coefficients` is ordered to match the schema-service fit output; the
/// runtime iterates it as a flat list and does not rely on order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GasModel {
    /// Constant overhead added to the fit regardless of input shape.
    pub base: u64,
    /// Per-dimension coefficients as `(dimension, weight)`. Weight is an
    /// f64 because the fit output is real-valued; runtime envelope check
    /// ignores the weight and uses only the dimension identity.
    pub coefficients: Vec<(InputDimension, f64)>,
    /// Upper bound on `Σ size_i(input)` summed across every dimension.
    /// Inputs above this ceiling are rejected pre-execution so we stay
    /// inside the calibrated regime where the fit is trustworthy.
    pub max_input_size: u64,
}

/// A WASM transform attached to a view: compiled bytes + the per-invocation
/// fuel ceiling enforced on every device that executes it.
///
/// `max_gas` is required — the whole point of MDT-E is that a given
/// `(transform, input)` pair must either succeed on every device or
/// surface `InvalidTransform("... gas exceeded ...")` on every device.
/// Allowing a missing budget would let one device silently skip fuel metering and
/// produce state that other devices can't reproduce. The schema service
/// enforces `0 < max_gas <= 10^18` at registration (see
/// `schema_service_core::state_transforms::MAX_GAS_CEILING`).
///
/// `gas_model` is optional because views registered before MDT-F Phase 1
/// have no fit. When present, the runtime enforces the calibrated
/// [`GasModel::max_input_size`] envelope pre-execution (MDT-F Phase 2) and
/// — in a later task — derives `max_gas` from the coefficients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmTransformSpec {
    /// Compiled WASM module bytes.
    pub bytes: Vec<u8>,
    /// Per-invocation fuel ceiling. Wasmtime fuel is set to exactly this
    /// value on every `Store` before `transform` is called.
    pub max_gas: u64,
    /// Calibrated gas model from the MDT-F Phase 1 fit harness. `None`
    /// for views that predate the fit — those views skip envelope
    /// rejection and run under `max_gas` alone. `#[serde(default)]` so
    /// existing persisted views deserialize without migration.
    #[serde(default)]
    pub gas_model: Option<GasModel>,
}

/// What inputs to capture into the `TriggerFiring` audit row's
/// `input_snapshot` field. TH6a spec §5–6.
///
/// Default for existing serialized views is `Off` (preserves prior
/// behavior — no snapshot capture). New views constructed via the
/// schema_service client builder default to `ErrorsOnly`; raw API
/// consumers who omit the field also get `Off`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FiringCaptureMode {
    /// Never capture an input snapshot.
    #[default]
    Off,
    /// Capture only when the firing's outcome is `Error`.
    ErrorsOnly,
    /// Capture for every firing regardless of outcome.
    All,
}

/// Per-bucket retention caps for `TriggerFiring` rows belonging to a
/// single view. TH6a spec §3.
///
/// Buckets are defined as views over the firing-row state:
/// * `errors_with_snapshot` — `outcome=Error AND input_snapshot IS NOT NULL`
/// * `successes_with_snapshot` — `outcome=Success AND input_snapshot IS NOT NULL`
/// * `metadata_only` — `input_snapshot IS NULL` (regardless of outcome)
///
/// When a snapshot bucket exceeds its cap, the oldest row's snapshot
/// fields are nulled (the row stays as a metadata-only entry); when the
/// metadata bucket exceeds its cap, the oldest row is deleted entirely.
/// All zeros (the deserialize default) disables retention — appropriate
/// for `firing_capture: Off` where nothing accumulates anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FiringRetention {
    #[serde(default)]
    pub errors_with_snapshot: u32,
    #[serde(default)]
    pub successes_with_snapshot: u32,
    #[serde(default)]
    pub metadata_only: u32,
}

impl FiringRetention {
    /// Default retention caps applied by the schema_service client
    /// builder when the caller didn't specify (TH6a spec §6). Mirrored
    /// here so fold_db tests can construct the same defaults without
    /// pulling in the client crate.
    pub const fn client_defaults() -> Self {
        Self {
            errors_with_snapshot: 100,
            successes_with_snapshot: 10,
            metadata_only: 1000,
        }
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
    /// WASM transform (compiled bytes + system-wide `max_gas`). `None`
    /// means identity pass-through — the view materializes source
    /// fields directly without running any code, so no fuel ceiling is
    /// required. When `Some`, fuel metering is mandatory (MDT-E).
    #[serde(default)]
    pub wasm_transform: Option<WasmTransformSpec>,
    /// Typed output schema: field_name → type.
    pub output_fields: HashMap<String, FieldValueType>,
    /// Trigger configuration that controls when the view is fired.
    /// Empty / missing defaults to a single `OnWrite` trigger so views
    /// persisted before the trigger feature continue to invalidate on
    /// every source mutation.
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    /// Per-firing input-snapshot capture policy. TH6a spec §5–6.
    /// `#[serde(default)]` ensures existing serialized views (which
    /// don't carry this field) deserialize as `Off`, preserving
    /// pre-TH6a behavior.
    #[serde(default)]
    pub firing_capture: FiringCaptureMode,
    /// Per-view retention caps for `TriggerFiring` rows. TH6a spec §3.
    /// Zero caps (the deserialize default) effectively disable
    /// retention since nothing is captured under `firing_capture: Off`.
    #[serde(default)]
    pub firing_retention: FiringRetention,
}

impl TransformView {
    pub fn new(
        name: impl Into<String>,
        schema_type: SchemaType,
        key_config: Option<KeyConfig>,
        input_queries: Vec<Query>,
        wasm_transform: Option<WasmTransformSpec>,
        output_fields: HashMap<String, FieldValueType>,
    ) -> Self {
        Self {
            name: name.into(),
            schema_type,
            key_config,
            input_queries,
            wasm_transform,
            output_fields,
            triggers: Vec::new(),
            firing_capture: FiringCaptureMode::default(),
            firing_retention: FiringRetention::default(),
        }
    }

    /// Effective trigger list. Empty `triggers` defaults to
    /// `[Trigger::OnWrite { schemas: source_schemas }]` so every view fires
    /// on mutation unless the caller explicitly opted into a different
    /// trigger policy.
    pub fn effective_triggers(&self) -> Vec<Trigger> {
        if self.triggers.is_empty() {
            vec![Trigger::OnWrite {
                schemas: self.source_schemas(),
            }]
        } else {
            self.triggers.clone()
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

    /// For identity views, returns output_field → (source_schema, source_field).
    /// Returns None for WASM views (write-back requires inverse transform).
    pub fn source_field_map(&self) -> Option<HashMap<String, (String, String)>> {
        if !self.is_identity() {
            return None;
        }
        let mut map = HashMap::new();
        for query in &self.input_queries {
            for field_name in &query.fields {
                if self.output_fields.contains_key(field_name) {
                    map.insert(
                        field_name.clone(),
                        (query.schema_name.clone(), field_name.clone()),
                    );
                }
            }
        }
        Some(map)
    }

    /// Synthesize a [`crate::schema::types::Schema`] (AKA
    /// `DeclarativeSchemaDefinition`) that mirrors this view's declared
    /// output shape. The synthesized schema is registered alongside the
    /// view so derived mutations from the transform fire path — which
    /// target `schema_name = view.name` — land as normal atoms via
    /// `MutationManager::write_mutations_batch_async`.
    ///
    /// The synthesized schema has:
    ///
    /// - `name` / `schema_type` / `key` copied from the view.
    /// - `fields` listing every output field.
    /// - `field_types` mirroring the view's typed output fields.
    /// - `runtime_fields` populated (via [`crate::schema::types::Schema::populate_runtime_fields`])
    ///   so each field gets a `FieldVariant` matching the view's
    ///   schema_type. This is what `MutationManager::write_mutations_batch_async`
    ///   reaches for during Phase 2 (atom creation) — without it the
    ///   mutation pipeline errors with "Schema not found".
    /// - `source = SchemaSource::User` (views are treated as user-defined
    ///   schemas from the runtime's perspective; their actual origin is
    ///   recorded in the view registry).
    ///
    /// No data classifications are attached — the view's output inherits
    /// whatever classification schema_service pinned at registration, and
    /// the per-field classification check is enforced by
    /// `load_schema_from_json`, not by `load_schema_internal`. Derived
    /// views bypass the JSON path.
    pub fn to_synthesized_schema(
        &self,
    ) -> Result<crate::schema::types::Schema, crate::schema::SchemaError> {
        use crate::schema::types::Schema;

        let field_names: Vec<String> = {
            let mut v: Vec<String> = self.output_fields.keys().cloned().collect();
            v.sort();
            v
        };

        let mut schema = Schema::new(
            self.name.clone(),
            self.schema_type.clone(),
            self.key_config.clone(),
            Some(field_names),
            None,
            None,
        );
        schema.field_types = self.output_fields.clone();
        schema.populate_runtime_fields()?;
        Ok(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_transform_spec_gas_model_default_deserializes() {
        // `gas_model` is `#[serde(default)]` so legacy WasmTransformSpec
        // values persisted before Phase 2 deserialize cleanly with
        // `gas_model = None`. This pins the compatibility contract.
        let legacy_json = serde_json::json!({ "bytes": [1, 2, 3], "max_gas": 1_000_000 });
        let spec: WasmTransformSpec = serde_json::from_value(legacy_json).expect("deserialize");
        assert!(spec.gas_model.is_none());
        assert_eq!(spec.max_gas, 1_000_000);
    }

    #[test]
    fn test_gas_model_round_trip() {
        // The new GasModel / InputDimension / FieldId types must round-trip
        // through serde since they're embedded in WasmTransformSpec which
        // is persisted via TypedKvStore.
        let model = GasModel {
            base: 100,
            coefficients: vec![
                (
                    InputDimension::FieldBytes(FieldId {
                        schema: "BlogPost".to_string(),
                        field: "content".to_string(),
                    }),
                    2.5,
                ),
                (
                    InputDimension::FieldCount(FieldId {
                        schema: "Author".to_string(),
                        field: "name".to_string(),
                    }),
                    1.0,
                ),
            ],
            max_input_size: 65_536,
        };
        let bytes = serde_json::to_vec(&model).expect("serialize");
        let decoded: GasModel = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(decoded, model);
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
                Query::new(
                    "BlogPost".to_string(),
                    vec!["title".to_string(), "content".to_string()],
                ),
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
    fn firing_capture_default_is_off_for_existing_views() {
        // TH6a spec §6 — existing serialized views (which don't carry
        // `firing_capture`) must deserialize as `Off` so pre-TH6a
        // behavior (no snapshot capture) is preserved on upgrade.
        let legacy_json = serde_json::json!({
            "name": "Legacy",
            "schema_type": "Single",
            "input_queries": [],
            "output_fields": {}
        });
        let view: TransformView = serde_json::from_value(legacy_json).expect("deserialize");
        assert_eq!(view.firing_capture, FiringCaptureMode::Off);
        assert_eq!(view.firing_retention, FiringRetention::default());
        // All retention caps are zero on a legacy view — nothing to evict
        // since nothing's being captured.
        assert_eq!(view.firing_retention.errors_with_snapshot, 0);
        assert_eq!(view.firing_retention.successes_with_snapshot, 0);
        assert_eq!(view.firing_retention.metadata_only, 0);
    }

    #[test]
    fn firing_capture_round_trips_explicit_values() {
        // When a caller sets the new fields explicitly, they round-trip
        // through serde. Confirms the schema_service builder wire-shape
        // pass-through (TH6a spec §5).
        let view = TransformView {
            name: "V".to_string(),
            schema_type: SchemaType::Single,
            key_config: None,
            input_queries: vec![],
            wasm_transform: None,
            output_fields: HashMap::new(),
            triggers: vec![],
            firing_capture: FiringCaptureMode::ErrorsOnly,
            firing_retention: FiringRetention::client_defaults(),
        };
        let bytes = serde_json::to_vec(&view).expect("serialize");
        let decoded: TransformView = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(decoded.firing_capture, FiringCaptureMode::ErrorsOnly);
        assert_eq!(decoded.firing_retention.errors_with_snapshot, 100);
        assert_eq!(decoded.firing_retention.successes_with_snapshot, 10);
        assert_eq!(decoded.firing_retention.metadata_only, 1000);
    }

    #[test]
    fn firing_capture_serializes_snake_case() {
        // Wire format is stable snake_case so cross-language consumers
        // (TH6b CLI, schema_service) can match against fixed strings.
        assert_eq!(
            serde_json::to_string(&FiringCaptureMode::ErrorsOnly).unwrap(),
            "\"errors_only\""
        );
        assert_eq!(
            serde_json::to_string(&FiringCaptureMode::All).unwrap(),
            "\"all\""
        );
        assert_eq!(
            serde_json::to_string(&FiringCaptureMode::Off).unwrap(),
            "\"off\""
        );
    }

    #[test]
    fn test_is_identity() {
        let identity = TransformView::new(
            "id_view",
            SchemaType::Single,
            None,
            vec![],
            None,
            HashMap::new(),
        );
        assert!(identity.is_identity());

        let wasm_view = TransformView::new(
            "wasm_view",
            SchemaType::Single,
            None,
            vec![],
            Some(WasmTransformSpec {
                bytes: vec![0, 1, 2],
                max_gas: 1_000_000,
                gas_model: None,
            }),
            HashMap::new(),
        );
        assert!(!wasm_view.is_identity());
    }
}
