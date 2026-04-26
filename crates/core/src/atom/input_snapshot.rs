//! Canonical hasher for transform input snapshots.
//!
//! Pure helper. Produces a deterministic SHA-256 hex string from the exact
//! `HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>>` shape
//! assembled at `view/resolver.rs::execute_wasm_transform` before the WASM
//! call. The result is the value that will populate
//! `Provenance::Derived::input_snapshot_hash` once transform output is routed
//! through `MutationManager` (project `view-compute-as-mutations`, not this
//! PR).
//!
//! Nothing in production calls this yet — see
//! `gbrain get projects/molecule-provenance-dag` for the 6-PR arc. This is
//! PR 3; the only callers are unit tests in this module.
//!
//! # Canonical encoding — locked forever
//!
//! Governed by `Provenance::Derived::encoding_version`. Bumping is a content-
//! address-breaking change for every derived molecule; do not change the
//! layout without a new `encoding_version`.
//!
//! For every `(schema, field, key, value)` entry, sorted ascending by
//! `schema`, then by `field`, then by the canonical string form of `key`,
//! append:
//!
//! ```text
//! schema_bytes | 0x00 | field_bytes | 0x00 | key_canonical_bytes | 0x00 | value_canonical_bytes | 0x00
//! ```
//!
//! - `schema_bytes` / `field_bytes` — UTF-8 of the map keys.
//! - `key_canonical_bytes` — UTF-8 of `KeyValue::to_string()` (see
//!   `schema::types::key_value::KeyValue`'s `Display` impl).
//! - `value_canonical_bytes` — UTF-8 of canonical JSON of `FieldValue::value`
//!   (see [`canonical_json_bytes`]). Only the `value` field contributes;
//!   `atom_uuid`, `molecule_uuid`, `writer_pubkey` and other metadata do not.
//!
//! The SHA-256 of the concatenated stream (hex-encoded, 64 chars) is the
//! return value. An empty input hashes to the SHA-256 of the empty byte
//! string.

use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Deterministically hash a transform input snapshot.
///
/// See module docs for the canonical encoding. Returns a 64-char lowercase
/// SHA-256 hex string suitable for `Provenance::Derived::input_snapshot_hash`.
#[must_use]
pub fn hash_input_snapshot(
    inputs: &HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>>,
) -> String {
    let mut hasher = Sha256::new();

    let mut schemas: Vec<&String> = inputs.keys().collect();
    schemas.sort();

    for schema in schemas {
        let fields_map = &inputs[schema];
        let mut fields: Vec<&String> = fields_map.keys().collect();
        fields.sort();

        for field in fields {
            let entries = &fields_map[field];
            let mut entries_sorted: Vec<(String, &KeyValue, &FieldValue)> =
                entries.iter().map(|(k, v)| (k.to_string(), k, v)).collect();
            entries_sorted.sort_by(|a, b| a.0.cmp(&b.0));

            for (key_canonical, _, fv) in &entries_sorted {
                hasher.update(schema.as_bytes());
                hasher.update([0x00]);
                hasher.update(field.as_bytes());
                hasher.update([0x00]);
                hasher.update(key_canonical.as_bytes());
                hasher.update([0x00]);
                let value_bytes = canonical_json_bytes(&fv.value);
                hasher.update(&value_bytes);
                hasher.update([0x00]);
            }
        }
    }

    format!("{:x}", hasher.finalize())
}

/// Canonical JSON encoding of a `serde_json::Value`.
///
/// Stable across insertion order: object keys are emitted in ascending
/// lexicographic order. Recursive — nested objects are canonicalized too.
/// Primitive encodings match `serde_json`'s default output so that a
/// round-trip through `serde_json` preserves bytes for scalar / array cases.
///
/// This is a hand-rolled canonicalizer (no new crate dep) — the sort-keys
/// behavior is pinned by [`canonicalizes_nested_object_keys`] and by
/// [`hash_equal_across_nested_json_key_order`].
fn canonical_json_bytes(value: &serde_json::Value) -> Vec<u8> {
    let mut buf = Vec::new();
    write_canonical_json(value, &mut buf);
    buf
}

