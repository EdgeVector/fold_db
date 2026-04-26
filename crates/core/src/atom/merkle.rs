//! Merkle tree utility for source-molecule sets.
//!
//! This is the leaf-and-root primitive behind `Provenance::Derived`'s
//! `sources_merkle_root`. Leaves are `MoleculeRef::canonical_bytes()`
//! outputs; the root pins the set of source molecules that flowed into a
//! derived molecule without inlining the full list.
//!
//! **Canonical forever.** Hash function is SHA-256. Odd layers duplicate
//! the last node (Bitcoin-style). Changing either choice changes the
//! content address of every derived molecule — gated by bumping
//! `Provenance::Derived::encoding_version`.
//!
//! Step 2 of the 6-PR arc in `gbrain get projects/molecule-provenance-dag`.
//! No production call sites exist yet; only tests in this module call these
//! functions. Project 2 (`view-compute-as-mutations`) wires them in.

use sha2::{Digest, Sha256};

/// Errors returnable from Merkle-proof construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MerkleError {
    /// `index` was not a valid position within `leaves` (0..len).
    IndexOutOfRange { index: usize, len: usize },
    /// Caller asked for a proof against an empty leaf set.
    EmptyLeaves,
}

impl std::fmt::Display for MerkleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexOutOfRange { index, len } => {
                write!(f, "merkle proof index {} out of range (len {})", index, len)
            }
            Self::EmptyLeaves => write!(f, "merkle proof requested over empty leaf set"),
        }
    }
}

impl std::error::Error for MerkleError {}

/// Hash a single byte slice with SHA-256 into a fixed 32-byte array.
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Hash the concatenation of two 32-byte nodes.
fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Build a Merkle root over `leaves`.
///
/// - Empty input returns `sha256("")` (the SHA-256 of the empty byte
///   string: `e3b0c442…b7852b855`). This is an intentional sentinel for
///   "no sources" — a derived molecule with zero inputs still has a
///   well-defined root.
/// - Odd layers duplicate the last node before pairing (Bitcoin-style).
///
/// Both choices are pinned forever by the known-vector tests; a change
/// breaks every previously-stored `sources_merkle_root`.
#[must_use]
pub fn merkle_root(leaves: &[Vec<u8>]) -> [u8; 32] {
    if leaves.is_empty() {
        return sha256(b"");
    }

    let mut layer: Vec<[u8; 32]> = leaves.iter().map(|leaf| sha256(leaf)).collect();

    while layer.len() > 1 {
        if !layer.len().is_multiple_of(2) {
            let last = *layer.last().expect("non-empty layer");
            layer.push(last);
        }
        layer = layer
            .chunks_exact(2)
            .map(|pair| hash_pair(&pair[0], &pair[1]))
            .collect();
    }

    layer[0]
}

/// Build an inclusion proof for the leaf at `index` within `leaves`.
///
/// Returns the sibling hashes along the path from leaf to root. The proof
/// is the minimal set of co-path nodes needed by `verify_merkle_proof`.
/// Odd layers duplicate the last node before pairing, so the sibling of a
/// self-paired last node is a copy of itself.
///
/// # Errors
///
/// - `MerkleError::EmptyLeaves` if `leaves` is empty.
/// - `MerkleError::IndexOutOfRange` if `index >= leaves.len()`.
pub fn merkle_proof(leaves: &[Vec<u8>], index: usize) -> Result<Vec<[u8; 32]>, MerkleError> {
    if leaves.is_empty() {
        return Err(MerkleError::EmptyLeaves);
    }
    if index >= leaves.len() {
        return Err(MerkleError::IndexOutOfRange {
            index,
            len: leaves.len(),
        });
    }

    let mut layer: Vec<[u8; 32]> = leaves.iter().map(|leaf| sha256(leaf)).collect();
    let mut idx = index;
    let mut proof: Vec<[u8; 32]> = Vec::new();

    while layer.len() > 1 {
        if !layer.len().is_multiple_of(2) {
            let last = *layer.last().expect("non-empty layer");
            layer.push(last);
        }
        let sibling_idx = if idx.is_multiple_of(2) {
            idx + 1
        } else {
            idx - 1
        };
        proof.push(layer[sibling_idx]);
        layer = layer
            .chunks_exact(2)
            .map(|pair| hash_pair(&pair[0], &pair[1]))
            .collect();
        idx /= 2;
    }

    Ok(proof)
}

