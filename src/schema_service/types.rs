use crate::schema::types::data_classification::DataClassification;
use crate::schema::types::field_value_type::FieldValueType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::schema::types::operations::Query;
use crate::schema::types::schema::DeclarativeSchemaType;
use crate::schema::types::Schema;

/// A canonical field entry in the global field registry.
/// Carries description (for semantic matching), type (for enforcement),
/// optional data classification (for sensitivity labeling), and optional
/// interest category (for discovery/social features).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalField {
    pub description: String,
    pub field_type: FieldValueType,
    /// Data classification label for this field. `None` for legacy fields
    /// that were registered before classification was required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<DataClassification>,
    /// Interest category for discovery (e.g. "Photography", "Cooking", "Running").
    /// Assigned by LLM at field registration time. `None` for fields that don't
    /// map to a user interest (e.g. content_hash, source, id fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interest_category: Option<String>,
}

/// Response containing a list of available schema names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemasListResponse {
    pub schemas: Vec<String>,
}

/// Response containing all available schemas with their definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableSchemasResponse {
    pub schemas: Vec<Schema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaAddOutcome {
    Added(Schema, HashMap<String, String>), // Schema and mutation_mappers
    AlreadyExists(Schema, HashMap<String, String>), // Exact same identity hash + mappers from canonicalization
    /// Existing schema was expanded with new fields (old schema name, expanded schema, mappers)
    Expanded(String, Schema, HashMap<String, String>),
}

/// Error response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Request structure for adding a schema with mutation mappers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddSchemaRequest {
    pub schema: Schema,
    pub mutation_mappers: HashMap<String, String>,
}

/// Response structure for adding a schema with mutation mappers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddSchemaResponse {
    pub schema: Schema,
    pub mutation_mappers: HashMap<String, String>,
    /// When a schema expansion occurred, this contains the old schema name
    /// that was replaced. The node should remove the old schema and load the new one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced_schema: Option<String>,
}

/// Reload response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadResponse {
    pub success: bool,
    pub schemas_loaded: usize,
}

/// Health check response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

/// A schema entry with its similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarSchemaEntry {
    pub schema: Schema,
    pub similarity: f64,
}

/// Response for the find-similar-schemas endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarSchemasResponse {
    pub query_schema: String,
    pub threshold: f64,
    pub similar_schemas: Vec<SimilarSchemaEntry>,
}

/// Request for resetting the schema service database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetRequest {
    pub confirm: bool,
}

/// Response for resetting the schema service database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetResponse {
    pub success: bool,
    pub message: String,
}

/// A single schema lookup entry in a batch reuse request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaLookupEntry {
    pub descriptive_name: String,
    pub fields: Vec<String>,
}

/// Batch request: multiple schema names to check at once
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSchemaReuseRequest {
    pub schemas: Vec<SchemaLookupEntry>,
}

/// Result for a single matched schema in the batch reuse check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaReuseMatch {
    pub schema: Schema,
    pub matched_descriptive_name: String,
    pub is_exact_match: bool,
    pub field_rename_map: HashMap<String, String>,
    pub is_superset: bool,
    pub unmapped_fields: Vec<String>,
}

/// Batch response: input descriptive_name -> match result.
/// Only names with matches are included; missing keys = no match found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSchemaReuseResponse {
    pub matches: HashMap<String, SchemaReuseMatch>,
}

// ============== View Types ==============

/// A stored view definition in the global registry.
/// Views are computed lenses: input queries + optional WASM transform → output schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredView {
    /// View name (human-readable)
    pub name: String,
    /// Queries that feed data into this view
    pub input_queries: Vec<Query>,
    /// sha256 hash referencing Global Transform Registry — fetched on demand
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform_hash: Option<String>,
    /// Fallback: inline WASM bytes (for local/dev use only, not registered)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wasm_bytes: Option<Vec<u8>>,
    /// Identity hash of the output schema (registered via add_schema)
    pub output_schema_name: String,
    /// Schema type for the view output
    pub schema_type: DeclarativeSchemaType,
}

/// Request to register a new view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddViewRequest {
    /// Human-readable view name
    pub name: String,
    /// Descriptive name for the output schema (used in similarity matching)
    pub descriptive_name: String,
    /// Queries that feed data into this view
    pub input_queries: Vec<Query>,
    /// Output field names
    pub output_fields: Vec<String>,
    /// Descriptions for each output field (required for semantic matching)
    pub field_descriptions: HashMap<String, String>,
    /// Classifications for each output field
    #[serde(default)]
    pub field_classifications: HashMap<String, Vec<String>>,
    /// Data classifications for each output field (sensitivity + domain)
    #[serde(default)]
    pub field_data_classifications: HashMap<String, DataClassification>,
    /// Optional WASM transform bytes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wasm_bytes: Option<Vec<u8>>,
    /// Optional reference to a pre-registered transform in the Global Transform
    /// Registry. When set without `wasm_bytes`, the bytes are fetched from the
    /// registry. When set with `wasm_bytes`, the hash must match
    /// `sha256(wasm_bytes)` or the request is rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform_hash: Option<String>,
    /// Schema type for the view output
    #[serde(default = "default_schema_type")]
    pub schema_type: DeclarativeSchemaType,
}

