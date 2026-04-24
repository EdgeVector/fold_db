//! Forward + reverse lineage indexes for derived molecules.
//!
//! PR 6 of `projects/molecule-provenance-dag`. Scaffolding only — no
//! production call sites. A future project (`view-compute-as-mutations`)
//! wires `LineageIndex::insert` into `MutationManager` when derived
//! molecules land on view schemas.
//!
//! # What this stores
//!
//! - **Forward** (`lineage_forward`): `derived_molecule_uuid -> Vec<MoleculeRef>`.
//!   "List all sources of Y."
//! - **Reverse** (`lineage_reverse`): `MoleculeRef::canonical_bytes() ->
//!   Vec<String>` of derived UUIDs. "List all derivatives of X." This is the
//!   redaction-case index.
//!
//! Both namespaces are **local-only**. `SyncingNamespacedStore` declares them
//! in `LOCAL_ONLY_NAMESPACES` so writes never enter the sync log. Rebuilding
//! either index is possible by replaying local molecule state — see
//! [`LineageIndex::verify_merkle_consistency`] for the consistency check used
//! during replay to confirm stored sources match the derived molecule's
//! `Provenance::Derived::sources_merkle_root`.
//!
//! # What this does not do
//!
//! - Does not construct `Provenance::Derived` molecules — that is PR of
//!   project 2.
//! - Does not run rebuild-from-replay end-to-end. Only the Merkle consistency
//!   helper is shipped here; the full replay loop is part of project 2.
//! - Is not atomic across the forward/reverse pair. `insert` and `remove`
//!   perform read-modify-write on the reverse entries. With no production
//!   callers in this PR, that is acceptable; the future project must add
//!   serialisation if concurrent writers touch the same derived UUID.

use crate::atom::{merkle::merkle_root, provenance::MoleculeRef};
use crate::error::FoldDbError;
use crate::storage::traits::KvStore;
use std::sync::Arc;

/// Two-way index from derived molecules to their source `MoleculeRef` set,
/// and back.
#[derive(Clone)]
pub struct LineageIndex {
    forward: Arc<dyn KvStore>,
    reverse: Arc<dyn KvStore>,
}

impl LineageIndex {
    pub(crate) fn new(forward: Arc<dyn KvStore>, reverse: Arc<dyn KvStore>) -> Self {
        Self { forward, reverse }
    }

    /// Record that `derived_uuid` was derived from `sources`.
    ///
    /// Writes the forward entry (`derived_uuid -> sources`) and, for every
    /// source, appends `derived_uuid` into the reverse entry keyed by the
    /// source's canonical bytes. Reverse entries are kept sorted and deduped
    /// so [`get_reverse`] returns a stable order regardless of insertion
    /// sequence.
    pub async fn insert(
        &self,
        derived_uuid: &str,
        sources: &[MoleculeRef],
    ) -> Result<(), FoldDbError> {
        let forward_bytes = serde_json::to_vec(sources)?;
        self.forward
            .put(derived_uuid.as_bytes(), forward_bytes)
            .await?;

        for source in sources {
            let key = source.canonical_bytes();
            let mut derivatives = Self::read_reverse(&self.reverse, &key).await?;
            if !derivatives.iter().any(|d| d == derived_uuid) {
                derivatives.push(derived_uuid.to_string());
                derivatives.sort();
            }
            let bytes = serde_json::to_vec(&derivatives)?;
            self.reverse.put(&key, bytes).await?;
        }

        Ok(())
    }

    /// Look up the source `MoleculeRef`s for a derived molecule.
    pub async fn get_forward(
        &self,
        derived_uuid: &str,
    ) -> Result<Option<Vec<MoleculeRef>>, FoldDbError> {
        match self.forward.get(derived_uuid.as_bytes()).await? {
            Some(bytes) => {
                let sources: Vec<MoleculeRef> = serde_json::from_slice(&bytes)?;
                Ok(Some(sources))
            }
            None => Ok(None),
        }
    }

    /// List every derived molecule that consumed `source_ref`. Returns an
    /// empty `Vec` when the source has no known derivatives. The returned
    /// list is sorted ascending — callers may rely on this ordering.
    pub async fn get_reverse(&self, source_ref: &MoleculeRef) -> Result<Vec<String>, FoldDbError> {
        let key = source_ref.canonical_bytes();
        Self::read_reverse(&self.reverse, &key).await
    }

