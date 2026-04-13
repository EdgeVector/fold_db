// Core database operations
pub mod core;
// Domain stores (private namespace fields, public operation methods)
pub mod atom_store;
pub mod metadata_store;
pub mod permissions_store;
pub mod schema_store;
pub mod view_store;
// Thin delegator impl blocks on DbOperations (backward compat)
pub mod atom_operations;
mod conflict_operations;
mod metadata_operations;
pub mod native_index;
pub mod org_operations;
pub mod public_key_operations;
mod schema_operations;
mod trust_operations;
mod view_operations;
// Re-exports
pub use atom_store::{AtomStore, MoleculeData};
pub use core::DbOperations;
pub use metadata_store::MetadataStore;
pub use native_index::{IndexResult, NativeIndexManager};
pub use permissions_store::PermissionsStore;
pub use schema_store::SchemaStore;
pub use view_store::ViewStore;
