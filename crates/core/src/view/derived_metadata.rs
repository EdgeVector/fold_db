//! Compute the `Provenance::Derived`-shaped metadata for a transform fire.
//!
//! Pure, deterministic helper. Produced during `ViewResolver::resolve` and
//! consumed downstream when transform output is routed through
//! `MutationManager` (project `view-compute-as-mutations`, PR 2). This module
//! is PR 1 — additive plumbing only; no production caller constructs a
//! `DerivedMetadata` yet.
//!
//! The three hash fields mirror `Provenance::Derived`:
//!
//! - `wasm_hash` — SHA-256 of the WASM module bytes (or of the empty byte
//!   string for identity views, signalling "no transform").
//! - `input_snapshot_hash` — canonical hash of the input slice, via
//!   [`crate::atom::input_snapshot::hash_input_snapshot`].
//! - `sources_merkle_root` — SHA-256 hex of the Merkle root over the set of
//!   source [`MoleculeRef`]s, via [`crate::atom::merkle::merkle_root`] with
//!   [`MoleculeRef::canonical_bytes`] as leaves.
//!
//! The extracted `sources` are a deterministic ordering of the source
//! molecules, suitable for the lineage forward index. A `FieldValue` is
//! eligible to become a source leaf iff it carries both `molecule_uuid` and
//! a non-empty `atom_uuid` — otherwise it came from a path that doesn't
//! correspond to a persisted atom (e.g. transform-output entries currently
//! synthesized with blank provenance, or override values) and is skipped.

use crate::atom::merkle::merkle_root;
use crate::atom::{input_snapshot, MoleculeRef};
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Derived-provenance inputs computed from a transform fire.
///
/// Carries everything needed to construct `Provenance::derived(...)` plus
/// the full source list for the lineage forward index. Used in PR 2
/// (`view-compute-as-mutations`) to build the `Mutation` that will persist
/// the transform output; here in PR 1 we only plumb the type and the
/// computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedMetadata {
    /// SHA-256 hex of the WASM module bytes. For identity views (no WASM)
    /// this is the SHA-256 of the empty byte string — a stable marker.
    pub wasm_hash: String,
    /// SHA-256 hex of the canonical input snapshot. See
    /// [`input_snapshot::hash_input_snapshot`] for the encoding contract.
    pub input_snapshot_hash: String,
    /// SHA-256 hex of the Merkle root over the source `MoleculeRef`
    /// canonical bytes. Empty source set hashes to SHA-256 of the empty
    /// string (pinned by `merkle_root` contract).
    pub sources_merkle_root: String,
    /// Deterministically-ordered source molecule references used as Merkle
    /// leaves. Ordering: by `molecule_uuid` ascending, then by the
    /// canonical string form of the key, then by `atom_uuid`, then by
    /// `written_at`. Deterministic across runs so the Merkle root is
    /// reproducible.
    pub sources: Vec<MoleculeRef>,
}

/// Compute the derived metadata for a transform fire.
///
/// `wasm_bytes` is the WASM module bytes, or an empty slice for identity
/// views. `query_results` is the same shape passed to
/// `ViewResolver::execute_wasm_transform` — the outermost key is the
/// schema name, middle key is the field name, innermost is the `KeyValue`
/// → `FieldValue` mapping.
///
/// Source extraction: each `FieldValue` with `molecule_uuid = Some(_)` and
/// a non-empty `atom_uuid` contributes one `MoleculeRef`. `written_at`
/// defaults to `0` when the read path did not populate it (pre-plumbing
/// call sites); downstream callers can recompute once the gap is closed.
/// Duplicate `MoleculeRef`s (same `(molecule_uuid, atom_uuid, key,
/// written_at)`) collapse to a single leaf — the Merkle root treats
/// sources as a set, not a multiset.
#[must_use]
pub fn compute_derived_metadata(
    wasm_bytes: &[u8],
    query_results: &HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>>,
) -> DerivedMetadata {
    let wasm_hash = {
        let mut hasher = Sha256::new();
        hasher.update(wasm_bytes);
        format!("{:x}", hasher.finalize())
    };

    let input_snapshot_hash = input_snapshot::hash_input_snapshot(query_results);

    let sources = extract_sources(query_results);
    let leaves: Vec<Vec<u8>> = sources.iter().map(MoleculeRef::canonical_bytes).collect();
    let root = merkle_root(&leaves);
    let sources_merkle_root = hex_lower(&root);

    DerivedMetadata {
        wasm_hash,
        input_snapshot_hash,
        sources_merkle_root,
        sources,
    }
}

