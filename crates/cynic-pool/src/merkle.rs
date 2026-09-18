//! Merkle proofs for cross-registry integrity and share verification.

use blake3::Hasher;
use serde::{Deserialize, Serialize};

pub const LEAF_PREFIX: &[u8] = b"cynic-pool-leaf";
pub const NODE_PREFIX: &[u8] = b"cynic-pool-node";

/// Derive a 32-byte key from a domain-separation prefix.
fn derive_key_32(prefix: &[u8]) -> [u8; 32] {
    let hash = blake3::hash(prefix);
    let mut key = [0u8; 32];
    key.copy_from_slice(hash.as_bytes());
    key
}

static LEAF_KEY: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
static NODE_KEY: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();

pub fn hash_leaf(data: &[u8]) -> [u8; 32] {
    let key = LEAF_KEY.get_or_init(|| derive_key_32(LEAF_PREFIX));
    let mut hasher = Hasher::new_keyed(key);
    hasher.update(data);
    hasher.finalize().into()
}

pub fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let key = NODE_KEY.get_or_init(|| derive_key_32(NODE_PREFIX));
    let mut hasher = Hasher::new_keyed(key);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

pub fn pool_merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::new();
        for i in (0..level.len()).step_by(2) {
            let left = &level[i];
            let right = level.get(i + 1).unwrap_or(left);
            next.push(hash_node(left, right));
        }
        level = next;
    }
    level[0]
}

pub fn pool_merkle_proof(leaves: &[[u8; 32]], index: usize) -> Option<Vec<[u8; 32]>> {
    if index >= leaves.len() {
        return None;
    }
    let mut proof = Vec::new();
    let mut level = leaves.to_vec();
    let mut idx = index;
    while level.len() > 1 {
        let mut next = Vec::new();
        for i in (0..level.len()).step_by(2) {
            let left = level[i];
            let right = level.get(i + 1).copied().unwrap_or(left);
            if i == idx {
                proof.push(right);
            } else if i + 1 == idx {
                proof.push(left);
            }
            next.push(hash_node(&left, &right));
        }
        idx /= 2;
        level = next;
    }
    Some(proof)
}

pub fn verify_pool_proof(
    leaf: &[u8; 32],
    root: &[u8; 32],
    proof: &[[u8; 32]],
    index: usize,
) -> bool {
    let mut current = *leaf;
    let mut idx = index;
    for sibling in proof {
        current = if idx % 2 == 0 {
            hash_node(&current, sibling)
        } else {
            hash_node(sibling, &current)
        };
        idx /= 2;
    }
    current == *root
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_hash: String,
    pub root_hash: String,
    pub path: Vec<String>,
    pub index: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_root_empty() {
        let root = pool_merkle_root(&[]);
        assert_eq!(root, [0u8; 32]);
    }

    #[test]
    fn merkle_root_single() {
        let leaf = hash_leaf(b"share1");
        let root = pool_merkle_root(&[leaf]);
        assert_eq!(root, leaf);
    }

    #[test]
    fn merkle_root_multiple() {
        let leaves: Vec<[u8; 32]> = (0..8u32).map(|i| hash_leaf(&i.to_be_bytes())).collect();
        let root = pool_merkle_root(&leaves);
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = pool_merkle_proof(&leaves, i).unwrap();
            assert!(verify_pool_proof(leaf, &root, &proof, i));
        }
    }

    #[test]
    fn merkle_proof_invalid_index() {
        let leaves: Vec<[u8; 32]> = (0..4u32).map(|i| hash_leaf(&i.to_be_bytes())).collect();
        assert!(pool_merkle_proof(&leaves, 10).is_none());
    }
}
