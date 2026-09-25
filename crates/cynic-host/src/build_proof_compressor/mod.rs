<<<<<<< HEAD
//! Build proof compressor — re-exported from `cynic-build` per Foundation §481 (SoC).

pub use cynic_build::compressor::*;
=======
use chrono::Utc;
use cynic_protocol::build_proof::{BuildProofCard, BUILD_PROOF_CARD_SCHEMA_V1};
use sha2::{Digest, Sha256};

pub fn compress_build_proof(
    epoch: u64,
    commit_hash: &str,
    crates_affected: Vec<String>,
    lines_inserted: usize,
    lines_deleted: usize,
    audit_v2_status: &str,
    q_score_bps: Option<u32>,
    raw_summary: &str,
) -> BuildProofCard {
    let timestamp = Utc::now().to_rfc3339();
    let sanitized_summary = sanitize_public_text(raw_summary);

    let mut hasher = Sha256::new();
    hasher.update(epoch.to_be_bytes());
    hasher.update(commit_hash.as_bytes());
    hasher.update(timestamp.as_bytes());
    hasher.update(lines_inserted.to_be_bytes());
    hasher.update(lines_deleted.to_be_bytes());
    hasher.update(audit_v2_status.as_bytes());
    for c in &crates_affected {
        hasher.update(c.as_bytes());
    }
    let proof_digest = hex::encode(hasher.finalize());

    let card_id = format!("bpc-{}-{}", &commit_hash[..commit_hash.len().min(8)], &proof_digest[..8]);

    BuildProofCard {
        schema: BUILD_PROOF_CARD_SCHEMA_V1,
        card_id,
        epoch,
        timestamp,
        commit_hash: commit_hash.to_string(),
        crates_affected,
        lines_inserted,
        lines_deleted,
        audit_v2_status: audit_v2_status.to_string(),
        q_score_bps,
        proof_digest,
        public_summary: sanitized_summary,
    }
}

fn sanitize_public_text(text: &str) -> String {
    let mut clean = text.to_string();
    // Neutralise les chemins absolus pour protéger la vie privée de l'opérateur
    clean = clean.replace("C:\\Users\\", "/users/");
    clean = clean.replace("C:/Users/", "/users/");
    clean
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_build_proof_generation() {
        let card = compress_build_proof(
            42,
            "c308fef123456789",
            vec!["cynic-protocol".to_string(), "cynic-host".to_string()],
            945,
            65,
            "ok",
            Some(6180),
            "Implemented host hardening and specs under C:/Users/operator",
        );

        assert_eq!(card.schema, BUILD_PROOF_CARD_SCHEMA_V1);
        assert_eq!(card.epoch, 42);
        assert_eq!(card.commit_hash, "c308fef123456789");
        assert_eq!(card.lines_inserted, 945);
        assert_eq!(card.lines_deleted, 65);
        assert_eq!(card.proof_digest.len(), 64);
        assert!(!card.public_summary.contains("C:/Users/"));
    }
}
>>>>>>> github/main
