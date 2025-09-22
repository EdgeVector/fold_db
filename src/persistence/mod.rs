//! Persistence utilities for working with native (non-JSON) data paths.
//!
//! The persistence module centralizes storage logic for native
//! `FieldValue` structures so the rest of the application can avoid
//! touching raw `serde_json::Value` instances. The initial
//! implementation introduces [`NativePersistence`], a helper that
//! validates records against typed schema metadata before converting
//! them into the database representation.

pub mod native_persistence;

pub use native_persistence::{
    KeyConfig, NativePersistence, NativeRecordKey, NativeSchemaProvider, PersistenceError,
    SchemaDescription,
};
