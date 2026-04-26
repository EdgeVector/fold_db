use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Represents the schema-level type information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(
        export,
        export_to = "bindings/src/fold_node/static-react/src/types/generated.ts"
    )
)]
pub enum DeclarativeSchemaType {
    /// Single schema without range semantics
    Single,
    /// Schema keyed by a single hash key (unordered collection)
    Hash,
    /// Schema that stores data in a key range
    Range,
    /// Schema that uses hashed and ranged keys for partitioning
    HashRange,
}

// Schema is now DeclarativeSchemaDefinition - the unified declarative schema type
pub use crate::schema::types::declarative_schemas::DeclarativeSchemaDefinition as Schema;