/// Verify that `leaf` at `index` with `proof` hashes to `expected_root`.
///
/// Walks the proof from leaf to root, picking left/right sibling ordering
/// from the bit at each level of `index`. Returns `true` iff the computed
/// root matches `expected_root`.
#[must_use]
pub fn verify_merkle_proof(
    leaf: &[u8],
    index: usize,
    proof: &[[u8; 32]],
    expected_root: &[u8; 32],
) -> bool {
    let mut node = sha256(leaf);
    let mut idx = index;
    for sibling in proof {
        node = if idx.is_multiple_of(2) {
            hash_pair(&node, sibling)
        } else {
            hash_pair(sibling, &node)
        };
        idx /= 2;
    }
    &node == expected_root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::MoleculeRef;

    fn hex(bytes: &[u8; 32]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    fn leaves(items: &[&[u8]]) -> Vec<Vec<u8>> {
        items.iter().map(|s| s.to_vec()).collect()
    }

    // Known-vector tests. These hex strings are load-bearing forever — they
    // pin SHA-256 + Bitcoin-style odd duplication as the canonical choice.
    // A change to either means every previously-stored
    // `sources_merkle_root` becomes unverifiable.

    #[test]
    fn known_vector_empty() {
        // sha256("") — defined sentinel for "no sources".
        let root = merkle_root(&[]);
        assert_eq!(
            hex(&root),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        );
    }

    #[test]
    fn known_vector_one_leaf() {
        let root = merkle_root(&leaves(&[b"leaf0"]));
        assert_eq!(
            hex(&root),
            "4d5a9584d985e8fb44015a8affa9b76f1ff16f65e61df7156d8e8159e1448978",
        );
    }

    #[test]
    fn known_vector_two_leaves() {
        let root = merkle_root(&leaves(&[b"leaf0", b"leaf1"]));
        assert_eq!(
            hex(&root),
            "884ff14f19d1564614ab3184d7bdc35a1a9ff90d36ac962b05a81aeb56027c22",
        );
    }

    #[test]
    fn known_vector_three_leaves_odd_duplication() {
        // Pins the Bitcoin-style odd-duplication choice. Three leaves →
        // layer 0 duplicates the last: [h0, h1, h2, h2] → layer 1
        // [hash(h0||h1), hash(h2||h2)] → root.
        let root = merkle_root(&leaves(&[b"leaf0", b"leaf1", b"leaf2"]));
        assert_eq!(
            hex(&root),
            "4cfe0e066467f4ba247406e44f608011104dcbfa537bb21752a1f3a48b04da0b",
        );
    }

    #[test]
    fn known_vector_four_leaves() {
        let root = merkle_root(&leaves(&[b"leaf0", b"leaf1", b"leaf2", b"leaf3"]));
        assert_eq!(
            hex(&root),
            "8910150e02a7fe57232749c31f7cfd48a8439011e34227c6b7e3eb7d98440ee6",
        );
    }

    #[test]
    fn round_trip_proof_verifies_for_every_index_at_every_size() {
        for n in [1usize, 2, 3, 4, 7, 8] {
            let leaves_vec: Vec<Vec<u8>> =
                (0..n).map(|i| format!("leaf{}", i).into_bytes()).collect();
            let root = merkle_root(&leaves_vec);
            for i in 0..n {
                let proof = merkle_proof(&leaves_vec, i)
                    .unwrap_or_else(|e| panic!("proof for n={} i={} failed: {}", n, i, e));
                assert!(
                    verify_merkle_proof(&leaves_vec[i], i, &proof, &root),
                    "verify failed for n={} i={}",
                    n,
                    i,
                );
            }
        }
    }

    #[test]
    fn flipping_any_bit_in_any_leaf_changes_the_root() {
        let base_leaves = leaves(&[b"leaf0", b"leaf1", b"leaf2", b"leaf3"]);
        let base_root = merkle_root(&base_leaves);

        for leaf_i in 0..base_leaves.len() {
            for byte_i in 0..base_leaves[leaf_i].len() {
                for bit in 0..8u8 {
                    let mut mutated = base_leaves.clone();
                    mutated[leaf_i][byte_i] ^= 1 << bit;
                    let mutated_root = merkle_root(&mutated);
                    assert_ne!(
                        mutated_root, base_root,
                        "flipping leaf {} byte {} bit {} did not change root",
                        leaf_i, byte_i, bit,
                    );
                }
            }
        }
    }

    #[test]
    fn verify_rejects_wrong_index() {
        let leaves_vec = leaves(&[b"leaf0", b"leaf1", b"leaf2", b"leaf3"]);
        let root = merkle_root(&leaves_vec);
        let proof = merkle_proof(&leaves_vec, 1).expect("proof for index 1");

        // Passing the same leaf + proof but a different index must fail:
        // the left/right ordering at each level of verification comes from
        // index bits, so swapping an index flips a pair and corrupts the
        // derived root.
        for wrong in [0usize, 2, 3] {
            assert!(
                !verify_merkle_proof(&leaves_vec[1], wrong, &proof, &root),
                "verify wrongly accepted leaf 1 at index {}",
                wrong,
            );
        }
    }

    #[test]
    fn proof_index_out_of_range_errors() {
        let leaves_vec = leaves(&[b"leaf0", b"leaf1", b"leaf2"]);
        let err = merkle_proof(&leaves_vec, 3).expect_err("index 3 is out of range for len 3");
        assert_eq!(err, MerkleError::IndexOutOfRange { index: 3, len: 3 });
    }

    #[test]
    fn proof_over_empty_leaves_errors() {
        let err = merkle_proof(&[], 0).expect_err("empty leaves must error");
        assert_eq!(err, MerkleError::EmptyLeaves);
    }

    #[test]
    fn integration_with_molecule_ref_canonical_bytes() {
        // Actual use case: source-molecule references as Merkle leaves.
        let refs = [
            MoleculeRef {
                molecule_uuid: "mol-a".to_string(),
                atom_uuid: "atom-a".to_string(),
                key: None,
                written_at: 1000,
            },
            MoleculeRef {
                molecule_uuid: "mol-b".to_string(),
                atom_uuid: "atom-b".to_string(),
                key: Some("k1".to_string()),
                written_at: 2000,
            },
            MoleculeRef {
                molecule_uuid: "mol-c".to_string(),
                atom_uuid: "atom-c".to_string(),
                key: Some("k2".to_string()),
                written_at: 3000,
            },
        ];
        let leaves_vec: Vec<Vec<u8>> = refs.iter().map(MoleculeRef::canonical_bytes).collect();
        let root = merkle_root(&leaves_vec);

        for (i, mref) in refs.iter().enumerate() {
            let proof = merkle_proof(&leaves_vec, i).expect("proof");
            assert!(
                verify_merkle_proof(&mref.canonical_bytes(), i, &proof, &root),
                "MoleculeRef proof failed at index {}",
                i,
            );
        }
    }
}
