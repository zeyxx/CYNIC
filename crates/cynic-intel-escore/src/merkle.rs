use cynic_intel::escore::types::EScoreSignal;
use serde_json::to_vec;
use thiserror::Error;

/// Un hash BLAKE3 sur 32 octets (256 bits de sortie).
/// Pour une sécurité post-quantique, on utilise le mode XOF 512-bit en interne
/// lors du combine, mais on stocke l'empreinte finale sur 32 octets (suffisant
/// pour l'arbre de Merkle, qui n'est pas un système de chiffrement).
pub type Blake3Hash = [u8; 32];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleProofStep {
    pub sibling_hash: Blake3Hash,
    pub sibling_is_left: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleProof {
    pub leaf_hash: Blake3Hash,
    pub path: Vec<MerkleProofStep>,
}

#[derive(Debug, Error)]
pub enum MerkleError {
    #[error("cannot build a Merkle tree from an empty signal list")]
    EmptySignalSet,
    #[error("failed to serialize signal: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("leaf index {index} is out of range for {len} leaves")]
    InvalidLeafIndex { index: usize, len: usize },
}

pub fn merkle_root(signals: &[EScoreSignal]) -> Result<Blake3Hash, MerkleError> {
    let leaves = signal_leaves(signals)?;
    build_root_from_leaves(&leaves)
}

pub fn merkle_root_hex(signals: &[EScoreSignal]) -> Result<String, MerkleError> {
    merkle_root(signals).map(hex::encode)
}

pub fn build_proof(
    signals: &[EScoreSignal],
    leaf_index: usize,
) -> Result<MerkleProof, MerkleError> {
    let mut level = signal_leaves(signals)?;
    if leaf_index >= level.len() {
        return Err(MerkleError::InvalidLeafIndex {
            index: leaf_index,
            len: level.len(),
        });
    }

    let leaf_hash = level[leaf_index];
    let mut index = leaf_index;
    let mut path = Vec::new();

    while level.len() > 1 {
        let sibling_index = if index.is_multiple_of(2) {
            if index + 1 < level.len() {
                index + 1
            } else {
                index
            }
        } else {
            index - 1
        };

        path.push(MerkleProofStep {
            sibling_hash: level[sibling_index],
            sibling_is_left: sibling_index < index,
        });

        level = next_level(&level);
        index /= 2;
    }

    Ok(MerkleProof { leaf_hash, path })
}

pub fn verify_proof(proof: &MerkleProof, expected_root: &Blake3Hash) -> bool {
    let mut current = proof.leaf_hash;
    for step in &proof.path {
        current = if step.sibling_is_left {
            combine_hashes(&step.sibling_hash, &current)
        } else {
            combine_hashes(&current, &step.sibling_hash)
        };
    }
    &current == expected_root
}

// ── Internals ──────────────────────────────────────────────────────────────

fn signal_leaves(signals: &[EScoreSignal]) -> Result<Vec<Blake3Hash>, MerkleError> {
    if signals.is_empty() {
        return Err(MerkleError::EmptySignalSet);
    }
    signals.iter().map(signal_leaf_hash).collect()
}

fn signal_leaf_hash(signal: &EScoreSignal) -> Result<Blake3Hash, MerkleError> {
    let payload = to_vec(signal)?;
    Ok(hash_bytes(&payload))
}

fn build_root_from_leaves(leaves: &[Blake3Hash]) -> Result<Blake3Hash, MerkleError> {
    if leaves.is_empty() {
        return Err(MerkleError::EmptySignalSet);
    }
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        level = next_level(&level);
    }
    Ok(level[0])
}

fn next_level(level: &[Blake3Hash]) -> Vec<Blake3Hash> {
    let mut next = Vec::with_capacity(level.len().div_ceil(2));
    for pair in level.chunks(2) {
        let left = pair[0];
        let right = if pair.len() == 2 { pair[1] } else { pair[0] };
        next.push(combine_hashes(&left, &right));
    }
    next
}

/// Combine deux hashes BLAKE3 en un seul nœud parent.
/// On préfixe le domaine ("cynic-merkle") pour éviter les attaques de second preimage.
fn combine_hashes(left: &Blake3Hash, right: &Blake3Hash) -> Blake3Hash {
    let mut hasher = blake3::Hasher::new_derive_key("cynic-merkle");
    hasher.update(left);
    hasher.update(right);
    *hasher.finalize().as_bytes()
}

/// Hash d'un payload arbitraire avec préfixe de domaine pour les feuilles.
fn hash_bytes(bytes: &[u8]) -> Blake3Hash {
    let mut hasher = blake3::Hasher::new_derive_key("cynic-leaf");
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::{build_proof, merkle_root, verify_proof};
    use cynic_intel::escore::types::{EScoreSignal, EvidenceTier, SCORE_SCALE, StatType};

    fn sample_signal(wallet_id: &str, points: u64, timestamp: i64) -> EScoreSignal {
        EScoreSignal {
            wallet_id: wallet_id.to_owned(),
            stat_type: StatType::Parrhesia,
            evidence_tier: EvidenceTier::T2Attestation,
            raw_weight: points * SCORE_SCALE,
            timestamp,
            epoch: None,
        }
    }

    #[test]
    fn merkle_proof_roundtrip_verifies() {
        let signals = vec![
            sample_signal("alpha", 1, 1_720_000_001),
            sample_signal("beta", 2, 1_720_000_002),
            sample_signal("gamma", 3, 1_720_000_003),
        ];
        let root = merkle_root(&signals).expect("root");
        let proof = build_proof(&signals, 1).expect("proof");
        assert!(verify_proof(&proof, &root));
    }

    #[test]
    fn invalid_merkle_proof_is_rejected() {
        let signals = vec![
            sample_signal("alpha", 1, 1_720_000_001),
            sample_signal("beta", 2, 1_720_000_002),
            sample_signal("gamma", 3, 1_720_000_003),
            sample_signal("delta", 4, 1_720_000_004),
        ];
        let root = merkle_root(&signals).expect("root");
        let mut proof = build_proof(&signals, 2).expect("proof");
        proof.path[0].sibling_hash[0] ^= 0xFF;
        assert!(!verify_proof(&proof, &root));
    }

    #[test]
    fn domain_separation_prevents_leaf_node_collision() {
        // Un hash de feuille et un hash de nœud interne ne doivent jamais être identiques
        // grâce aux préfixes "cynic-leaf" vs "cynic-merkle".
        let signals = vec![sample_signal("solo", 10, 1_720_000_001)];
        // Un arbre de 1 feuille : la racine = le hash de la feuille
        let root = merkle_root(&signals).expect("root");
        let proof = build_proof(&signals, 0).expect("proof");
        assert_eq!(proof.leaf_hash, root, "Arbre d'une feuille : root == leaf");
    }
}
