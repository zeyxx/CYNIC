use serde::{Deserialize, Serialize};

use crate::runtime_seed::RuntimeArtifactSource;

pub const ARTIFACT_TRANSFER_RECEIPT_SCHEMA: &str = "cynic.artifact.transfer_receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactTransferReceipt {
    pub schema: String,
    pub status: String,
    pub source: RuntimeArtifactSource,
    pub destination: String,
    pub expected_digest: Option<String>,
    pub observed_digest: String,
    pub bytes_written: u64,
    pub mutates: bool,
    pub evidence_ref: String,
    pub blockers: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_serialization() {
        let receipt = ArtifactTransferReceipt {
            schema: ARTIFACT_TRANSFER_RECEIPT_SCHEMA.to_string(),
            status: "proved".to_string(),
            source: RuntimeArtifactSource {
                transport: crate::runtime_seed::RuntimeArtifactTransportKind::LocalFile,
                locator: "/tmp/src".to_string(),
            },
            destination: "/tmp/dst".to_string(),
            expected_digest: Some("sha256:abc".to_string()),
            observed_digest: "sha256:def".to_string(),
            bytes_written: 42,
            mutates: true,
            evidence_ref: "sha256:def".to_string(),
            blockers: Vec::new(),
        };

        let json = serde_json::to_string(&receipt).expect("serialize");
        let back: ArtifactTransferReceipt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(receipt, back);
    }
}
