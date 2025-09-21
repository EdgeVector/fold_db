//! Constants for schema definitions and transform operations

/// HashRange schema key configuration field names
pub const HASH_FIELD_NAME: &str = "hash_field";
pub const RANGE_FIELD_NAME: &str = "range_field";

/// HashRange schema key configuration JSON field names
pub const KEY_CONFIG_HASH_FIELD: &str = "hash_field";
pub const KEY_CONFIG_RANGE_FIELD: &str = "range_field";

/// Special field names used in transforms
pub const ATOM_UUID_FIELD: &str = "$atom_uuid";
pub const KEY_FIELD_NAME: &str = "key";

/// Transform configuration constants
pub const DEFAULT_TRANSFORM_ID_SUFFIX: &str = "declarative";
pub const DEFAULT_OUTPUT_FIELD_NAME: &str = "output";
pub const DEFAULT_VALIDATION_MAX_LOGIC_LENGTH: usize = 10000;

/// System identifiers for mutations and operations
pub const TRANSFORM_SYSTEM_ID: &str = "transform_system";
pub const DATA_STORAGE_SYSTEM_ID: &str = "data_storage_system";
