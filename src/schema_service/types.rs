use crate::schema::types::data_classification::DataClassification;
use crate::schema::types::field_value_type::FieldValueType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::schema::types::operations::Query;
use crate::schema::types::schema::DeclarativeSchemaType;
use crate::schema::types::Schema;

/// A canonical field entry in the global field registry.
/// Carries description (for semantic matching), type (for enforcement),
/// and optional data classification (for sensitivity labeling).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalField {
    pub description: String,
    pub field_type: FieldValueType,
    /// Data classification label for this field. `None` for legacy fields
    /// that were registered before classification was required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<DataClassification>,
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
    /// Optional WASM transform bytes (base64-encoded in JSON)
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