    /// Drop every trace of `derived_uuid`. Removes the forward entry, then
    /// removes `derived_uuid` from every reverse entry its sources recorded.
    /// Reverse entries that become empty are deleted outright so scans stay
    /// clean.
    pub async fn remove(&self, derived_uuid: &str) -> Result<(), FoldDbError> {
        let sources = match self.forward.get(derived_uuid.as_bytes()).await? {
            Some(bytes) => {
                let parsed: Vec<MoleculeRef> = serde_json::from_slice(&bytes)?;
                parsed
            }
            None => return Ok(()),
        };

        self.forward.delete(derived_uuid.as_bytes()).await?;

        for source in sources {
            let key = source.canonical_bytes();
            let mut derivatives = Self::read_reverse(&self.reverse, &key).await?;
            derivatives.retain(|d| d != derived_uuid);
            if derivatives.is_empty() {
                self.reverse.delete(&key).await?;
            } else {
                let bytes = serde_json::to_vec(&derivatives)?;
                self.reverse.put(&key, bytes).await?;
            }
        }

        Ok(())
    }

    /// Check that `merkle_root(sources.canonical_bytes())` matches
    /// `expected_root_hex`. Used during rebuild-from-replay to confirm that
    /// the stored source list is consistent with the derived molecule's
    /// on-wire `Provenance::Derived::sources_merkle_root`. Any mismatch —
    /// reorder, added source, dropped source, mutated field — flips the
    /// return value to `false`.
    #[must_use]
    pub fn verify_merkle_consistency(sources: &[MoleculeRef], expected_root_hex: &str) -> bool {
        let leaves: Vec<Vec<u8>> = sources.iter().map(MoleculeRef::canonical_bytes).collect();
        let computed = merkle_root(&leaves);
        let computed_hex: String = computed.iter().map(|b| format!("{:02x}", b)).collect();
        computed_hex == expected_root_hex
    }

