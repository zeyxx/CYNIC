use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A minimal, explicitly legacy-shaped record. It is an input to migration,
/// not a CYNIC authority and not a derived financial fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyObservation {
    pub signature: String,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub confirmation_status: Option<String>,
    pub source_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityError {
    MissingSignature,
    InvalidSignature,
    InvalidSlot,
    InvalidJson(String),
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRejection {
    pub path: PathBuf,
    pub error: CompatibilityError,
}

#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub root: PathBuf,
    pub files_scanned: u64,
    pub observations: Vec<crate::solana_archive::SolanaObservation>,
    pub rejections: Vec<MigrationRejection>,
    pub coverage: crate::solana_archive::CoverageLedger,
}

pub fn from_odysseum_value(value: &Value) -> Result<LegacyObservation, CompatibilityError> {
    let observation = value.get("observation").unwrap_or(value);
    let signature = observation
        .get("signature")
        .and_then(Value::as_str)
        .ok_or(CompatibilityError::MissingSignature)?
        .trim();
    if signature.is_empty() {
        return Err(CompatibilityError::InvalidSignature);
    }

    let slot = observation.get("slot").and_then(Value::as_u64);
    if observation.get("slot").is_some() && slot.is_none() {
        return Err(CompatibilityError::InvalidSlot);
    }

    Ok(LegacyObservation {
        signature: signature.to_string(),
        slot,
        block_time: observation.get("blockTime").and_then(Value::as_i64),
        confirmation_status: observation
            .get("confirmationStatus")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        source_label: "odysseum_compatibility_fixture".to_string(),
    })
}

pub fn migrate_odysseum_json_bytes(
    bytes: &[u8],
) -> Result<crate::solana_archive::SolanaObservation, CompatibilityError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| CompatibilityError::InvalidJson(error.to_string()))?;
    let legacy = from_odysseum_value(&value)?;
    let digest = blake3::hash(bytes).to_hex().to_string();
    Ok(crate::solana_archive::SolanaObservation {
        cluster: "mainnet".to_string(),
        signature: legacy.signature,
        slot: legacy.slot,
        commitment: match legacy.confirmation_status.as_deref() {
            Some("finalized") => crate::solana_archive::Commitment::Finalized,
            Some("confirmed") => crate::solana_archive::Commitment::Confirmed,
            Some("processed") => crate::solana_archive::Commitment::Processed,
            _ => crate::solana_archive::Commitment::Unknown,
        },
        provider: crate::solana_archive::ProviderKind::LegacyCompatibility,
        retrieved_at_unix_ms: None,
        raw_payload_digest: format!("blake3:{digest}"),
        decoder_profile: None,
        decode_status: crate::solana_archive::DecodeStatus::RawOnly,
    })
}

pub fn migrate_odysseum_json_file(
    path: impl AsRef<Path>,
) -> Result<crate::solana_archive::SolanaObservation, CompatibilityError> {
    let bytes = fs::read(path).map_err(|error| CompatibilityError::Io(error.to_string()))?;
    migrate_odysseum_json_bytes(&bytes)
}

pub fn migrate_odysseum_json_directory(
    root: impl AsRef<Path>,
    target: impl Into<String>,
    cluster: impl Into<String>,
) -> io::Result<MigrationReport> {
    let root = root.as_ref().to_path_buf();
    let cluster = cluster.into();
    let mut pending = vec![root.clone()];
    let mut files_scanned = 0;
    let mut observations = Vec::new();
    let mut rejections = Vec::new();
    let mut coverage = crate::solana_archive::CoverageLedger::new(target, cluster.clone());

    while let Some(path) = pending.pop() {
        if fs::metadata(&path)?.is_dir() {
            for entry in fs::read_dir(&path)? {
                pending.push(entry?.path());
            }
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }

        files_scanned += 1;
        match migrate_odysseum_json_file(&path) {
            Ok(mut observation) => {
                observation.cluster = cluster.clone();
                coverage.record_observation(observation.signature.clone());
                observations.push(observation);
            }
            Err(error) => rejections.push(MigrationRejection { path, error }),
        }
    }

    if rejections.is_empty() {
        coverage.mark_exhausted();
    } else {
        coverage.record_gap(
            "legacy_events_rejected",
            format!("{} JSON files could not migrate", rejections.len()),
        );
    }

    Ok(MigrationReport {
        root,
        files_scanned,
        observations,
        rejections,
        coverage,
    })
}

pub fn read_json_lines(
    path: impl AsRef<Path>,
) -> io::Result<Vec<Result<crate::solana_archive::SolanaObservation, CompatibilityError>>> {
    let contents = fs::read_to_string(path)?;
    Ok(contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| migrate_odysseum_json_bytes(line.as_bytes()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn migration_marks_legacy_source_explicitly() {
        let value = json!({
            "observation": {
                "signature": "sig",
                "slot": 42,
                "confirmationStatus": "finalized"
            },
            "transaction": {}
        });
        let observation = from_odysseum_value(&value).expect("legacy event should parse");
        assert_eq!(observation.signature, "sig");
        assert_eq!(observation.slot, Some(42));
        assert_eq!(observation.source_label, "odysseum_compatibility_fixture");
    }

    #[test]
    fn migration_rejects_missing_signature() {
        let error = from_odysseum_value(&json!({ "observation": {} })).unwrap_err();
        assert_eq!(error, CompatibilityError::MissingSignature);
    }

    #[test]
    fn migration_preserves_raw_digest_and_legacy_provenance() {
        let value = br#"{"observation":{"signature":"sig","slot":42}}"#;
        let observation = migrate_odysseum_json_bytes(value).expect("event should migrate");
        assert_eq!(
            observation.provider,
            crate::solana_archive::ProviderKind::LegacyCompatibility
        );
        assert!(observation.raw_payload_digest.starts_with("blake3:"));
        assert_eq!(observation.slot, Some(42));
    }
}
