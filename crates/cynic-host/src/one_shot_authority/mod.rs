//! Local filesystem-backed authority for verifying and consuming one-shot capabilities.
//!
//! Module layout:
//!  - `mod signer`  : signing port + local ed25519 adapter
//!  - rest of this file : verification / consumption / crash recovery
//!
//! Splitting signing from verification keeps the two key operations
//! behind independent ports (`CapabilitySigningPort` vs
//! `OneShotCapabilityAuthorityPort`).

mod signer;

pub use signer::{CapabilitySigningPort, LocalCapabilitySigningKey};

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use cynic_protocol::{
    ONE_SHOT_CAPABILITY_CONSUMPTION_SCHEMA, ONE_SHOT_CAPABILITY_OUTCOME_SCHEMA, OneShotCapability,
    OneShotCapabilityConsumption, OneShotCapabilityOutcome, OneShotCapabilityOutcomeStatus,
};
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

pub trait OneShotCapabilityAuthorityPort {
    fn observe_unfinished_consumptions(&self) -> Result<Vec<OneShotCapabilityConsumption>, String>;
    fn recover_unfinished_consumptions(
        &self,
        now_unix_s: u64,
    ) -> Result<Vec<OneShotCapabilityOutcome>, String>;
    fn verify_and_consume(
        &self,
        capability: &OneShotCapability,
        subject: &str,
        scope: &str,
        action_digest: &str,
        now_unix_s: u64,
    ) -> Result<OneShotCapabilityConsumption, String>;
    fn record_outcome(&self, outcome: &OneShotCapabilityOutcome) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct LocalOneShotCapabilityAuthority {
    public_key_path: PathBuf,
    consumption_root: PathBuf,
}

impl LocalOneShotCapabilityAuthority {
    pub fn new(public_key_path: impl AsRef<Path>, consumption_root: impl AsRef<Path>) -> Self {
        Self {
            public_key_path: public_key_path.as_ref().to_path_buf(),
            consumption_root: consumption_root.as_ref().to_path_buf(),
        }
    }

    fn load_verifying_key(&self) -> Result<VerifyingKey, String> {
        let content = fs::read_to_string(&self.public_key_path)
            .map_err(|e| format!("failed to read authority public key: {}", e))?;
        let trimmed = content.trim();
        let bytes = hex::decode(trimmed)
            .map_err(|e| format!("invalid hex in authority public key: {}", e))?;
        if bytes.len() != 32 {
            return Err("authority public key must be 32 bytes".to_string());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        VerifyingKey::from_bytes(&arr).map_err(|e| format!("invalid ed25519 public key: {}", e))
    }
}

impl OneShotCapabilityAuthorityPort for LocalOneShotCapabilityAuthority {
    fn observe_unfinished_consumptions(&self) -> Result<Vec<OneShotCapabilityConsumption>, String> {
        if !self.consumption_root.exists() {
            return Ok(Vec::new());
        }
        let mut unfinished = Vec::new();
        let entries = fs::read_dir(&self.consumption_root)
            .map_err(|e| format!("failed to read consumption directory: {}", e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.ends_with(".consumption.json") {
                let capability_id = file_name.trim_end_matches(".consumption.json");
                let outcome_path = self
                    .consumption_root
                    .join(format!("{}.outcome.json", capability_id));
                if !outcome_path.exists() {
                    let content = fs::read_to_string(&path)
                        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
                    if let Ok(consumption) =
                        serde_json::from_str::<OneShotCapabilityConsumption>(&content)
                    {
                        unfinished.push(consumption);
                    }
                }
            }
        }
        Ok(unfinished)
    }

    fn recover_unfinished_consumptions(
        &self,
        now_unix_s: u64,
    ) -> Result<Vec<OneShotCapabilityOutcome>, String> {
        let unfinished = self.observe_unfinished_consumptions()?;
        let mut recovered = Vec::with_capacity(unfinished.len());
        for consumption in unfinished {
            let outcome = OneShotCapabilityOutcome {
                schema: ONE_SHOT_CAPABILITY_OUTCOME_SCHEMA.to_string(),
                capability_id: consumption.capability_id.clone(),
                consumption_digest: consumption.capability_digest,
                status: OneShotCapabilityOutcomeStatus::UnknownAfterCrash,
                recorded_at_unix_s: now_unix_s,
                evidence_refs: vec![format!(
                    "one_shot_consumption:{}",
                    consumption.capability_id
                )],
                blockers: vec!["unknown_after_crash".to_string()],
            };
            self.record_outcome(&outcome)?;
            recovered.push(outcome);
        }
        Ok(recovered)
    }

    fn verify_and_consume(
        &self,
        capability: &OneShotCapability,
        subject: &str,
        scope: &str,
        action_digest: &str,
        now_unix_s: u64,
    ) -> Result<OneShotCapabilityConsumption, String> {
        let blockers = capability.validate_for(subject, scope, action_digest, now_unix_s);
        if !blockers.is_empty() {
            return Err(blockers.join(","));
        }

        let verifying_key = self.load_verifying_key()?;
        let expected_key_id = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(verifying_key.as_bytes()))
        );
        if capability.authority_key_id != expected_key_id {
            return Err("authority_key_id_mismatch".to_string());
        }