/// Extract the deduplicated, canonically-ordered `MoleculeRef` source set
/// from a query-result map.
fn extract_sources(
    query_results: &HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>>,
) -> Vec<MoleculeRef> {
    let mut refs: Vec<MoleculeRef> = query_results
        .values()
        .flat_map(|fields| fields.values())
        .flat_map(|entries| entries.iter())
        .filter_map(|(kv, fv)| molecule_ref_from(kv, fv))
        .collect();

    refs.sort_by(|a, b| {
        a.molecule_uuid
            .cmp(&b.molecule_uuid)
            .then_with(|| canonical_key(&a.key).cmp(canonical_key(&b.key)))
            .then_with(|| a.atom_uuid.cmp(&b.atom_uuid))
            .then_with(|| a.written_at.cmp(&b.written_at))
    });
    refs.dedup();
    refs
}

fn molecule_ref_from(kv: &KeyValue, fv: &FieldValue) -> Option<MoleculeRef> {
    if fv.atom_uuid.is_empty() {
        return None;
    }
    let molecule_uuid = fv.molecule_uuid.clone()?;
    let key_str = kv.to_string();
    let key = if key_str.is_empty() {
        None
    } else {
        Some(key_str)
    };
    Some(MoleculeRef {
        molecule_uuid,
        atom_uuid: fv.atom_uuid.clone(),
        key,
        written_at: fv.written_at.unwrap_or(0),
    })
}