    async fn read_reverse(
        reverse: &Arc<dyn KvStore>,
        key: &[u8],
    ) -> Result<Vec<String>, FoldDbError> {
        match reverse.get(key).await? {
            Some(bytes) => {
                let parsed: Vec<String> = serde_json::from_slice(&bytes)?;
                Ok(parsed)
            }
            None => Ok(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::merkle::merkle_root;
    use crate::db_operations::DbOperations;
    use crate::storage::SledPool;
    use tempfile::TempDir;

    async fn fresh_index() -> (TempDir, DbOperations) {
        let tmp = TempDir::new().unwrap();
        let pool = Arc::new(SledPool::new(tmp.path().to_path_buf()));
        let ops = DbOperations::from_sled(pool).await.unwrap();
        (tmp, ops)
    }

    fn mref(mol: &str, atom: &str, key: Option<&str>, written_at: u64) -> MoleculeRef {
        MoleculeRef {
            molecule_uuid: mol.to_string(),
            atom_uuid: atom.to_string(),
            key: key.map(str::to_string),
            written_at,
        }
    }

    #[tokio::test]
    async fn forward_round_trip() {
        let (_tmp, ops) = fresh_index().await;
        let idx = ops.lineage();
        let sources = vec![
            mref("mol-a", "atom-a", None, 1),
            mref("mol-b", "atom-b", Some("k"), 2),
        ];

        idx.insert("derived-1", &sources).await.unwrap();

        let got = idx.get_forward("derived-1").await.unwrap();
        assert_eq!(got, Some(sources));
        assert_eq!(idx.get_forward("not-there").await.unwrap(), None);
    }

    #[tokio::test]
    async fn reverse_round_trip() {
        let (_tmp, ops) = fresh_index().await;
        let idx = ops.lineage();
        let s1 = mref("mol-a", "atom-a", None, 10);
        let s2 = mref("mol-b", "atom-b", Some("k"), 20);

        idx.insert("derived-1", &[s1.clone(), s2.clone()])
            .await
            .unwrap();

        assert_eq!(
            idx.get_reverse(&s1).await.unwrap(),
            vec!["derived-1".to_string()],
        );
        assert_eq!(
            idx.get_reverse(&s2).await.unwrap(),
            vec!["derived-1".to_string()],
        );
        // Unknown source returns empty, not an error.
        let unknown = mref("other", "atom", None, 0);
        assert_eq!(
            idx.get_reverse(&unknown).await.unwrap(),
            Vec::<String>::new()
        );
    }

    #[tokio::test]
    async fn multiple_derivatives_per_source_returns_stable_order() {
        let (_tmp, ops) = fresh_index().await;
        let idx = ops.lineage();
        let shared = mref("shared-mol", "shared-atom", None, 100);
        let only_first = mref("only-a", "atom-a", None, 1);
        let only_second = mref("only-b", "atom-b", None, 2);

        // Insert out of alphabetical order to force the sort-on-write path.
        idx.insert("derived-z", &[shared.clone(), only_second])
            .await
            .unwrap();
        idx.insert("derived-a", &[shared.clone(), only_first])
            .await
            .unwrap();
        idx.insert("derived-m", std::slice::from_ref(&shared))
            .await
            .unwrap();

        let derivatives = idx.get_reverse(&shared).await.unwrap();
        assert_eq!(
            derivatives,
            vec![
                "derived-a".to_string(),
                "derived-m".to_string(),
                "derived-z".to_string(),
            ],
            "reverse index must return derivatives in stable sorted order",
        );

        // Idempotent re-insert: the same derived_uuid does not duplicate.
        idx.insert("derived-m", std::slice::from_ref(&shared))
            .await
            .unwrap();
        let derivatives = idx.get_reverse(&shared).await.unwrap();
        assert_eq!(derivatives.len(), 3);
    }

    #[tokio::test]
    async fn remove_cascades_forward_and_reverse() {
        let (_tmp, ops) = fresh_index().await;
        let idx = ops.lineage();
        let shared = mref("shared-mol", "shared-atom", None, 100);
        let only_removed = mref("only-removed", "atom", None, 50);
        let kept = mref("kept-mol", "kept-atom", None, 200);

        idx.insert("derived-gone", &[shared.clone(), only_removed.clone()])
            .await
            .unwrap();
        idx.insert("derived-kept", &[shared.clone(), kept.clone()])
            .await
            .unwrap();

        idx.remove("derived-gone").await.unwrap();

        // Forward entry for the removed derived molecule is gone.
        assert_eq!(idx.get_forward("derived-gone").await.unwrap(), None);
        // `kept` remains intact.
        assert_eq!(
            idx.get_forward("derived-kept").await.unwrap(),
            Some(vec![shared.clone(), kept.clone()]),
        );

        // Reverse entry for the shared source now lists only `derived-kept`.
        assert_eq!(
            idx.get_reverse(&shared).await.unwrap(),
            vec!["derived-kept".to_string()],
        );
        // Reverse entry for the exclusively-referenced source is fully dropped.
        assert_eq!(
            idx.get_reverse(&only_removed).await.unwrap(),
            Vec::<String>::new(),
        );
        // Unrelated reverse entry untouched.
        assert_eq!(
            idx.get_reverse(&kept).await.unwrap(),
            vec!["derived-kept".to_string()],
        );

        // Removing something that doesn't exist is a no-op, not an error.
        idx.remove("never-existed").await.unwrap();
    }

    #[tokio::test]
    async fn verify_merkle_consistency_accepts_matching_root_and_rejects_mutation() {
        let sources = vec![
            mref("mol-a", "atom-a", None, 1),
            mref("mol-b", "atom-b", Some("k"), 2),
            mref("mol-c", "atom-c", Some("k2"), 3),
        ];
        let leaves: Vec<Vec<u8>> = sources.iter().map(MoleculeRef::canonical_bytes).collect();
        let expected_hex: String = merkle_root(&leaves)
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        assert!(LineageIndex::verify_merkle_consistency(
            &sources,
            &expected_hex
        ));

        let mut mutated = sources.clone();
        mutated[1].written_at += 1;
        assert!(!LineageIndex::verify_merkle_consistency(
            &mutated,
            &expected_hex
        ));

        // Also catches reordering: root is order-sensitive.
        let mut reordered = sources.clone();
        reordered.swap(0, 2);
        assert!(!LineageIndex::verify_merkle_consistency(
            &reordered,
            &expected_hex
        ));

        // Wrong hex — short, empty, or garbage — rejects.
        assert!(!LineageIndex::verify_merkle_consistency(&sources, ""));
        assert!(!LineageIndex::verify_merkle_consistency(
            &sources, "deadbeef"
        ));
    }
}