fn default_schema_type() -> DeclarativeSchemaType {
    DeclarativeSchemaType::Single
}

/// Outcome of registering a view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewAddOutcome {
    /// View registered, output schema was newly added
    Added(StoredView, Schema),
    /// View registered, output schema already existed
    AddedWithExistingSchema(StoredView, Schema),
    /// View registered, output schema was expanded from an existing one
    Expanded(StoredView, Schema, String), // view, schema, old_schema_name
}

/// Response for adding a view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddViewResponse {
    pub view: StoredView,
    pub output_schema: Schema,
    /// If the output schema expanded an existing one, the old schema name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced_schema: Option<String>,
}

/// Response containing a list of view names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewsListResponse {
    pub views: Vec<String>,
}

/// Response containing all views with their definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableViewsResponse {
    pub views: Vec<StoredView>,
}

// ============== Transform Types ==============

/// A registered transform in the Global Transform Registry.
/// Metadata record — does NOT include wasm_bytes (stored separately).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformRecord {
    /// sha256(wasm_bytes) — the canonical identity
    pub hash: String,
    /// Human-readable name (e.g. "downgrade_medical_to_summary")
    pub name: String,
    /// Semver version string
    pub version: String,
    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The input queries the transform was registered against. Views linked
    /// to this transform must query the same (schema_name, field) pairs so
    /// the Phase 1/2 classification stays coherent with what the transform
    /// actually sees at runtime.
    #[serde(default)]
    pub input_queries: Vec<Query>,
    /// Input field types expected by the transform (resolved from input_queries)
    pub input_schema: HashMap<String, FieldValueType>,
    /// Output field types produced by the transform
    pub output_schema: HashMap<String, FieldValueType>,
    /// URL to source code (GitHub, etc.) — for verifiability
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// When registered (Unix timestamp)
    pub registered_at: u64,
    /// Phase 1: max of all input field classifications
    pub input_ceiling: DataClassification,
    /// Phase 2: NMI-derived output classification (or ceiling if inconclusive)
    pub output_classification: DataClassification,
    /// Input field → output field → NMI score
    #[serde(default)]
    pub nmi_matrix: HashMap<String, HashMap<String, f32>>,
    /// true if Phase 2 ran with sufficient samples
    pub classification_verified: bool,
    /// How many synthetic samples Phase 2 used (0 if Phase 2 skipped)
    pub sample_count: u32,
    /// Enforced classification: max(ceiling, output)
    pub assigned_classification: DataClassification,
}

/// Request to register a new transform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterTransformRequest {
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Queries defining what this transform reads — field classifications resolved from schema service
    pub input_queries: Vec<Query>,
    /// Output field types produced by the transform
    pub output_fields: HashMap<String, FieldValueType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// The compiled WASM bytes (base64-encoded in JSON)
    pub wasm_bytes: Vec<u8>,
}

/// Response for registering a transform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterTransformResponse {
    /// Computed sha256 hash
    pub hash: String,
    /// Full transform record (without wasm_bytes)
    pub record: TransformRecord,
    /// Whether the transform was newly added or already existed
    pub outcome: TransformAddOutcome,
}

/// Outcome of registering a transform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformAddOutcome {
    /// Transform was newly registered
    Added,
    /// Transform already exists (same hash) — idempotent
    AlreadyExists,
}

/// Response containing a list of transform hashes + names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformsListResponse {
    pub transforms: Vec<TransformListEntry>,
}

/// A single entry in the transforms list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformListEntry {
    pub hash: String,
    pub name: String,
    pub version: String,
}

/// Response containing all transforms with full metadata (no wasm_bytes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableTransformsResponse {
    pub transforms: Vec<TransformRecord>,
}

/// Request to verify a WASM blob matches a hash
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyTransformRequest {
    pub hash: String,
    pub wasm_bytes: Vec<u8>,
}

/// Response for verify endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyTransformResponse {
    pub hash: String,
    pub matches: bool,
    pub computed_hash: String,
}

/// A transform entry with its similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarTransformEntry {
    pub record: TransformRecord,
    pub similarity: f64,
}

/// Response for the find-similar-transforms endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarTransformsResponse {
    pub query_name: String,
    pub threshold: f64,
    pub similar_transforms: Vec<SimilarTransformEntry>,
}