fn canonical_key(key: &Option<String>) -> &str {
    key.as_deref().unwrap_or("")
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{:02x}", b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fv(atom: &str, molecule: Option<&str>, written_at: Option<u64>) -> FieldValue {
        FieldValue {
            value: json!({ "v": atom }),
            atom_uuid: atom.to_string(),
            source_file_name: None,
            metadata: None,
            molecule_uuid: molecule.map(ToString::to_string),
            molecule_version: None,
            writer_pubkey: None,
            written_at,
        }
    }

    fn inputs_single(
        schema: &str,
        field: &str,
        key: KeyValue,
        value: FieldValue,
    ) -> HashMap<String, HashMap<String, HashMap<KeyValue, FieldValue>>> {
        let mut inner = HashMap::new();
        inner.insert(key, value);
        let mut fields = HashMap::new();
        fields.insert(field.to_string(), inner);
        let mut outer = HashMap::new();
        outer.insert(schema.to_string(), fields);
        outer
    }

    #[test]
    fn wasm_hash_matches_sha256_of_bytes() {
        let bytes = b"hello world";
        let md = compute_derived_metadata(bytes, &HashMap::new());
        let expected = {
            let mut h = Sha256::new();
            h.update(bytes);
            format!("{:x}", h.finalize())
        };
        assert_eq!(md.wasm_hash, expected);
    }

    #[test]
    fn empty_wasm_hashes_to_sha256_empty() {
        let md = compute_derived_metadata(&[], &HashMap::new());
        // SHA-256 of empty string.
        assert_eq!(
            md.wasm_hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn empty_inputs_merkle_root_is_sha256_empty() {
        let md = compute_derived_metadata(b"wasm", &HashMap::new());
        assert_eq!(
            md.sources_merkle_root,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert!(md.sources.is_empty());
    }

    #[test]
    fn sources_include_only_field_values_with_molecule_and_atom() {
        let key = KeyValue::new(None, Some("k1".to_string()));
        let good = fv("atom-1", Some("mol-1"), Some(10));
        let no_mol = fv("atom-2", None, Some(20));
        let no_atom = fv("", Some("mol-3"), Some(30));
        let mut inner = HashMap::new();
        inner.insert(key.clone(), good);
        inner.insert(KeyValue::new(None, Some("k2".to_string())), no_mol);
        inner.insert(KeyValue::new(None, Some("k3".to_string())), no_atom);
        let mut fields = HashMap::new();
        fields.insert("f".to_string(), inner);
        let mut inputs = HashMap::new();
        inputs.insert("S".to_string(), fields);

        let md = compute_derived_metadata(b"w", &inputs);
        assert_eq!(md.sources.len(), 1);
        assert_eq!(md.sources[0].atom_uuid, "atom-1");
        assert_eq!(md.sources[0].molecule_uuid, "mol-1");
        assert_eq!(md.sources[0].written_at, 10);
    }

    #[test]
    fn source_ordering_is_deterministic() {
        // Two entries with different molecule uuids; feed in opposite
        // insertion orders and verify source order is the same.
        let key_a = KeyValue::new(None, Some("a".to_string()));
        let key_b = KeyValue::new(None, Some("b".to_string()));
        let fa = fv("atom-a", Some("mol-2"), Some(1));
        let fb = fv("atom-b", Some("mol-1"), Some(1));

        let mut inner1 = HashMap::new();
        inner1.insert(key_a.clone(), fa.clone());
        inner1.insert(key_b.clone(), fb.clone());
        let mut fields1 = HashMap::new();
        fields1.insert("f".to_string(), inner1);
        let mut inputs1 = HashMap::new();
        inputs1.insert("S".to_string(), fields1);

        let mut inner2 = HashMap::new();
        inner2.insert(key_b.clone(), fb.clone());
        inner2.insert(key_a.clone(), fa.clone());
        let mut fields2 = HashMap::new();
        fields2.insert("f".to_string(), inner2);
        let mut inputs2 = HashMap::new();
        inputs2.insert("S".to_string(), fields2);

        let md1 = compute_derived_metadata(b"w", &inputs1);
        let md2 = compute_derived_metadata(b"w", &inputs2);
        assert_eq!(md1.sources, md2.sources);
        assert_eq!(md1.sources_merkle_root, md2.sources_merkle_root);
        // mol-1 sorts before mol-2.
        assert_eq!(md1.sources[0].molecule_uuid, "mol-1");
        assert_eq!(md1.sources[1].molecule_uuid, "mol-2");
    }

    #[test]
    fn duplicate_sources_collapse_to_single_leaf() {
        let key = KeyValue::new(None, Some("k".to_string()));
        let f = fv("atom-1", Some("mol-1"), Some(42));
        let inputs_once = inputs_single("S1", "f", key.clone(), f.clone());
        let mut inputs_twice_fields = HashMap::new();
        inputs_twice_fields.insert("f".to_string(), {
            let mut inner = HashMap::new();
            inner.insert(key.clone(), f.clone());
            inner
        });
        inputs_twice_fields.insert("g".to_string(), {
            let mut inner = HashMap::new();
            inner.insert(key.clone(), f.clone());
            inner
        });
        let mut inputs_twice = HashMap::new();
        inputs_twice.insert("S1".to_string(), inputs_twice_fields);

        let md_once = compute_derived_metadata(b"w", &inputs_once);
        let md_twice = compute_derived_metadata(b"w", &inputs_twice);
        // Sources are a set — the two identical refs collapse.
        assert_eq!(md_once.sources, md_twice.sources);
        assert_eq!(md_once.sources_merkle_root, md_twice.sources_merkle_root);
        // input_snapshot_hash WILL differ (one has field g, one doesn't).
        assert_ne!(md_once.input_snapshot_hash, md_twice.input_snapshot_hash);
    }

    #[test]
    fn input_snapshot_hash_matches_direct_call() {
        let key = KeyValue::new(None, Some("k".to_string()));
        let f = fv("atom-1", Some("mol-1"), Some(7));
        let inputs = inputs_single("S", "f", key, f);

        let md = compute_derived_metadata(b"w", &inputs);
        let direct = input_snapshot::hash_input_snapshot(&inputs);
        assert_eq!(md.input_snapshot_hash, direct);
    }

    #[test]
    fn merkle_root_known_vector_for_single_source() {
        // One source → Merkle root is SHA-256 of its canonical bytes.
        let key = KeyValue::new(None, Some("k".to_string()));
        let f = fv("atom-1", Some("mol-1"), Some(0x0102_0304_0506_0708));
        let inputs = inputs_single("S", "f", key.clone(), f);
        let md = compute_derived_metadata(b"w", &inputs);

        let r = &md.sources[0];
        let expected = {
            let mut h = Sha256::new();
            h.update(r.canonical_bytes());
            format!("{:x}", h.finalize())
        };
        assert_eq!(md.sources_merkle_root, expected);
    }

    #[test]
    fn written_at_none_defaults_to_zero_in_ref() {
        let key = KeyValue::new(None, Some("k".to_string()));
        let f = fv("atom-1", Some("mol-1"), None);
        let inputs = inputs_single("S", "f", key, f);
        let md = compute_derived_metadata(b"w", &inputs);
        assert_eq!(md.sources[0].written_at, 0);
    }
}
