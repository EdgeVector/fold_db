//! Provenance types for molecules.
//!
//! User writes carry a signature; derived writes (future transform output)
//! carry a cryptographic pointer to the WASM module, input snapshot, and
//! source-molecule Merkle root. Both variants are defined here; neither is
//! wired through `Mutation` / `Molecule` / `AtomEntry` yet — see
//! `gbrain get projects/molecule-provenance-dag` for the 6-PR arc this is
//! step 1 of.

use serde::{Deserialize, Serialize};

/// Writer identity and verifiability information for a molecule.
///
/// `User` — signed by an end-user's keypair; authority is by signature.
/// `Derived` — produced by a deterministic WASM transform; unsigned.
/// Authority is by recomputation: given `wasm_hash` + `input_snapshot_hash`,
/// any node can re-run the transform and check the output matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Provenance {
    /// User-originated write. Signed by the user's Ed25519 keypair.
    User {
        /// Base64-encoded Ed25519 public key of the signer.
        pubkey: String,
        /// Base64-encoded Ed25519 signature over canonical bytes.
        signature: String,
        /// Signature scheme version (1 = hand-rolled canonical concat,
        /// matching `Molecule::build_canonical_bytes`).
        signature_version: u8,
    },
    /// Transform-output write. Unsigned; verifiable by recomputation.
    Derived {
        /// SHA-256 hex of the WASM module bytes that produced this molecule.
        wasm_hash: String,
        /// SHA-256 hex of the canonical input snapshot fed to WASM. This is
        /// the content address of the transform inputs.
        input_snapshot_hash: String,
        /// SHA-256 hex of the Merkle root over the source `MoleculeRef`s.
        /// The full source set lives in local rebuildable indexes (PR 6),
        /// not on the molecule.
        sources_merkle_root: String,
        /// Canonicalization version for `input_snapshot_hash` and the Merkle
        /// leaves. Starts at 1. Bump if and only if the canonical byte layout
        /// changes — a change here changes the content address, so treat as
        /// forever.
        encoding_version: u8,
    },
}

impl Provenance {
    /// Constructor for `User` variant with `signature_version = 1` (the only
    /// version currently defined).
    #[must_use]
    pub fn user(pubkey: String, signature: String) -> Self {
        Self::User {
            pubkey,
            signature,
            signature_version: 1,
        }
    }

    /// Constructor for `Derived` variant with `encoding_version = 1` (the only
    /// version currently defined).
    #[must_use]
    pub fn derived(
        wasm_hash: String,
        input_snapshot_hash: String,
        sources_merkle_root: String,
    ) -> Self {
        Self::Derived {
            wasm_hash,
            input_snapshot_hash,
            sources_merkle_root,
            encoding_version: 1,
        }
    }
}

/// Canonical reference to a single atom version on a single molecule.
///
/// Used as a Merkle leaf for `Provenance::Derived::sources_merkle_root` and
/// as the payload for the forward/reverse lineage indexes (PR 6). The
/// `written_at` pins recomputation to the exact source version even if the
/// molecule has moved on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MoleculeRef {
    pub molecule_uuid: String,
    pub atom_uuid: String,
    /// `None` for single-keyed molecules, `Some(k)` for range-keyed molecules.
    pub key: Option<String>,
    /// Nanoseconds since the Unix epoch at which this atom was written.
    pub written_at: u64,
}

