//! External persistence trait for the schema service.
//!
//! This trait lets deployments plug in their own storage backend without
//! the `fold_db` library having to know about any particular cloud service.
//! The built-in local backend is Sled (see `SchemaStorage::Sled`). Remote
//! deployments (e.g. the schema-infra Lambda) implement this trait and
//! construct the schema service via `SchemaServiceState::new_with_external`.
//!
//! Design notes:
//! - All methods are async so implementations can talk to network services
//!   (S3, DynamoDB, etc.) without blocking the tokio runtime.
//! - The trait is intentionally low-level: it owns persistence only. The
//!   schema service in `fold_db` keeps all business logic (canonicalization,
//!   similarity, expansion, classification) — the implementation only has
//!   to answer "given this key, save/load these bytes."
//! - `save_*` methods take already-serialized domain objects. Implementations
//!   serialize to whatever on-disk format they like (JSON blob, DynamoDB
//!   attribute map, etc.).
//! - `load_all_*` methods return the full domain set because the schema
//!   service caches everything in memory at startup and serves reads from
//!   the cache.

use std::collections::HashMap;

use async_trait::async_trait;

use super::types::{CanonicalField, StoredView, TransformRecord};
use crate::error::FoldDbResult;
use crate::schema::types::Schema;

/// Persistence backend for the schema service.
///
/// Implementations live outside `fold_db` (for example, in the
/// schema-infra Lambda). Each method is a single storage primitive —
/// no business logic — so backends stay easy to build and test.
#[async_trait]
pub trait ExternalSchemaPersistence: Send + Sync {
    // ============== Schemas ==============

    /// Persist a single schema. Schemas are keyed by `schema.name`
    /// (which is the content-hash identity).
    ///
    /// Must be idempotent: a second call with the same schema must
    /// succeed as a no-op.
    async fn save_schema(&self, schema: &Schema) -> FoldDbResult<()>;

    /// Load every schema from storage.
    ///
    /// Called once during schema service startup to populate the
    /// in-memory cache. Returns a map from `schema.name` → Schema.
    async fn load_all_schemas(&self) -> FoldDbResult<HashMap<String, Schema>>;

    // ============== Canonical fields ==============

    /// Persist a single canonical field entry keyed by `name`.
    ///
    /// Must be idempotent.
    async fn save_canonical_field(&self, name: &str, field: &CanonicalField) -> FoldDbResult<()>;

    /// Load every canonical field from storage.
    async fn load_all_canonical_fields(&self) -> FoldDbResult<HashMap<String, CanonicalField>>;

    // ============== Views ==============

    /// Persist a single view keyed by `view.name`.
    async fn save_view(&self, view: &StoredView) -> FoldDbResult<()>;

    /// Load every view from storage.
    async fn load_all_views(&self) -> FoldDbResult<HashMap<String, StoredView>>;

    // ============== Transforms ==============

    /// Persist a transform record (metadata only — not the WASM bytes).
    /// Keyed by `record.hash`.
    async fn save_transform_metadata(&self, record: &TransformRecord) -> FoldDbResult<()>;

    /// Persist the raw WASM bytes for a transform. Stored separately
    /// from metadata because bytes can be large; backends may choose
    /// a different storage medium (e.g. S3 alongside DynamoDB metadata).
    async fn save_transform_wasm(&self, hash: &str, wasm_bytes: &[u8]) -> FoldDbResult<()>;

    /// Load every transform record (metadata only — not the WASM bytes).
    async fn load_all_transforms(&self) -> FoldDbResult<HashMap<String, TransformRecord>>;

    /// Fetch WASM bytes for a single transform by hash. Returns `None`
    /// if the hash is unknown.
    async fn load_transform_wasm(&self, hash: &str) -> FoldDbResult<Option<Vec<u8>>>;

    /// Persist the Rust source text for a transform, keyed by the WASM
    /// hash. Called only when the transform was submitted as source
    /// (not pre-compiled bytes).
    ///
    /// Default implementation is a no-op so existing backends stay
    /// compatible; source will simply not be retrievable from those
    /// backends. Override to enable server-side source storage.
    async fn save_transform_source(&self, _hash: &str, _source: &str) -> FoldDbResult<()> {
        Ok(())
    }

    /// Fetch the Rust source text for a transform by hash. Returns `None`
    /// if the transform has no stored source (pre-compiled upload) or the
    /// hash is unknown.
    ///
    /// Default implementation returns `None`.
    async fn load_transform_source(&self, _hash: &str) -> FoldDbResult<Option<String>> {
        Ok(None)
    }
}
