//! Canonical attribute keys + redaction macros.
//!
//! All `tracing` event/span fields across the fold_db ecosystem should use
//! the constants in this module rather than ad-hoc strings — that way the
//! format-time deny-list (Phase 1 / T3) and the CI lint (Phase 5) have a
//! single source of truth to enforce.
//!
//! ## Naming conventions
//!
//! - HTTP, DB, and service-level keys follow OpenTelemetry semantic
//!   conventions (`http.method`, `db.system`, `service.name`).
//! - fold_db-specific keys use a `fold.*` prefix.
//! - User-derived identifiers that need correlatability but not legibility
//!   use the `*.hash` suffix and **must** be wrapped in [`redact_id!`] at
//!   the call site.
//!
//! ## Redaction
//!
//! Two macros, two purposes:
//!
//! - [`redact!`] — **opaque**. Replaces the value with the literal
//!   `<redacted>`. Use for things you never want to see again: passwords,
//!   raw API keys, encrypted blob contents.
//! - [`redact_id!`] — **correlatable**. Hashes the value with xxhash64 and
//!   formats it as `<id:abcd1234>` (8 hex chars, low 32 bits). Use for
//!   identifiers you need to follow across log lines without exposing the
//!   underlying string (user hashes, document IDs, schema names in
//!   sensitive contexts).
//!
//! Both return a value, so they slot directly into `tracing` field
//! positions:
//!
//! ```ignore
//! tracing::info!(
//!     user.hash = %observability::redact_id!(&user_hash),
//!     api.key = %observability::redact!(&api_key),
//!     "request received",
//! );
//! ```

// ---------------------------------------------------------------------------
// HTTP — OpenTelemetry semantic conventions
// ---------------------------------------------------------------------------

pub const HTTP_METHOD: &str = "http.method";
pub const HTTP_ROUTE: &str = "http.route";
pub const HTTP_URL: &str = "http.url";
pub const HTTP_STATUS_CODE: &str = "http.status_code";
pub const HTTP_TARGET: &str = "http.target";
pub const HTTP_USER_AGENT: &str = "http.user_agent";

// ---------------------------------------------------------------------------
// DB / storage — OpenTelemetry semantic conventions where applicable
// ---------------------------------------------------------------------------

pub const DB_SYSTEM: &str = "db.system";
pub const DB_OPERATION: &str = "db.operation";
pub const DB_STATEMENT: &str = "db.statement";

// ---------------------------------------------------------------------------
// Service identity
// ---------------------------------------------------------------------------

pub const SERVICE_NAME: &str = "service.name";
pub const SERVICE_VERSION: &str = "service.version";

// ---------------------------------------------------------------------------
// fold_db-specific
// ---------------------------------------------------------------------------

/// Hashed user identifier. Always wrap the value in [`redact_id!`].
pub const USER_HASH: &str = "user.hash";

/// Logical schema name (e.g. `Notes`, `Conversation`).
pub const SCHEMA_NAME: &str = "schema.name";

/// Field within a schema.
pub const SCHEMA_FIELD: &str = "schema.field";

/// Operation against a schema (e.g. `query`, `mutation`, `transform`).
pub const SCHEMA_OPERATION: &str = "schema.operation";

/// fold_db node identifier (the persistent Ed25519-derived ID).
pub const FOLD_NODE_ID: &str = "fold.node_id";

/// Logical "feature" tag from the legacy `LogFeature` enum, surfaced on
/// every event for filter parity during the Phase 3 migration.
pub const FOLD_FEATURE: &str = "fold.feature";

/// Range key (or sort-order key) used in a query.
pub const FOLD_QUERY_RANGE: &str = "fold.query.range";

/// Number of rows / records affected by an operation.
pub const FOLD_RESULT_COUNT: &str = "fold.result.count";

// ---------------------------------------------------------------------------
// Redaction macros
// ---------------------------------------------------------------------------

/// Replace a sensitive value with the literal `<redacted>`.
///
/// Returns `&'static str` so it composes with `tracing`'s field syntax:
/// `tracing::info!(api.key = %observability::redact!(value), "...")`.
///
/// The `$value` expression is intentionally **not** evaluated — there is no
/// reason to compute a string we will never look at, and we want to avoid
/// accidentally materializing a secret on the stack.
#[macro_export]
macro_rules! redact {
    ($value:expr) => {{
        // Suppress unused-variable lints if the caller passes a let-binding.
        let _ = &$value;
        "<redacted>"
    }};
}

/// Replace a sensitive identifier with `<id:HHHHHHHH>` where `HHHHHHHH` is
/// the lowercase 8-hex-character (low 32-bit) xxhash64 of the input.
///
/// Returns `String`. Accepts anything that implements `AsRef<[u8]>` (so
/// `&str`, `String`, `&[u8]`, `Vec<u8>` all work).
#[macro_export]
macro_rules! redact_id {
    ($value:expr) => {{
        let __obs_redact_id_value = &$value;
        let __obs_redact_id_bytes: &[u8] =
            ::std::convert::AsRef::<[u8]>::as_ref(__obs_redact_id_value);
        let __obs_redact_id_hash = ::xxhash_rust::xxh64::xxh64(__obs_redact_id_bytes, 0) as u32;
        format!("<id:{:08x}>", __obs_redact_id_hash)
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn redact_returns_static_marker() {
        let secret = String::from("hunter2");
        let out = crate::redact!(secret);
        assert_eq!(out, "<redacted>");
    }

    #[test]
    fn redact_does_not_consume_value() {
        let secret = String::from("hunter2");
        let _out = crate::redact!(secret);
        // `secret` should still be usable — the macro must not move it.
        assert_eq!(secret.len(), 7);
    }

    #[test]
    fn redact_id_is_deterministic_and_format_correct() {
        let id = "user-1234";
        let a = crate::redact_id!(id);
        let b = crate::redact_id!(id);
        assert_eq!(a, b, "redact_id! must be deterministic for the same input");
        assert!(a.starts_with("<id:"), "format prefix");
        assert!(a.ends_with('>'), "format suffix");

        // `<id:` (4) + 8 hex + `>` (1) = 13.
        assert_eq!(a.len(), 13, "expected `<id:HHHHHHHH>` shape, got {a}");

        let hex = &a[4..12];
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "expected lowercase hex digits, got {hex}"
        );
    }

    #[test]
    fn redact_id_differs_for_different_inputs() {
        let a = crate::redact_id!("alice");
        let b = crate::redact_id!("bob");
        assert_ne!(a, b);
    }

    #[test]
    fn redact_id_accepts_str_string_and_bytes() {
        let s: &str = "abc";
        let owned: String = String::from("abc");
        let bytes: &[u8] = b"abc";

        let from_str = crate::redact_id!(s);
        let from_string = crate::redact_id!(owned);
        let from_bytes = crate::redact_id!(bytes);

        assert_eq!(from_str, from_string);
        assert_eq!(from_str, from_bytes);
    }
}