impl MoleculeRef {
    /// Canonical byte encoding used as a Merkle leaf.
    ///
    /// Layout (stable forever — bump `Provenance::Derived::encoding_version`
    /// if this ever changes):
    ///
    /// ```text
    /// molecule_uuid | 0x00 | atom_uuid | 0x00 | key_or_empty | 0x00 | written_at(u64 BE)
    /// ```
    ///
    /// `key_or_empty` is the empty byte string when `key` is `None`, and the
    /// UTF-8 bytes of the key otherwise. Mirrors the pattern in
    /// `molecule.rs::build_canonical_bytes` (lines 149-163).
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.molecule_uuid.as_bytes());
        buf.push(0x00);
        buf.extend_from_slice(self.atom_uuid.as_bytes());
        buf.push(0x00);
        if let Some(k) = &self.key {
            buf.extend_from_slice(k.as_bytes());
        }
        buf.push(0x00);
        buf.extend_from_slice(&self.written_at.to_be_bytes());
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_serde_round_trip() {
        let p = Provenance::User {
            pubkey: "pubkey-b64".to_string(),
            signature: "sig-b64".to_string(),
            signature_version: 1,
        };
        let json = serde_json::to_string(&p).expect("serialize");
        let back: Provenance = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p, back);
    }

    #[test]
    fn derived_serde_round_trip() {
        let p = Provenance::Derived {
            wasm_hash: "a".repeat(64),
            input_snapshot_hash: "b".repeat(64),
            sources_merkle_root: "c".repeat(64),
            encoding_version: 1,
        };
        let json = serde_json::to_string(&p).expect("serialize");
        let back: Provenance = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p, back);
    }

    #[test]
    fn user_constructor_defaults_signature_version_to_1() {
        let p = Provenance::user("pk".to_string(), "sig".to_string());
        match p {
            Provenance::User {
                signature_version, ..
            } => assert_eq!(signature_version, 1),
            _ => panic!("expected User variant"),
        }
    }

    #[test]
    fn derived_constructor_defaults_encoding_version_to_1() {
        let p = Provenance::derived("wasm".to_string(), "input".to_string(), "root".to_string());
        match p {
            Provenance::Derived {
                encoding_version, ..
            } => assert_eq!(encoding_version, 1),
            _ => panic!("expected Derived variant"),
        }
    }

    #[test]
    fn molecule_ref_canonical_bytes_known_vector_with_key() {
        let r = MoleculeRef {
            molecule_uuid: "mol".to_string(),
            atom_uuid: "atom".to_string(),
            key: Some("k1".to_string()),
            written_at: 0x0102_0304_0506_0708_u64,
        };
        let expected: Vec<u8> = [
            b"mol".as_slice(),
            &[0x00],
            b"atom".as_slice(),
            &[0x00],
            b"k1".as_slice(),
            &[0x00],
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
        ]
        .concat();
        assert_eq!(r.canonical_bytes(), expected);
    }

    #[test]
    fn molecule_ref_canonical_bytes_known_vector_without_key() {
        let r = MoleculeRef {
            molecule_uuid: "mol".to_string(),
            atom_uuid: "atom".to_string(),
            key: None,
            written_at: 0,
        };
        // `None` key collapses to an empty byte string between the two 0x00
        // separators, leaving two adjacent 0x00 bytes. written_at = 0 → 8 zero
        // bytes.
        let expected: Vec<u8> = [
            b"mol".as_slice(),
            &[0x00],
            b"atom".as_slice(),
            &[0x00, 0x00],
            &[0, 0, 0, 0, 0, 0, 0, 0],
        ]
        .concat();
        assert_eq!(r.canonical_bytes(), expected);
    }

    #[test]
    fn molecule_ref_canonical_bytes_sensitive_to_every_field() {
        let base = MoleculeRef {
            molecule_uuid: "mol".to_string(),
            atom_uuid: "atom".to_string(),
            key: Some("k".to_string()),
            written_at: 42,
        };
        let base_bytes = base.canonical_bytes();

        let mut m1 = base.clone();
        m1.molecule_uuid = "mol2".to_string();
        assert_ne!(
            m1.canonical_bytes(),
            base_bytes,
            "molecule_uuid sensitivity"
        );

        let mut m2 = base.clone();
        m2.atom_uuid = "atom2".to_string();
        assert_ne!(m2.canonical_bytes(), base_bytes, "atom_uuid sensitivity");

        let mut m3 = base.clone();
        m3.key = Some("k2".to_string());
        assert_ne!(m3.canonical_bytes(), base_bytes, "key sensitivity");

        let mut m4 = base.clone();
        m4.written_at = 43;
        assert_ne!(m4.canonical_bytes(), base_bytes, "written_at sensitivity");
    }

    #[test]
    fn molecule_ref_canonical_bytes_distinguishes_none_from_empty_string_key() {
        let with_none = MoleculeRef {
            molecule_uuid: "mol".to_string(),
            atom_uuid: "atom".to_string(),
            key: None,
            written_at: 0,
        };
        let with_empty = MoleculeRef {
            molecule_uuid: "mol".to_string(),
            atom_uuid: "atom".to_string(),
            key: Some(String::new()),
            written_at: 0,
        };
        // Both produce identical bytes by design — `key_or_empty` is empty in
        // both cases. Nothing else can disambiguate them at the Merkle leaf
        // level. Document this by asserting it, so a future change that splits
        // the two has to delete the assertion.
        assert_eq!(with_none.canonical_bytes(), with_empty.canonical_bytes());
    }

    #[test]
    fn molecule_ref_serde_round_trip() {
        let r = MoleculeRef {
            molecule_uuid: "mol".to_string(),
            atom_uuid: "atom".to_string(),
            key: Some("k".to_string()),
            written_at: 99,
        };
        let json = serde_json::to_string(&r).expect("serialize");
        let back: MoleculeRef = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }
}