        let signing_payload = capability
            .signing_payload()
            .map_err(|e| format!("failed to generate signing payload: {}", e))?;
        let sig_bytes = hex::decode(&capability.signature_hex)
            .map_err(|e| format!("invalid hex in signature: {}", e))?;
        if sig_bytes.len() != 64 {
            return Err("signature must be 64 bytes".to_string());
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_arr);
        verifying_key
            .verify_strict(&signing_payload, &signature)
            .map_err(|_| "signature_verification_failed".to_string())?;

        fs::create_dir_all(&self.consumption_root)
            .map_err(|e| format!("failed to create consumption root: {}", e))?;
        let consumption_path = self
            .consumption_root
            .join(format!("{}.consumption.json", capability.capability_id));

        let capability_digest = format!("sha256:{}", hex::encode(Sha256::digest(&signing_payload)));

        let consumption = OneShotCapabilityConsumption {
            schema: ONE_SHOT_CAPABILITY_CONSUMPTION_SCHEMA.to_string(),
            capability_id: capability.capability_id.clone(),
            subject: subject.to_string(),
            scope: scope.to_string(),
            action_digest: action_digest.to_string(),
            consumed_at_unix_s: now_unix_s,
            authority_key_id: capability.authority_key_id.clone(),
            capability_digest,
        };

        let json = serde_json::to_vec_pretty(&consumption)
            .map_err(|e| format!("failed to serialize consumption: {}", e))?;
        write_new_durable(&consumption_path, &json, "capability_already_consumed")?;

        Ok(consumption)
    }