fn write_canonical_json(value: &serde_json::Value, buf: &mut Vec<u8>) {
    use serde_json::Value;
    match value {
        Value::Null => buf.extend_from_slice(b"null"),
        Value::Bool(true) => buf.extend_from_slice(b"true"),
        Value::Bool(false) => buf.extend_from_slice(b"false"),
        Value::Number(n) => buf.extend_from_slice(n.to_string().as_bytes()),
        Value::String(s) => {
            // serde_json's string serialization handles escaping; String
            // serialization is infallible.
            let encoded = serde_json::to_string(s)
                .expect("serde_json string serialization is infallible for &String");
            buf.extend_from_slice(encoded.as_bytes());
        }
        Value::Array(arr) => {
            buf.push(b'[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_canonical_json(v, buf);
            }
            buf.push(b']');
        }
        Value::Object(obj) => {
            let mut entries: Vec<(&String, &Value)> = obj.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            buf.push(b'{');
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                let encoded_key = serde_json::to_string(*k)
                    .expect("serde_json string serialization is infallible for &String");
                buf.extend_from_slice(encoded_key.as_bytes());
                buf.push(b':');
                write_canonical_json(v, buf);
            }
            buf.push(b'}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fv(value: serde_json::Value) -> FieldValue {
        FieldValue {
            value,
            atom_uuid: String::new(),
            source_file_name: None,
            metadata: None,
            molecule_uuid: None,
            molecule_version: None,
            writer_pubkey: None,
            written_at: None,
        }
    }

    fn kv(hash: Option<&str>, range: Option<&str>) -> KeyValue {
        KeyValue::new(hash.map(String::from), range.map(String::from))
    }

    /// Build the 2×2×2 fixture used by the pinned-vector test and by
    /// insertion-order tests.
    fn fixture_2x2x2() -> HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>> {
        let mut inputs = HashMap::new();

        let mut schema_a = HashMap::new();

        let mut field_a1 = HashMap::new();
        field_a1.insert(kv(Some("h1"), None), fv(json!("a1-h1")));
        field_a1.insert(kv(Some("h2"), None), fv(json!("a1-h2")));

        let mut field_a2 = HashMap::new();
        field_a2.insert(kv(Some("h1"), None), fv(json!(1)));
        field_a2.insert(kv(Some("h2"), None), fv(json!(2)));

        schema_a.insert("f1".to_string(), field_a1);
        schema_a.insert("f2".to_string(), field_a2);

        let mut schema_b = HashMap::new();

        let mut field_b1 = HashMap::new();
        field_b1.insert(kv(Some("h1"), None), fv(json!(true)));
        field_b1.insert(kv(Some("h2"), None), fv(json!(false)));

        let mut field_b2 = HashMap::new();
        field_b2.insert(kv(Some("h1"), None), fv(json!(null)));
        field_b2.insert(kv(Some("h2"), None), fv(json!([1, 2, 3])));

        schema_b.insert("f1".to_string(), field_b1);
        schema_b.insert("f2".to_string(), field_b2);

        inputs.insert("SchemaA".to_string(), schema_a);
        inputs.insert("SchemaB".to_string(), schema_b);
        inputs
    }

    /// Pinned-forever known vector. Changing this hash means a breaking change
    /// to the canonical encoding — bump `Provenance::Derived::encoding_version`.
    #[test]
    fn hash_input_snapshot_known_vector() {
        let inputs = fixture_2x2x2();
        let got = hash_input_snapshot(&inputs);

        // Reconstruct the expected byte stream by hand in canonical order to
        // pin the encoding. If this test fails, the canonical layout has
        // drifted — bump encoding_version.
        let mut expected_bytes: Vec<u8> = Vec::new();
        let entries: [(&str, &str, &str, &[u8]); 8] = [
            ("SchemaA", "f1", "h1", b"\"a1-h1\""),
            ("SchemaA", "f1", "h2", b"\"a1-h2\""),
            ("SchemaA", "f2", "h1", b"1"),
            ("SchemaA", "f2", "h2", b"2"),
            ("SchemaB", "f1", "h1", b"true"),
            ("SchemaB", "f1", "h2", b"false"),
            ("SchemaB", "f2", "h1", b"null"),
            ("SchemaB", "f2", "h2", b"[1,2,3]"),
        ];
        for (schema, field, key, value) in entries {
            expected_bytes.extend_from_slice(schema.as_bytes());
            expected_bytes.push(0x00);
            expected_bytes.extend_from_slice(field.as_bytes());
            expected_bytes.push(0x00);
            expected_bytes.extend_from_slice(key.as_bytes());
            expected_bytes.push(0x00);
            expected_bytes.extend_from_slice(value);
            expected_bytes.push(0x00);
        }
        let expected_hex = format!("{:x}", Sha256::digest(&expected_bytes));

        assert_eq!(got, expected_hex);
        // Length sanity: SHA-256 hex is always 64 chars.
        assert_eq!(got.len(), 64);
    }

    /// Build the same logical snapshot with schema / field / key insertions
    /// in different orders — hash must be identical.
    #[test]
    fn hash_insensitive_to_insertion_order() {
        let canonical = hash_input_snapshot(&fixture_2x2x2());

        // Rebuild with reversed insertion order at every level.
        let mut inputs = HashMap::new();

        let mut schema_b = HashMap::new();
        let mut field_b2 = HashMap::new();
        field_b2.insert(kv(Some("h2"), None), fv(json!([1, 2, 3])));
        field_b2.insert(kv(Some("h1"), None), fv(json!(null)));
        let mut field_b1 = HashMap::new();
        field_b1.insert(kv(Some("h2"), None), fv(json!(false)));
        field_b1.insert(kv(Some("h1"), None), fv(json!(true)));
        schema_b.insert("f2".to_string(), field_b2);
        schema_b.insert("f1".to_string(), field_b1);

        let mut schema_a = HashMap::new();
        let mut field_a2 = HashMap::new();
        field_a2.insert(kv(Some("h2"), None), fv(json!(2)));
        field_a2.insert(kv(Some("h1"), None), fv(json!(1)));
        let mut field_a1 = HashMap::new();
        field_a1.insert(kv(Some("h2"), None), fv(json!("a1-h2")));
        field_a1.insert(kv(Some("h1"), None), fv(json!("a1-h1")));
        schema_a.insert("f2".to_string(), field_a2);
        schema_a.insert("f1".to_string(), field_a1);

        inputs.insert("SchemaB".to_string(), schema_b);
        inputs.insert("SchemaA".to_string(), schema_a);

        assert_eq!(hash_input_snapshot(&inputs), canonical);
    }

    /// Empty input hashes to SHA-256 of empty byte string.
    #[test]
    fn hash_empty_input_is_sha256_of_empty() {
        let got = hash_input_snapshot(&HashMap::new());
        // sha256("") — well-known vector.
        assert_eq!(
            got,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// Hash changes when any of schema, field, key, value changes.
    #[test]
    fn hash_sensitive_to_each_component() {
        fn one_entry(
            schema: &str,
            field: &str,
            key: KeyValue,
            value: serde_json::Value,
        ) -> HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>> {
            let mut e = HashMap::new();
            e.insert(key, fv(value));
            let mut f = HashMap::new();
            f.insert(field.to_string(), e);
            let mut s = HashMap::new();
            s.insert(schema.to_string(), f);
            s
        }

        let base = hash_input_snapshot(&one_entry("S", "f", kv(Some("k"), None), json!(1)));

        let diff_schema = hash_input_snapshot(&one_entry("S2", "f", kv(Some("k"), None), json!(1)));
        assert_ne!(diff_schema, base, "schema sensitivity");

        let diff_field = hash_input_snapshot(&one_entry("S", "f2", kv(Some("k"), None), json!(1)));
        assert_ne!(diff_field, base, "field sensitivity");

        let diff_key = hash_input_snapshot(&one_entry("S", "f", kv(Some("k2"), None), json!(1)));
        assert_ne!(diff_key, base, "key sensitivity");

        let diff_value = hash_input_snapshot(&one_entry("S", "f", kv(Some("k"), None), json!(2)));
        assert_ne!(diff_value, base, "value sensitivity");
    }

    /// Object values with different key insertion orders must produce the
    /// same input_snapshot_hash. Pins the canonical-JSON sort-keys behavior.
    #[test]
    fn hash_equal_across_nested_json_key_order() {
        let make = |value: serde_json::Value| {
            let mut e = HashMap::new();
            e.insert(kv(Some("k"), None), fv(value));
            let mut f = HashMap::new();
            f.insert("f".to_string(), e);
            let mut s = HashMap::new();
            s.insert("S".to_string(), f);
            s
        };

        // Same object, two different insertion orders. serde_json::json!
        // preserves order via Map's backing impl, so these actually differ
        // in bytes before canonicalization.
        let a = json!({"a": 1, "b": 2});
        let b = json!({"b": 2, "a": 1});

        assert_eq!(hash_input_snapshot(&make(a)), hash_input_snapshot(&make(b)));
    }

    /// Pins the canonical-JSON encoding directly so a silent change to
    /// `canonical_json_bytes` is caught without relying on the full
    /// hash-input-snapshot pipeline.
    #[test]
    fn canonicalizes_nested_object_keys() {
        let a = canonical_json_bytes(&json!({"a": 1, "b": {"y": 2, "x": 1}}));
        let b = canonical_json_bytes(&json!({"b": {"x": 1, "y": 2}, "a": 1}));
        assert_eq!(a, b);
        // Pin the exact byte output.
        assert_eq!(a, br#"{"a":1,"b":{"x":1,"y":2}}"#.to_vec());
    }
}
