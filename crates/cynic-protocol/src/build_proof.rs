use serde::{Deserialize, Serialize};

pub const BUILD_PROOF_CARD_SCHEMA_V1: &str = "cynic.build_proof_card.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildProofCard {
    pub schema: &'static str,
    pub card_id: String,
    pub epoch: u64,
    pub timestamp: String,
    pub commit_hash: String,
    pub crates_affected: Vec<String>,
    pub lines_inserted: usize,
    pub lines_deleted: usize,
    pub audit_v2_status: String,
    pub q_score_bps: Option<u32>,
    pub proof_digest: String,
    pub public_summary: String,
}
