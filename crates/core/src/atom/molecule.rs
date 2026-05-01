use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{deterministic_molecule_uuid, now_nanos, KeyMetadata, MergeConflict};
use crate::atom::provenance::Provenance;
use crate::security::Ed25519KeyPair;

/// A reference to a single atom version.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Molecule {
    molecule_uuid: String,
    /// The current atom entry with write timestamp.
    /// Kept as a flattened pair for backward-compat: old data without
    /// `written_at` will deserialize with `written_at: 0` via serde default.
    atom_uuid: String,
    #[serde(default)]
    written_at: u64,
    #[schema(value_type = String, format = "date-time")]
    updated_at: DateTime<Utc>,
    #[serde(default)]
    version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key_metadata: Option<KeyMetadata>,
    /// Base64-encoded public key of the writer who last signed this molecule.
    #[serde(default)]
    writer_pubkey: String,
    /// Base64-encoded Ed25519 signature over canonical bytes.
    #[serde(default)]
    signature: String,
    /// Signature scheme version (1 = hand-rolled canonical concat).
    #[serde(default)]
    signature_version: u8,
    /// Writer identity and verifiability info. Additive during the
    /// `projects/molecule-provenance-dag` migration: `None` on pre-PR-5
    /// molecules; `Some(Provenance::User{..})` once the signing path
    /// populates it. Kept alongside `writer_pubkey` / `signature` /
    /// `signature_version` (not in place of) until a follow-up PR removes
    /// them after full wire-through. NOT included in the canonical signed
    /// bytes — it duplicates pubkey + signature in a typed form and must
    /// not perturb the signature verification payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provenance: Option<Provenance>,
}

impl Molecule {
    /// Creates a new unsigned Molecule with a deterministic UUID derived from schema + field name.
    /// The molecule starts unsigned (empty strings, version 0) — call `set_atom_uuid` to sign.
    #[must_use]
    pub fn new(atom_uuid: String, schema_name: &str, field_name: &str) -> Self {
        Self {
            molecule_uuid: deterministic_molecule_uuid(schema_name, field_name),
            atom_uuid,
            written_at: now_nanos(),
            updated_at: Utc::now(),
            version: 0,
            key_metadata: None,
            writer_pubkey: String::new(),
            signature: String::new(),
            signature_version: 0,
            provenance: None,
        }
    }

    /// Returns the unique identifier of this molecule.
    #[must_use]
    pub fn uuid(&self) -> &str {
        &self.molecule_uuid
    }

    /// Returns the timestamp of the last update.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Updates the reference to point to a new Atom version and signs the molecule.
    /// Bumps the version counter only when the atom actually changes.
    pub fn set_atom_uuid(&mut self, atom_uuid: String, keypair: &Ed25519KeyPair) {
        if self.atom_uuid != atom_uuid {
            self.version += 1;
        }
        self.atom_uuid = atom_uuid;
        self.written_at = now_nanos();
        self.updated_at = Utc::now();

        // Build canonical bytes and sign
        let canonical = Self::build_canonical_bytes(
            &self.molecule_uuid,
            &self.atom_uuid,
            self.version,
            self.written_at,
        );
        let (sig, pubkey) = crate::security::sign_molecule_update(&canonical, keypair);
        self.signature = sig.clone();
        self.writer_pubkey = pubkey.clone();
        self.signature_version = 1;
        self.provenance = Some(Provenance::user(pubkey, sig));
        self.debug_assert_provenance_consistent();
    }

    /// Returns the version counter for this molecule.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Returns the UUID of the referenced Atom.
    #[must_use]
    pub fn get_atom_uuid(&self) -> &String {
        &self.atom_uuid
    }

    /// Returns the write timestamp (nanos since epoch) for the current atom.
    #[must_use]
    pub fn written_at(&self) -> u64 {
        self.written_at
    }

    /// Sets per-key metadata on the molecule.
    pub fn set_key_metadata(&mut self, meta: KeyMetadata) {
        self.key_metadata = Some(meta);
    }

    /// Returns the per-key metadata, if any.
    #[must_use]
    pub fn get_key_metadata(&self) -> Option<&KeyMetadata> {
        self.key_metadata.as_ref()
    }

