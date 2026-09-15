use chrono::Utc;
use crate::build_proof::{BUILD_PROOF_CARD_SCHEMA_V1, BuildProofCard};
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

    let card_id = format!(
        "bpc-{}-{}",
        &commit_hash[..commit_hash.len().min(8)],
        &proof_digest[..8]
    );

    BuildProofCard {
        schema: BUILD_PROOF_CARD_SCHEMA_V1.to_string(),
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
    // Dynamic user home directory redaction
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() && home != "/" {
            clean = clean.replace(&home, "[REDACTED_PATH]");
        }
    }
    // Standard sensitive system path redactions
    for prefix in &[
        "/home/user",
        "/home/cynic",
        "/var/lib/cynic",
        "/root",
        "/etc/cynic",
    ] {
        clean = clean.replace(prefix, "[REDACTED_PATH]");
    }
    clean
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_build_proof_determinism() {
        let card = compress_build_proof(
            1,
            "4e314810b65679dc6ebda032be15fb92e592f664",
            vec!["cynic-protocol".into(), "cynic-node".into()],
            150,
            20,
            "pass",
            Some(9200),
            "Build executed at /home/user/Bureau/CYNIC",
        );

        assert_eq!(card.schema, BUILD_PROOF_CARD_SCHEMA_V1.to_string());
        assert!(card.card_id.starts_with("bpc-4e314810-"));
        assert!(!card.public_summary.contains("/home/user"));
        assert!(card.public_summary.contains("[REDACTED_PATH]"));
        assert_eq!(card.proof_digest.len(), 64);
    }
}
