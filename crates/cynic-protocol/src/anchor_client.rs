use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ANCHOR_CLIENT_SCHEMA_V1: &str = "cynic.anchor_client.v1";

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

pub fn derive_pda_digest(seeds: &[&[u8]], program_id: &str) -> String {
    let mut hasher = Sha256::new();
    for seed in seeds {
        hasher.update(seed);
    }
    hasher.update(program_id.as_bytes());
    hex::encode(hasher.finalize())
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

    let pda = derive_pda_digest(&[b"cynic_poh", node_authority.as_bytes(), &epoch.to_le_bytes()], program_id);

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
            [1u8; 32],
            [2u8; 32],
        );
        assert_eq!(ix.instruction_name, "record_poh_checkpoint");
        assert_eq!(ix.accounts.len(), 3);
        assert!(ix.data_hex.len() > 16);
    }
}