    /// Updates the atom reference WITHOUT signing.
    /// Only for ephemeral in-memory operations (e.g., rewind for time-travel queries).
    /// The resulting molecule will not pass `verify()`.
    pub(crate) fn set_atom_uuid_unsigned(&mut self, atom_uuid: String) {
        if self.atom_uuid != atom_uuid {
            self.version += 1;
        }
        self.atom_uuid = atom_uuid;
        self.written_at = now_nanos();
        self.updated_at = Utc::now();
        // Intentionally leave signature fields untouched (or stale)
    }

    /// Returns the base64-encoded public key of the writer who last signed this molecule.
    #[must_use]
    pub fn writer_pubkey(&self) -> &str {
        &self.writer_pubkey
    }

    /// Returns the base64-encoded signature.
    #[must_use]
    pub fn signature(&self) -> &str {
        &self.signature
    }

    /// Returns the signature version.
    #[must_use]
    pub fn signature_version(&self) -> u8 {
        self.signature_version
    }

    /// Returns the typed provenance, when populated.
    ///
    /// Additive metadata alongside `writer_pubkey` / `signature` during the
    /// `projects/molecule-provenance-dag` migration. `None` on molecules
    /// written before PR 5 (or never signed).
    #[must_use]
    pub fn provenance(&self) -> Option<&Provenance> {
        self.provenance.as_ref()
    }

    /// Debug-only assertion that `provenance` and the legacy
    /// `writer_pubkey` / `signature` fields agree when both are populated.
    ///
    /// During the migration window `Provenance::User` duplicates the
    /// existing fields. If a code path sets one and not the other — or
    /// sets them to different values — that drift is a bug. In debug
    /// builds this fires on the spot; in release it compiles out.
    fn debug_assert_provenance_consistent(&self) {
        debug_assert!(
            matches!(
                &self.provenance,
                Some(Provenance::User {
                    pubkey,
                    signature,
                    ..
                }) if pubkey == &self.writer_pubkey && signature == &self.signature
            ),
            "Molecule.provenance drift: writer_pubkey={:?} signature={:?} provenance={:?}",
            self.writer_pubkey,
            self.signature,
            self.provenance
        );
    }

    /// Builds canonical bytes for signing/verification.
    /// Layout: molecule_uuid | 0x00 | atom_uuid | 0x00 | version(u64 BE) | written_at(u64 BE)
    fn build_canonical_bytes(
        molecule_uuid: &str,
        atom_uuid: &str,
        version: u64,
        written_at: u64,
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(molecule_uuid.as_bytes());
        buf.push(0x00);
        buf.extend_from_slice(atom_uuid.as_bytes());
        buf.push(0x00);
        buf.extend_from_slice(&version.to_be_bytes());
        buf.extend_from_slice(&written_at.to_be_bytes());
        buf
    }

    /// Verifies the molecule's signature against its canonical bytes.
    /// Returns false for unsigned molecules (signature_version == 0).
    #[must_use]
    pub fn verify(&self) -> bool {
        if self.signature_version == 0 {
            return false;
        }
        let canonical = Self::build_canonical_bytes(
            &self.molecule_uuid,
            &self.atom_uuid,
            self.version,
            self.written_at,
        );
        crate::security::verify_molecule_signature(&canonical, &self.signature, &self.writer_pubkey)
    }

