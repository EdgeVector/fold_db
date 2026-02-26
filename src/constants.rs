/// Common constants used across the FoldDB project.
///
/// These defaults are used for command line arguments and
/// configuration when explicit values are not provided.
pub const DEFAULT_HTTP_PORT: u16 = 9001;
pub const DEFAULT_SCHEMA_SERVICE_PORT: u16 = 9002;

/// The sole ID for the single public key used for signing validation.
pub const SINGLE_PUBLIC_KEY_ID: &str = "SYSTEM_WIDE_PUBLIC_KEY";

/// Default schema service URL (dev).
pub const DEFAULT_SCHEMA_SERVICE_URL: &str =
    "https://y0q3m6vk75.execute-api.us-west-2.amazonaws.com";
