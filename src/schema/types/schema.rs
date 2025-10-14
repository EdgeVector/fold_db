use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

/// Represents the schema-level type information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS, ToSchema)]
#[ts(export, export_to = "bindings/src/datafold_node/static-react/src/types/generated.ts")]
pub enum DeclarativeSchemaType {
    /// Single schema without range semantics
    Single,
    /// Schema that stores data in a key range
    Range,
    /// Schema that uses hashed and ranged keys for partitioning
    HashRange,
}


pub fn default_schema_type() -> DeclarativeSchemaType {
    DeclarativeSchemaType::Single
}

// Schema is now DeclarativeSchemaDefinition - the unified declarative schema type
pub use crate::schema::types::declarative_schemas::DeclarativeSchemaDefinition as Schema;