use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ANCHOR_CLIENT_SCHEMA_V1: &str = "cynic.anchor_client.v1";
pub const POH_CHECKPOINT_BATCH_SCHEMA: &str = "cynic.poh_checkpoint_batch.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountMetaRef {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorInstructionPayload {
    pub schema: &'static str,
    pub program_id: String,
    pub accounts: Vec<AccountMetaRef>,
    pub data_hex: String,
    pub instruction_name: String,
}

/// Batch d'agrégation PoH compressant N événements offchain en 1 transaction Solana onchain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoHCheckpointBatch {
    pub schema: &'static str,
    pub epoch: u64,
    pub node_authority: String,
    pub merkle_root_hex: String,
    pub round_digest_hex: String,
    pub event_count: usize,
    pub instruction: AnchorInstructionPayload,
}

pub fn derive_pda_digest(seeds: &[&[u8]], program_id: &str) -> String {
    let mut hasher = Sha256::new();
    for seed in seeds {
        hasher.update(seed);
    }
    hasher.update(program_id.as_bytes());
    hex::encode(hasher.finalize())
}

/// Calcule la racine de Merkle déterministe ($O(N)$) avec séparation de domaine SHA-256.
pub fn compute_merkle_root_from_digests(digests: &[String]) -> [u8; 32] {
    if digests.is_empty() {
        let mut hasher = Sha256::new();
        hasher.update(b"cynic-poh-empty");
        return hasher.finalize().into();
    }

    // 1. Feuilles de Merkle avec préfixe de domaine "cynic-poh-leaf:"
    let mut current_level: Vec<[u8; 32]> = digests
        .iter()
        .map(|digest| {
            let mut hasher = Sha256::new();
            hasher.update(b"cynic-poh-leaf:");
            hasher.update(digest.as_bytes());
            hasher.finalize().into()
        })
        .collect();

    // 2. Réduction pyramidale avec préfixe de nœud interne "cynic-poh-node:"
    while current_level.len() > 1 {
        let mut next_level = Vec::with_capacity((current_level.len() + 1) / 2);
        for chunk in current_level.chunks(2) {
            let mut hasher = Sha256::new();
            hasher.update(b"cynic-poh-node:");
            hasher.update(&chunk[0]);
            if chunk.len() == 2 {
                hasher.update(&chunk[1]);
            } else {
                // Si nombre impair, duplication de la dernière feuille
                hasher.update(&chunk[0]);
            }
            next_level.push(hasher.finalize().into());
        }
        current_level = next_level;
    }

    current_level[0]
}

/// Assemble un batch complet de checkpointing PoH pour le programme Solana cynic-anchor.
pub fn build_poh_checkpoint_from_digests(
    program_id: &str,
    node_authority: &str,
    epoch: u64,
    digests: &[String],
    timestamp_ms: u128,
) -> PoHCheckpointBatch {
    let merkle_root = compute_merkle_root_from_digests(digests);

    // Calcul du digest de round : SHA-256(epoch | node_authority | merkle_root | timestamp)
    let mut round_hasher = Sha256::new();
    round_hasher.update(b"cynic-poh-round:");
    round_hasher.update(&epoch.to_le_bytes());
    round_hasher.update(node_authority.as_bytes());
    round_hasher.update(&merkle_root);
    round_hasher.update(&timestamp_ms.to_le_bytes());
    let round_digest: [u8; 32] = round_hasher.finalize().into();

    let instruction =
        build_record_poh_ix(program_id, node_authority, epoch, merkle_root, round_digest);

    PoHCheckpointBatch {
        schema: POH_CHECKPOINT_BATCH_SCHEMA,
        epoch,
        node_authority: node_authority.to_string(),
        merkle_root_hex: hex::encode(merkle_root),
        round_digest_hex: hex::encode(round_digest),
        event_count: digests.len(),
        instruction,
    }
}