    fn record_outcome(&self, outcome: &OneShotCapabilityOutcome) -> Result<(), String> {
        fs::create_dir_all(&self.consumption_root)
            .map_err(|e| format!("failed to create consumption root: {}", e))?;
        let outcome_path = self
            .consumption_root
            .join(format!("{}.outcome.json", outcome.capability_id));
        let json = serde_json::to_vec_pretty(outcome)
            .map_err(|e| format!("failed to serialize outcome: {}", e))?;
        write_new_durable(&outcome_path, &json, "capability_outcome_already_recorded")?;
        Ok(())
    }
}

fn write_new_durable(path: &Path, bytes: &[u8], exists_code: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                format!("{exists_code}:{}", path.display())
            } else {
                format!("failed to create {}: {error}", path.display())
            }
        })?;
    file.write_all(bytes)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to sync {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::one_shot_authority::OneShotCapabilityAuthorityPort;
    use ed25519_dalek::{Signer, SigningKey};

    fn signed_bootstrap_capability(
        signing_key: &SigningKey,
        action_digest: String,
    ) -> OneShotCapability {
        let verifying_key = signing_key.verifying_key();
        let mut capability = OneShotCapability {
            schema: cynic_protocol::ONE_SHOT_CAPABILITY_SCHEMA.to_string(),
            status: "admitted".to_string(),
            authority_class: "MachineEnrollment".to_string(),
            capability_id: "bootstrap-build-1".to_string(),
            subject: "node-key:sha256:subject".to_string(),
            scope: "build-runner-bootstrap".to_string(),
            input_digest: format!("sha256:{}", "a".repeat(64)),
            executor_digest: format!("sha256:{}", "b".repeat(64)),
            issued_at_unix_s: 10,
            expires_at_unix_s: 30,
            allowed_action_digests: vec![action_digest],
            authority_key_id: format!(
                "sha256:{}",
                hex::encode(Sha256::digest(verifying_key.as_bytes()))
            ),
            signature_hex: String::new(),
            evidence_refs: vec!["bootstrap_binding:fixture".to_string()],
            budget: None,
        };
        capability.signature_hex = hex::encode(
            signing_key
                .sign(
                    &capability
                        .signing_payload()
                        .expect("serialize capability signing payload"),
                )
                .to_bytes(),
        );
        capability
    }

    #[test]
    fn recovery_persists_unknown_after_crash_and_is_idempotent() {
        let root = tempfile::tempdir().expect("temporary consumption root");
        let consumption = OneShotCapabilityConsumption {
            schema: ONE_SHOT_CAPABILITY_CONSUMPTION_SCHEMA.to_string(),
            capability_id: "bootstrap-build-1".to_string(),
            subject: "node-key:sha256:subject".to_string(),
            scope: "build-runner-bootstrap".to_string(),
            action_digest: format!("sha256:{}", "a".repeat(64)),
            consumed_at_unix_s: 10,
            authority_key_id: format!("sha256:{}", "b".repeat(64)),
            capability_digest: format!("sha256:{}", "c".repeat(64)),
        };
        fs::write(
            root.path().join("bootstrap-build-1.consumption.json"),
            serde_json::to_vec(&consumption).expect("encode consumption"),
        )
        .expect("write consumption");
        let authority = LocalOneShotCapabilityAuthority::new("unused", root.path());

        let recovered = authority
            .recover_unfinished_consumptions(20)
            .expect("persist crash recovery");
        assert_eq!(recovered.len(), 1);
        assert_eq!(
            recovered[0].status,
            OneShotCapabilityOutcomeStatus::UnknownAfterCrash
        );
        assert_eq!(recovered[0].blockers, vec!["unknown_after_crash"]);
        assert!(
            authority
                .recover_unfinished_consumptions(21)
                .expect("recovery is idempotent")
                .is_empty()
        );
    }

    #[test]
    fn signed_consumption_recovers_crash_durably_and_refuses_reuse() {
        let root = tempfile::tempdir().expect("temporary authority root");
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let public_key_path = root.path().join("authority.pub");
        fs::write(
            &public_key_path,
            hex::encode(signing_key.verifying_key().as_bytes()),
        )
        .expect("write authority public key");
        let action_digest = cynic_protocol::build_runner_bootstrap_action_digest("make:build")
            .expect("build action digest");
        let capability = signed_bootstrap_capability(&signing_key, action_digest.clone());
        let authority = LocalOneShotCapabilityAuthority::new(&public_key_path, root.path());

        let consumption = authority
            .verify_and_consume(
                &capability,
                "node-key:sha256:subject",
                "build-runner-bootstrap",
                &action_digest,
                20,
            )
            .expect("consume a valid signed capability");
        assert!(
            root.path()
                .join("bootstrap-build-1.consumption.json")
                .is_file()
        );

        let recovered = authority
            .recover_unfinished_consumptions(21)
            .expect("recover interrupted build");
        assert_eq!(recovered.len(), 1);
        assert_eq!(
            recovered[0].status,
            OneShotCapabilityOutcomeStatus::UnknownAfterCrash
        );
        assert_eq!(
            recovered[0].consumption_digest,
            consumption.capability_digest
        );
        let persisted: OneShotCapabilityOutcome = serde_json::from_slice(
            &fs::read(root.path().join("bootstrap-build-1.outcome.json"))
                .expect("read recovered outcome"),
        )
        .expect("decode recovered outcome");
        assert_eq!(persisted, recovered[0]);

        assert!(
            authority
                .verify_and_consume(
                    &capability,
                    "node-key:sha256:subject",
                    "build-runner-bootstrap",
                    &action_digest,
                    22,
                )
                .expect_err("reused capability must be refused")
                .starts_with("capability_already_consumed:")
        );
    }
}