    /// Merges another Molecule into this one using last-writer-wins.
    /// If both have different atom_uuids, the one with a later `written_at` wins.
    /// Returns a `MergeConflict` if there was a genuine conflict (different atoms).
    /// The merge result is signed by the provided keypair.
    pub fn merge(&mut self, other: &Molecule, keypair: &Ed25519KeyPair) -> Option<MergeConflict> {
        if self.atom_uuid == other.atom_uuid {
            return None;
        }
        let (winner_atom, loser_atom, winner_ts, loser_ts) = if other.written_at >= self.written_at
        {
            (
                other.atom_uuid.clone(),
                self.atom_uuid.clone(),
                other.written_at,
                self.written_at,
            )
        } else {
            (
                self.atom_uuid.clone(),
                other.atom_uuid.clone(),
                self.written_at,
                other.written_at,
            )
        };
        self.atom_uuid = winner_atom.clone();
        self.written_at = winner_ts;
        self.version += 1;
        self.updated_at = Utc::now();

        // Sign the merge result
        let canonical = Self::build_canonical_bytes(
            &self.molecule_uuid,
            &self.atom_uuid,
            self.version,
            self.written_at,
        );
        let (sig, pubkey) = crate::security::sign_molecule_update(&canonical, keypair);
        self.signature = sig.clone();
        self.writer_pubkey = pubkey.clone();
        self.signature_version = 1;
        self.provenance = Some(Provenance::user(pubkey, sig));
        self.debug_assert_provenance_consistent();

        Some(MergeConflict {
            key: "single".to_string(),
            winner_atom,
            loser_atom,
            winner_written_at: winner_ts,
            loser_written_at: loser_ts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Ed25519KeyPair;

    fn test_keypair() -> Ed25519KeyPair {
        Ed25519KeyPair::generate().unwrap()
    }

    #[test]
    fn test_version_starts_at_zero() {
        let mol = Molecule::new("atom-1".to_string(), "schema", "field");
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_version_bumps_on_change() {
        let kp = test_keypair();
        let mut mol = Molecule::new("atom-1".to_string(), "schema", "field");
        mol.set_atom_uuid("atom-2".to_string(), &kp);
        assert_eq!(mol.version(), 1);
        mol.set_atom_uuid("atom-3".to_string(), &kp);
        assert_eq!(mol.version(), 2);
    }

    #[test]
    fn test_version_no_bump_on_same_value() {
        let kp = test_keypair();
        let mut mol = Molecule::new("atom-1".to_string(), "schema", "field");
        mol.set_atom_uuid("atom-1".to_string(), &kp);
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_deterministic_uuid() {
        let mol1 = Molecule::new("atom-1".to_string(), "my_schema", "my_field");
        let mol2 = Molecule::new("atom-2".to_string(), "my_schema", "my_field");
        assert_eq!(
            mol1.uuid(),
            mol2.uuid(),
            "same schema+field => same molecule UUID"
        );
    }

    #[test]
    fn test_merge_no_conflict_same_atom() {
        let kp = test_keypair();
        let mut mol1 = Molecule::new("atom-1".to_string(), "s", "f");
        let mol2 = Molecule::new("atom-1".to_string(), "s", "f");
        assert!(mol1.merge(&mol2, &kp).is_none());
    }

    #[test]
    fn test_merge_conflict_later_wins() {
        let kp = test_keypair();
        let mut mol1 = Molecule::new("atom-1".to_string(), "s", "f");
        std::thread::sleep(std::time::Duration::from_millis(1));
        let mol2 = Molecule::new("atom-2".to_string(), "s", "f");
        let conflict = mol1.merge(&mol2, &kp).expect("should conflict");
        assert_eq!(conflict.winner_atom, "atom-2");
        assert_eq!(conflict.loser_atom, "atom-1");
        assert_eq!(mol1.get_atom_uuid(), "atom-2");
    }

    #[test]
    fn test_written_at_updates_on_set() {
        let kp = test_keypair();
        let mut mol = Molecule::new("atom-1".to_string(), "s", "f");
        let ts1 = mol.written_at();
        std::thread::sleep(std::time::Duration::from_millis(1));
        mol.set_atom_uuid("atom-2".to_string(), &kp);
        assert!(mol.written_at() >= ts1);
    }

    #[test]
    fn test_sign_verify_round_trip() {
        let kp = test_keypair();
        let mut mol = Molecule::new("atom-1".to_string(), "s", "f");
        mol.set_atom_uuid("atom-2".to_string(), &kp);
        assert!(mol.verify(), "signature should verify after set_atom_uuid");
        assert_eq!(mol.signature_version(), 1);
        assert!(!mol.writer_pubkey().is_empty());
        assert!(!mol.signature().is_empty());
    }

    #[test]
    fn test_tamper_detection() {
        let kp = test_keypair();
        let mut mol = Molecule::new("atom-1".to_string(), "s", "f");
        mol.set_atom_uuid("atom-2".to_string(), &kp);
        assert!(mol.verify());
        // Tamper with atom_uuid
        mol.atom_uuid = "atom-tampered".to_string();
        assert!(!mol.verify(), "signature should fail after tampering");
    }

    #[test]
    fn test_wrong_key_detection() {
        let kp_a = test_keypair();
        let kp_b = test_keypair();
        let mut mol = Molecule::new("atom-1".to_string(), "s", "f");
        mol.set_atom_uuid("atom-2".to_string(), &kp_a);
        assert!(mol.verify());
        // Swap writer_pubkey to key B's pubkey
        mol.writer_pubkey = kp_b.public_key_base64();
        assert!(!mol.verify(), "signature should fail with wrong key");
    }

    #[test]
    fn test_unsigned_molecule_verify_returns_false() {
        let mol = Molecule::new("atom-1".to_string(), "s", "f");
        assert!(!mol.verify(), "unsigned molecule should not verify");
    }

    // ------------------------------------------------------------------
    // molecule-provenance-dag PR 5 — additive `provenance: Option<Provenance>`.
    // ------------------------------------------------------------------

    /// Pre-PR-5 serialized shape — no `provenance` field. This is what sits
    /// in every sync log and every on-disk molecule written before the
    /// migration. It must deserialize cleanly into `provenance: None` and
    /// re-serialize byte-for-byte identical.
    const GOLDEN_PRE_PR5_MOLECULE_JSON: &str = r#"{"molecule_uuid":"mol-1","atom_uuid":"atom-1","written_at":42,"updated_at":"2026-01-01T00:00:00Z","version":1,"writer_pubkey":"","signature":"","signature_version":0}"#;

    #[test]
    fn pre_pr5_molecule_json_round_trips_unchanged() {
        let parsed: Molecule = serde_json::from_str(GOLDEN_PRE_PR5_MOLECULE_JSON)
            .expect("deserialize pre-PR-5 molecule shape");
        assert!(parsed.provenance().is_none());
        let reserialized = serde_json::to_string(&parsed).expect("serialize");
        assert_eq!(reserialized, GOLDEN_PRE_PR5_MOLECULE_JSON);
    }

    #[test]
    fn molecule_round_trips_with_provenance_user() {
        let kp = test_keypair();
        let mut mol = Molecule::new("atom-1".to_string(), "s", "f");
        mol.set_atom_uuid("atom-2".to_string(), &kp);
        assert!(matches!(mol.provenance(), Some(Provenance::User { .. })));
        let json = serde_json::to_string(&mol).expect("serialize");
        assert!(json.contains(r#""provenance":{"kind":"user""#));
        let back: Molecule = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.provenance(), mol.provenance());
    }

    /// When a molecule is signed via `set_atom_uuid`, the `Provenance::User`
    /// value must match the legacy `writer_pubkey` / `signature` fields.
    /// This is the whole point of the additive migration.
    #[test]
    fn set_atom_uuid_populates_provenance_user_matching_legacy_fields() {
        let kp = test_keypair();
        let mut mol = Molecule::new("atom-1".to_string(), "s", "f");
        mol.set_atom_uuid("atom-2".to_string(), &kp);
        let prov = mol.provenance().expect("signed molecule has provenance");
        match prov {
            Provenance::User {
                pubkey,
                signature,
                signature_version,
            } => {
                assert_eq!(pubkey, mol.writer_pubkey());
                assert_eq!(signature, mol.signature());
                assert_eq!(*signature_version, 1);
            }
            _ => panic!("expected User variant"),
        }
    }

    /// `merge`'s sign-after-merge path must populate provenance too,
    /// matching the merged-in signature.
    #[test]
    fn merge_populates_provenance_user_matching_legacy_fields() {
        let kp = test_keypair();
        let mut mol1 = Molecule::new("atom-1".to_string(), "s", "f");
        std::thread::sleep(std::time::Duration::from_millis(1));
        let mol2 = Molecule::new("atom-2".to_string(), "s", "f");
        mol1.merge(&mol2, &kp).expect("should conflict");
        let prov = mol1
            .provenance()
            .expect("merge-signed molecule has provenance");
        match prov {
            Provenance::User {
                pubkey, signature, ..
            } => {
                assert_eq!(pubkey, mol1.writer_pubkey());
                assert_eq!(signature, mol1.signature());
            }
            _ => panic!("expected User variant"),
        }
    }

    /// Adding `provenance` must not change the canonical signed bytes.
    /// The canonical bytes commit to `molecule_uuid | atom_uuid | version |
    /// written_at`; anything else included there invalidates every
    /// pre-existing signature. Pinned with a known hex vector.
    #[test]
    fn canonical_bytes_are_not_affected_by_provenance_field() {
        let bytes =
            Molecule::build_canonical_bytes("mol-x", "atom-x", 3, 0x0102_0304_0506_0708_u64);
        let expected: Vec<u8> = [
            b"mol-x".as_slice(),
            &[0x00],
            b"atom-x".as_slice(),
            &[0x00],
            &[0, 0, 0, 0, 0, 0, 0, 3],
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
        ]
        .concat();
        assert_eq!(bytes, expected);
    }
}
