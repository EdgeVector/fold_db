use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Transform stores a schema_name reference.
/// The full schema is stored in schemas_tree and looked up when needed.
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
