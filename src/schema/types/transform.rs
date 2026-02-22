use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Represents a transformation that can be applied to field values.
///
/// Transforms define how data from source fields is processed to produce
/// a derived value. They are expressed in a domain-specific language (DSL)
/// that supports basic arithmetic, comparisons, conditionals, and a small
/// set of built-in functions.
///
/// # Features
///
/// * Declarative syntax for expressing transformations
/// * Support for basic arithmetic, comparisons, and conditionals
/// * Optional signature for verification and auditability
/// * Payment requirements for accessing transformed data
/// * Automatic input dependency tracking
///
/// Parameters for registering a transform
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformRegistration {
    /// The ID of the transform
    pub transform_id: String,
    /// The transform itself
    pub transform: Transform,
    /// Fields that trigger the transform
    pub trigger_fields: Vec<String>,
}

/// Transform stores only a schema_name reference to avoid duplication.
/// The full schema is stored in schemas_tree and looked up when needed.
/// This saves ~50% storage for transform schemas (previously stored in both trees).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(
        export,
        export_to = "bindings/src/fold_node/static-react/src/types/generated.ts"
    )
)]
pub struct Transform {
    /// The name of the schema (stored in schemas_tree)
    pub schema_name: String,
}

impl Transform {
    /// Creates a new Transform from a schema name.
    #[must_use]
    pub fn from_schema_name(schema_name: String) -> Self {
        Self { schema_name }
    }

    /// Gets the schema name.
    pub fn get_schema_name(&self) -> &str {
        &self.schema_name
    }
}
