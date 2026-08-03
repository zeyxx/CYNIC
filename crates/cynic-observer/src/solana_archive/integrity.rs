use super::SolanaObservation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityStatus {
    Valid,
    Invalid,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrityCheck {
    pub signature: String,
    pub expected_digest: String,
    pub actual_digest: Option<String>,
    pub status: IntegrityStatus,
}

pub fn verify_payload(observation: &SolanaObservation, payload: &[u8]) -> IntegrityCheck {
    let actual = format!("blake3:{}", blake3::hash(payload).to_hex());
    let status = if observation.raw_payload_digest.is_empty() {
        IntegrityStatus::Unknown
    } else if observation.raw_payload_digest == actual {
        IntegrityStatus::Valid
    } else {
        IntegrityStatus::Invalid
    };

    IntegrityCheck {
        signature: observation.signature.clone(),
        expected_digest: observation.raw_payload_digest.clone(),
        actual_digest: Some(actual),
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{Commitment, DecodeStatus, ProviderKind};

    fn observation(payload: &[u8]) -> SolanaObservation {
        SolanaObservation {
            cluster: "mainnet".to_string(),
            signature: "sig".to_string(),
            slot: Some(1),
            commitment: Commitment::Finalized,
            provider: ProviderKind::LegacyCompatibility,
            retrieved_at_unix_ms: None,
            raw_payload_digest: format!("blake3:{}", blake3::hash(payload).to_hex()),
            decoder_profile: None,
            decode_status: DecodeStatus::RawOnly,
        }
    }

    #[test]
    fn matching_payload_is_valid() {
        let payload = br#"{"signature":"sig"}"#;
        assert_eq!(
            verify_payload(&observation(payload), payload).status,
            IntegrityStatus::Valid
        );
    }

    #[test]
    fn changed_payload_is_invalid() {
        let original = br#"{"signature":"sig"}"#;
        let check = verify_payload(&observation(original), br#"{"signature":"other"}"#);
        assert_eq!(check.status, IntegrityStatus::Invalid);
    }
}
