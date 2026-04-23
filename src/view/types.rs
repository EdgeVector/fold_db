use crate::schema::types::field::FieldValue;
use crate::schema::types::field_value_type::FieldValueType;
use crate::schema::types::key_config::KeyConfig;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::operations::Query;
use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
use crate::triggers::types::Trigger;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reason a transform view attempted to compute but could not produce a value.
///
/// Carried by [`ViewCacheState::Unavailable`] so reads see an explicit failure
/// instead of an endlessly-retrying `Empty` or a lying stale `Cached`. The
/// state is sticky *per input* — a source mutation invalidates
/// `Unavailable` back to `Empty` so recompute on the new input can succeed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnavailableReason {
    /// Transform exceeded its fuel budget for the given input size.
    /// Implemented by MDT-E (system-wide `max_gas` enforcement); the variant
    /// is defined here so the state machine is complete ahead of that work.
    GasExceeded { input_size: u64 },
    /// WASM module failed to compile.
    CompileError { message: String },
    /// Transform bytes could not be fetched from the schema-service registry.
    TransformBytesUnavailable,
    /// WASM runtime error during execution (trap, alloc failure, output
    /// parse error, type-validation failure).
    ExecutionError { message: String },
    /// Measured input for this invocation exceeded the transform's
    /// calibrated [`GasModel::max_input_size`] envelope. Rejected BEFORE
    /// `execute_wasm_transform` is called — no fuel is burned — so the
    /// failure is cleanly distinguishable from `GasExceeded` (which is a
    /// runtime fuel trap during execution).
    ExceedsCalibratedEnvelope { measured: u64, limit: u64 },
}

impl std::fmt::Display for UnavailableReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GasExceeded { input_size } => {
                write!(f, "gas exceeded (input_size={input_size})")
            }
            Self::CompileError { message } => write!(f, "compile error: {message}"),
            Self::TransformBytesUnavailable => write!(f, "transform bytes unavailable"),
            Self::ExecutionError { message } => write!(f, "execution error: {message}"),
            Self::ExceedsCalibratedEnvelope { measured, limit } => {
                write!(
                    f,
                    "input exceeds calibrated envelope (measured={measured}, limit={limit})"
                )
            }
        }
    }
}

/// Cache state for an entire view's computed output.
/// Per-view (not per-field) since the WASM transform is holistic.
///
/// ```text
///   Empty ──(background task spawned)──▶ Computing
///     ▲  │                                   │
///     │  │                                   │ (task completes)
///     │  │                                   ▼
///     │  └─(compute fails)─▶ Unavailable  Cached
///     │                           │          │
///     └────(invalidate: source mutation)─────┘
/// ```
///
/// Views deeper than level 1 (i.e., depending on other views) transition
/// through `Computing` during background precomputation. Queries against
/// a `Computing` view return an error until precomputation finishes.
///
/// `Unavailable` is the terminal "compute attempted but failed" state. It
/// does NOT loop retrying; reads return the carried reason. A source
/// mutation invalidates it back to `Empty` (via [`ViewCacheState::invalidate`]),
/// so the transform retries once the input has changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewCacheState {
    /// Never computed or invalidated.
    Empty,
    /// Background precomputation in progress. Queries should wait or error.
    Computing,
    /// Computed output: field_name → (key → value).
    Cached {
        entries: HashMap<String, Vec<(KeyValue, FieldValue)>>,
    },
    /// Computation attempted but failed. Sticky per input — reads return
    /// the reason instead of retrying. A source mutation invalidates this
    /// back to `Empty` so recompute on the new input can succeed.
    Unavailable { reason: UnavailableReason },
}

impl ViewCacheState {
    /// Reset cache to `Empty`. Called by the view orchestrator on source
    /// mutation — this is the path that clears `Unavailable` so a retry can
    /// succeed with the new input.
    pub fn invalidate(&mut self) {
        *self = ViewCacheState::Empty;
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, ViewCacheState::Empty)
    }

    pub fn is_computing(&self) -> bool {
        matches!(self, ViewCacheState::Computing)
    }

    pub fn is_unavailable(&self) -> bool {
        matches!(self, ViewCacheState::Unavailable { .. })
    }

    /// Returns the failure reason when the state is `Unavailable`, else `None`.
    pub fn unavailable_reason(&self) -> Option<&UnavailableReason> {
        match self {
            ViewCacheState::Unavailable { reason } => Some(reason),
            _ => None,
        }
    }
}

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
///    reject with [`UnavailableReason::ExceedsCalibratedEnvelope`] if the
///    total exceeds `max_input_size`. No fuel is burned on rejection.
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
/// `(transform, input)` pair must either succeed on every device or fail
/// with [`UnavailableReason::GasExceeded`] on every device. Allowing a
/// missing budget would let one device silently skip fuel metering and
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
    fn test_unavailable_invalidates_to_empty() {
        // Source mutation → invalidate clears Unavailable back to Empty so
        // the transform can retry on new input.
        let mut unavail = ViewCacheState::Unavailable {
            reason: UnavailableReason::GasExceeded { input_size: 42 },
        };
        assert!(unavail.is_unavailable());
        assert!(!unavail.is_empty());
        unavail.invalidate();
        assert!(unavail.is_empty());
        assert!(!unavail.is_unavailable());
    }

    #[test]
    fn test_unavailable_reason_accessor() {
        let state = ViewCacheState::Unavailable {
            reason: UnavailableReason::CompileError {
                message: "bad wasm".to_string(),
            },
        };
        assert_eq!(
            state.unavailable_reason(),
            Some(&UnavailableReason::CompileError {
                message: "bad wasm".to_string()
            })
        );

        let empty = ViewCacheState::Empty;
        assert_eq!(empty.unavailable_reason(), None);
    }

    #[test]
    fn test_unavailable_round_trip() {
        // Every variant must round-trip through serde_json (the backend
        // TypedKvStore uses for persistence) so Unavailable survives restart.
        let reasons = vec![
            UnavailableReason::GasExceeded { input_size: 12345 },
            UnavailableReason::CompileError {
                message: "compile failed at offset 0xDEADBEEF".to_string(),
            },
            UnavailableReason::TransformBytesUnavailable,
            UnavailableReason::ExecutionError {
                message: "trap: unreachable".to_string(),
            },
            UnavailableReason::ExceedsCalibratedEnvelope {
                measured: 98_765,
                limit: 50_000,
            },
        ];
        for reason in reasons {
            let state = ViewCacheState::Unavailable {
                reason: reason.clone(),
            };
            let bytes = serde_json::to_vec(&state).expect("serialize");
            let decoded: ViewCacheState = serde_json::from_slice(&bytes).expect("deserialize");
            assert_eq!(decoded.unavailable_reason(), Some(&reason));
        }
    }

    #[test]
    fn test_unavailable_reason_display() {
        assert_eq!(
            UnavailableReason::GasExceeded { input_size: 100 }.to_string(),
            "gas exceeded (input_size=100)"
        );
        assert_eq!(
            UnavailableReason::TransformBytesUnavailable.to_string(),
            "transform bytes unavailable"
        );
        assert_eq!(
            UnavailableReason::ExceedsCalibratedEnvelope {
                measured: 2048,
                limit: 1024
            }
            .to_string(),
            "input exceeds calibrated envelope (measured=2048, limit=1024)"
        );
    }

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