pub fn build_record_poh_ix(
    program_id: &str,
    node_authority: &str,
    epoch: u64,
    merkle_root: [u8; 32],
    round_digest: [u8; 32],
) -> AnchorInstructionPayload {
    let mut data = Vec::new();
    // Anchor 8-byte discriminator for record_poh_checkpoint: sha256("global:record_poh_checkpoint")[..8]
    let mut hasher = Sha256::new();
    hasher.update(b"global:record_poh_checkpoint");
    let disc = &hasher.finalize()[..8];
    data.extend_from_slice(disc);
    data.extend_from_slice(&epoch.to_le_bytes());
    data.extend_from_slice(&merkle_root);
    data.extend_from_slice(&round_digest);

    let pda = derive_pda_digest(
        &[
            b"cynic_poh",
            node_authority.as_bytes(),
            &epoch.to_le_bytes(),
        ],
        program_id,
    );

    AnchorInstructionPayload {
        schema: ANCHOR_CLIENT_SCHEMA_V1,
        program_id: program_id.to_string(),
        accounts: vec![
            AccountMetaRef {
                pubkey: pda,
                is_signer: false,
                is_writable: true,
            },
            AccountMetaRef {
                pubkey: node_authority.to_string(),
                is_signer: true,
                is_writable: true,
            },
            AccountMetaRef {
                pubkey: "11111111111111111111111111111111".to_string(),
                is_signer: false,
                is_writable: false,
            },
        ],
        data_hex: hex::encode(data),
        instruction_name: "record_poh_checkpoint".to_string(),
    }
}

pub fn build_create_futarchy_market_ix(
    program_id: &str,
    creator: &str,
    proposal_hash: [u8; 32],
    initial_liquidity_units: u64,
    min_conviction_threshold_bps: u32,
) -> AnchorInstructionPayload {
    let mut data = Vec::new();
    let mut hasher = Sha256::new();
    hasher.update(b"global:create_futarchy_market");
    let disc = &hasher.finalize()[..8];
    data.extend_from_slice(disc);
    data.extend_from_slice(&proposal_hash);
    data.extend_from_slice(&initial_liquidity_units.to_le_bytes());
    data.extend_from_slice(&min_conviction_threshold_bps.to_le_bytes());

    let pda = derive_pda_digest(&[b"futarchy_market", &proposal_hash], program_id);

    AnchorInstructionPayload {
        schema: ANCHOR_CLIENT_SCHEMA_V1,
        program_id: program_id.to_string(),
        accounts: vec![
            AccountMetaRef {
                pubkey: pda,
                is_signer: false,
                is_writable: true,
            },
            AccountMetaRef {
                pubkey: creator.to_string(),
                is_signer: true,
                is_writable: true,
            },
            AccountMetaRef {
                pubkey: "11111111111111111111111111111111".to_string(),
                is_signer: false,
                is_writable: false,
            },
        ],
        data_hex: hex::encode(data),
        instruction_name: "create_futarchy_market".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_record_poh_ix() {
        let ix = build_record_poh_ix(
            "CYNiC11111111111111111111111111111111111111",
            "NodeAuth1111111111111111111111111111111111",
            42,
            [0xAA; 32],
            [0xBB; 32],
        );
        assert_eq!(ix.instruction_name, "record_poh_checkpoint");
        assert_eq!(ix.accounts.len(), 3);
        assert!(ix.data_hex.len() > 16);
    }

    #[test]
    fn test_poh_checkpoint_from_digests_merkle_compression() {
        let digests = vec![
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
            "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        ];

        let batch = build_poh_checkpoint_from_digests(
            "CYNiC11111111111111111111111111111111111111",
            "NodeAuth1111111111111111111111111111111111",
            100,
            &digests,
            1_787_056_000_000,
        );

        assert_eq!(batch.schema, POH_CHECKPOINT_BATCH_SCHEMA);
        assert_eq!(batch.event_count, 3);
        assert_eq!(batch.merkle_root_hex.len(), 64);
        assert_eq!(batch.round_digest_hex.len(), 64);
        assert_eq!(batch.instruction.instruction_name, "record_poh_checkpoint");
    }
}
